//! # Long Print Chunking
//!
//! Inserts drain points into IR programs to prevent printer buffer overflow
//! during long prints with large graphics.
//!
//! ## Problem
//!
//! Thermal printers have limited internal buffers (~100-200KB). When sending
//! large amounts of data (especially graphics), the buffer can overflow causing
//! print failures, garbled output, or communication errors.
//!
//! **Key insight:** The issue is **data volume**, not print length. Text is tiny
//! (~50 bytes per line with styling) while images are massive:
//! - Raster: 72 bytes/row × 256 rows = ~18KB per chunk
//! - Band: 72 bytes/row × 24 rows = ~1.7KB per band
//!
//! You can print 500mm of text without issues, but 100mm of images will fail.
//!
//! ## Solution
//!
//! This module tracks cumulative **bytes sent** (not mm printed) and inserts
//! `Op::DrainBuffer` markers at natural boundaries when approaching the
//! threshold. The transport layer recognizes these markers and pauses to let
//! the printer catch up.
//!
//! ## Byte Estimation
//!
//! - Text line: ~50 bytes (text + commands)
//! - Raster chunk: ~18KB (256 rows × 72 bytes + 11-byte header)
//! - Band: ~1.7KB (24 rows × 72 bytes + 4-byte header + 3-byte feed)
//! - QR code: ~100-300 bytes
//! - Feed/styling: 2-4 bytes
//!
//! ## Natural Boundaries
//!
//! Drain points are only inserted at natural boundaries:
//! - After Raster graphics (large data)
//! - After Band graphics (medium data)
//! - After QR/PDF417 barcodes
//! - NOT after text/newlines (too frequent, data is tiny)

use super::ops::{Op, Program};

/// Default threshold before inserting a drain point (16KB).
/// Very conservative - ensures we pause frequently for large graphics.
/// 16KB ≈ 220 rows of full-width graphics ≈ 28mm.
pub const DEFAULT_DRAIN_THRESHOLD_BYTES: usize = 16 * 1024;

/// For backwards compatibility - converts mm to approximate bytes.
/// ~80mm of full-width graphics ≈ 640 rows × 72 bytes ≈ 46KB
pub const DEFAULT_DRAIN_THRESHOLD_MM: f32 = 80.0;

impl Program {
    /// Insert drain points to prevent buffer overflow during long prints.
    ///
    /// This should be called after optimization but before codegen:
    ///
    /// ```text
    /// Components → IR → Optimizer → insert_drain_points() → Codegen → Bytes
    /// ```
    ///
    /// Uses the default threshold of 64KB.
    pub fn insert_drain_points(self) -> Self {
        self.insert_drain_points_with_threshold_bytes(DEFAULT_DRAIN_THRESHOLD_BYTES)
    }

    /// Insert drain points with a custom threshold (in bytes).
    pub fn insert_drain_points_with_threshold_bytes(self, threshold_bytes: usize) -> Self {
        let ops = insert_drain_points_impl(self.ops, threshold_bytes);
        Program { ops }
    }

    /// Insert drain points with a custom threshold (in mm).
    /// Converts mm to approximate bytes assuming full-width graphics.
    pub fn insert_drain_points_with_threshold(self, threshold_mm: f32) -> Self {
        // 8 dots/mm × 72 bytes/row = 576 bytes per mm of full-width graphics
        let bytes_per_mm = 576.0;
        let threshold_bytes = (threshold_mm * bytes_per_mm) as usize;
        self.insert_drain_points_with_threshold_bytes(threshold_bytes)
    }
}

/// Internal implementation of drain point insertion.
fn insert_drain_points_impl(ops: Vec<Op>, threshold_bytes: usize) -> Vec<Op> {
    let mut result = Vec::with_capacity(ops.len() + ops.len() / 20); // Estimate ~5% growth
    let mut bytes_sent: usize = 0;

    for op in ops {
        // Calculate byte cost of this op
        let op_bytes = estimate_op_bytes(&op);

        // Only consider drain points for "heavy" operations (graphics, barcodes)
        let is_heavy = is_heavy_operation(&op);

        // For heavy ops, check before adding if we'd exceed threshold
        if is_heavy && bytes_sent > 0 && bytes_sent + op_bytes > threshold_bytes {
            result.push(Op::DrainBuffer);
            bytes_sent = 0;
        }

        // Add the op
        result.push(op.clone());
        bytes_sent += op_bytes;

        // For heavy ops, also check after adding (in case single op exceeds threshold)
        // This ensures we drain after large graphics even if they're the first op
        if is_heavy && bytes_sent >= threshold_bytes {
            result.push(Op::DrainBuffer);
            bytes_sent = 0;
        }
    }

    result
}

/// Estimate the byte cost of an op when encoded.
fn estimate_op_bytes(op: &Op) -> usize {
    match op {
        // ===== Heavy operations (graphics) =====

        // Raster: 11-byte header + data, chunked at 256 rows
        // But codegen chunks it, so estimate per-chunk
        Op::Raster { width, height, data } => {
            let width_bytes = width.div_ceil(8) as usize;
            let chunk_rows = 256;
            let total_rows = *height as usize;
            let num_chunks = (total_rows + chunk_rows - 1) / chunk_rows;
            // Each chunk: 11-byte header + row_data
            let data_per_chunk = width_bytes * chunk_rows.min(total_rows);
            num_chunks * (11 + data_per_chunk).min(11 + data.len())
        }

        // Band: 4-byte header + data + 3-byte feed per band
        Op::Band { width_bytes, data } => {
            let band_size = *width_bytes as usize * 24;
            let num_bands = (data.len() + band_size - 1) / band_size;
            // Each band: 4-byte header + band_data + 3-byte feed
            num_bands * (4 + band_size + 3)
        }

        // QR code: variable, typically 100-300 bytes
        Op::QrCode { data, .. } => 50 + data.len() * 2,

        // PDF417: variable, typically 100-500 bytes
        Op::Pdf417 { data, .. } => 50 + data.len() * 3,

        // 1D barcode: header + data + height encoding
        Op::Barcode1D { data, .. } => 20 + data.len(),

        // ===== Light operations (text, control) =====

        // Text: just the bytes
        Op::Text(s) => s.len(),

        // Newline: 1 byte
        Op::Newline => 1,

        // Feed: 3 bytes (ESC J n)
        Op::Feed { .. } => 3,

        // Cut: 3 bytes (ESC d n)
        Op::Cut { .. } => 3,

        // Init: 2 bytes (ESC @)
        Op::Init => 2,

        // Style commands: 2-4 bytes
        Op::SetAlign(_) => 4,
        Op::SetFont(_) => 4,
        Op::SetBold(_) => 2,
        Op::SetUnderline(_) => 3,
        Op::SetInvert(_) => 3,
        Op::SetSize { .. } => 4,
        Op::SetExpandedWidth(_) => 3,
        Op::SetExpandedHeight(_) => 3,
        Op::SetSmoothing(_) => 3,
        Op::SetUpperline(_) => 3,
        Op::SetUpsideDown(_) => 3,
        Op::SetReduced(_) => 4,
        Op::SetCodepage(_) => 3,
        Op::ResetStyle => 10, // Multiple commands
        Op::SetAbsolutePosition(_) => 4,

        // Raw: actual bytes
        Op::Raw(bytes) => bytes.len(),

        // NV operations: variable but small commands
        Op::NvStore { data, .. } => 20 + data.len(),
        Op::NvPrint { .. } => 10,
        Op::NvDelete { .. } => 10,

        // Drain buffer: marker bytes (not sent to printer)
        Op::DrainBuffer => 0,
    }
}

/// Check if this op is a "heavy" operation that warrants drain consideration.
/// Heavy = graphics or large barcodes that contribute significant data.
fn is_heavy_operation(op: &Op) -> bool {
    matches!(
        op,
        Op::Raster { .. } | Op::Band { .. } | Op::QrCode { .. } | Op::Pdf417 { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_drain_for_text_only() {
        // Text-only prints should never get drain points
        // (text is "light", not enough bytes to matter)
        let ops = vec![
            Op::Init,
            Op::Text("Hello".into()),
            Op::Newline,
            Op::Text("World".into()),
            Op::Newline,
            Op::Cut { partial: false },
        ];
        let program = Program { ops };
        let result = program.insert_drain_points();

        // Should not insert any drain points for text
        assert!(!result.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_no_drain_for_lots_of_text() {
        // Even lots of text shouldn't trigger drain (bytes are tiny)
        let mut ops = vec![Op::Init];
        for i in 0..1000 {
            ops.push(Op::Text(format!("Line {} with some text content", i)));
            ops.push(Op::Newline);
        }
        ops.push(Op::Cut { partial: false });

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Text is "light" - no drain points
        assert!(!result.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_drain_after_large_raster() {
        // Large raster should trigger drain (it's "heavy")
        // 576 wide x 1000 tall = 72 bytes/row * 1000 = 72KB > 64KB threshold
        let ops = vec![
            Op::Init,
            Op::Raster {
                width: 576,
                height: 1000,
                data: vec![0; 576 / 8 * 1000],
            },
            Op::Cut { partial: false },
        ];

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Should have drain point after the large raster
        assert!(result.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_drain_between_multiple_rasters() {
        // Multiple rasters should get drains between them
        let raster = Op::Raster {
            width: 576,
            height: 500, // ~36KB each
            data: vec![0; 576 / 8 * 500],
        };

        let ops = vec![
            Op::Init,
            raster.clone(),
            raster.clone(), // Two rasters = ~72KB, exceeds 64KB threshold
            Op::Cut { partial: false },
        ];

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Should have at least one drain point
        let drain_count = result
            .ops
            .iter()
            .filter(|op| matches!(op, Op::DrainBuffer))
            .count();
        assert!(drain_count >= 1, "Expected drain between large rasters");
    }

    #[test]
    fn test_no_drain_for_small_raster() {
        // Small raster under threshold shouldn't trigger drain
        let ops = vec![
            Op::Init,
            Op::Raster {
                width: 576,
                height: 100, // ~7KB, well under 64KB
                data: vec![0; 576 / 8 * 100],
            },
            Op::Cut { partial: false },
        ];

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Small raster doesn't need drain
        assert!(!result.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_drain_for_large_band() {
        // Large band graphics should trigger drain
        // 30 bands * (72*24 + 7) bytes ≈ 52KB
        let band_size = 72 * 24;
        let data = vec![0u8; band_size * 40]; // 40 bands ≈ 69KB

        let ops = vec![
            Op::Init,
            Op::Band {
                width_bytes: 72,
                data,
            },
            Op::Cut { partial: false },
        ];

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Large band should trigger drain
        assert!(result.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_drain_not_at_start() {
        // Drain should never be the first op
        let ops = vec![
            Op::Init,
            Op::Raster {
                width: 576,
                height: 1000,
                data: vec![0; 576 / 8 * 1000],
            },
        ];

        let program = Program { ops };
        let result = program.insert_drain_points();

        // Init should still be first
        assert!(matches!(result.ops[0], Op::Init));
    }

    #[test]
    fn test_custom_byte_threshold() {
        // Test custom byte threshold
        let ops = vec![
            Op::Init,
            Op::Raster {
                width: 576,
                height: 200, // ~14KB
                data: vec![0; 576 / 8 * 200],
            },
            Op::Cut { partial: false },
        ];

        let program = Program { ops };

        // With default 64KB threshold, no drain needed
        let result_64k = program.clone().insert_drain_points();
        assert!(!result_64k.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));

        // With 10KB threshold, should have drain
        let result_10k = program.insert_drain_points_with_threshold_bytes(10 * 1024);
        assert!(result_10k.ops.iter().any(|op| matches!(op, Op::DrainBuffer)));
    }

    #[test]
    fn test_estimate_raster_bytes() {
        let op = Op::Raster {
            width: 576,
            height: 256,
            data: vec![0; 576 / 8 * 256],
        };
        let bytes = estimate_op_bytes(&op);
        // 1 chunk of 256 rows: 11 header + 72*256 data = 18443 bytes
        assert!(bytes > 18000 && bytes < 19000);
    }

    #[test]
    fn test_estimate_band_bytes() {
        let band_size = 72 * 24;
        let op = Op::Band {
            width_bytes: 72,
            data: vec![0; band_size * 5], // 5 bands
        };
        let bytes = estimate_op_bytes(&op);
        // 5 bands * (4 header + 1728 data + 3 feed) = 8675 bytes
        assert!(bytes > 8000 && bytes < 9000);
    }

    #[test]
    fn test_estimate_qr_bytes() {
        let op = Op::QrCode {
            data: "test".into(),
            cell_size: 4,
            error_level: crate::protocol::barcode::qr::QrErrorLevel::M,
        };
        let bytes = estimate_op_bytes(&op);
        // 50 + 4*2 = 58 bytes estimate
        assert_eq!(bytes, 58);
    }

    #[test]
    fn test_text_is_not_heavy() {
        // Verify text operations are not considered "heavy"
        assert!(!is_heavy_operation(&Op::Text("test".into())));
        assert!(!is_heavy_operation(&Op::Newline));
        assert!(!is_heavy_operation(&Op::Feed { units: 100 }));
    }

    #[test]
    fn test_graphics_are_heavy() {
        // Verify graphics operations are considered "heavy"
        assert!(is_heavy_operation(&Op::Raster {
            width: 576,
            height: 100,
            data: vec![]
        }));
        assert!(is_heavy_operation(&Op::Band {
            width_bytes: 72,
            data: vec![]
        }));
        assert!(is_heavy_operation(&Op::QrCode {
            data: "test".into(),
            cell_size: 4,
            error_level: crate::protocol::barcode::qr::QrErrorLevel::M,
        }));
    }
}

//! # Long Print Splitting
//!
//! Splits large IR programs into multiple independent print jobs to prevent
//! printer buffer overflow during long prints with large graphics.
//!
//! ## Problem
//!
//! Thermal printers have limited internal buffers (~100-200KB). Large images
//! can overflow the buffer causing print failures.
//!
//! ## Solution
//!
//! Split large images into multiple **completely independent print jobs**
//! (~1000 rows each). Each job has its own Init command. The printer processes
//! each job fully before receiving the next.
//!
//! ## Example
//!
//! ```
//! use estrella::ir::{Op, Program};
//!
//! let mut program = Program::new();
//! program.push(Op::Init);
//! program.push(Op::Raster {
//!     width: 576,
//!     height: 2000,
//!     data: vec![0; 576 / 8 * 2000],
//! });
//! program.push(Op::Feed { units: 24 });
//! program.push(Op::Cut { partial: false });
//!
//! let programs = program.split_for_long_print();
//! // programs[0]: Init + Raster(rows 0-999)
//! // programs[1]: Init + Raster(rows 1000-1999) + Feed + Cut
//! ```

use super::ops::{Op, Program};

/// Default chunk size in bytes for splitting large raster images.
/// 45KB is conservative - printer buffer is ~50-60KB based on empirical testing.
/// Formula: chunk_rows = chunk_bytes / width_bytes
/// For 576-dot width: 45,000 / 72 = 625 rows ≈ 78mm
pub const DEFAULT_CHUNK_BYTES: usize = 45_000;

/// Band alignment - band mode must split at 24-row boundaries.
const BAND_HEIGHT: usize = 24;

impl Program {
    /// Split this program into multiple independent print jobs for long prints.
    ///
    /// Each chunk starts with an `Op::Init` and contains a portion of the
    /// graphics. Trailing operations (Feed, Cut) are only on the last chunk.
    /// This allows the printer to process each job completely before receiving
    /// the next, preventing buffer overflow.
    ///
    /// The chunk size is calculated from `DEFAULT_CHUNK_BYTES` (45KB) divided
    /// by the image's width in bytes. This ensures consistent buffer usage
    /// regardless of image width.
    ///
    /// Small images that fit in a single chunk are not split.
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::ir::Program;
    ///
    /// let program = Program::with_init();
    /// // ... add large raster ...
    /// let programs = program.split_for_long_print();
    /// ```
    pub fn split_for_long_print(self) -> Vec<Program> {
        // Job splitting disabled - tcdrain pacing in transport handles flow control.
        // TODO: re-enable if needed: self.split_for_long_print_with_max_bytes(DEFAULT_CHUNK_BYTES)
        vec![self]
    }

    /// Split with a custom maximum bytes per chunk.
    ///
    /// The chunk size in rows is calculated as: `max_bytes / width_bytes`
    pub fn split_for_long_print_with_max_bytes(self, max_bytes: usize) -> Vec<Program> {
        // Find the graphics operation to determine width
        let width_bytes = self.find_graphics_width_bytes().unwrap_or(72); // default to 576/8
        let chunk_rows = max_bytes / width_bytes;
        self.split_for_long_print_with_chunk_size(chunk_rows)
    }

    /// Split with a custom chunk size (in rows).
    ///
    /// For band mode, the chunk size will be rounded down to a multiple of 24.
    pub fn split_for_long_print_with_chunk_size(self, chunk_rows: usize) -> Vec<Program> {
        // Find graphics operations and trailing operations
        let (graphics_idx, graphics_op) = match self.find_splittable_graphics(chunk_rows) {
            Some(result) => result,
            None => {
                // No splittable graphics, return single program unchanged
                println!(
                    "[split] No splittable graphics found (threshold: {} rows), returning single program",
                    chunk_rows
                );
                return vec![self];
            }
        };

        // Get the graphics dimensions
        let (width, total_rows, data, is_band) = match graphics_op {
            Op::Raster { width, height, data } => (*width, *height as usize, data.clone(), false),
            Op::Band { width_bytes, data } => {
                let width = (*width_bytes as u16) * 8;
                let height = data.len() / (*width_bytes as usize);
                (width, height, data.clone(), true)
            }
            _ => unreachable!(),
        };

        // Determine effective chunk size (must be aligned to 24 for band mode)
        let effective_chunk_rows = if is_band {
            (chunk_rows / BAND_HEIGHT) * BAND_HEIGHT
        } else {
            chunk_rows
        };

        // If the image is small enough, no split needed
        if total_rows <= effective_chunk_rows {
            println!(
                "[split] Image {} rows <= {} chunk rows, no split needed",
                total_rows, effective_chunk_rows
            );
            return vec![self];
        }

        // Calculate bytes per row for splitting
        let width_bytes = width.div_ceil(8) as usize;

        let num_chunks = (total_rows + effective_chunk_rows - 1) / effective_chunk_rows;
        let total_bytes = total_rows * width_bytes;
        let chunk_bytes = effective_chunk_rows * width_bytes;
        println!(
            "[split] Splitting {} row {} image ({} bytes) into {} chunks of {} rows (~{}KB each)",
            total_rows,
            if is_band { "BAND" } else { "RASTER" },
            total_bytes,
            num_chunks,
            effective_chunk_rows,
            chunk_bytes / 1024
        );

        // Collect pre-graphics ops (excluding Init, we'll add our own)
        let pre_ops: Vec<Op> = self.ops[..graphics_idx]
            .iter()
            .filter(|op| !matches!(op, Op::Init))
            .cloned()
            .collect();

        // Collect post-graphics (trailing) ops
        let trailing_ops: Vec<Op> = self.ops[graphics_idx + 1..].to_vec();

        // Split the graphics data
        let mut programs = Vec::new();
        let mut row_offset = 0;

        while row_offset < total_rows {
            let chunk_height = (total_rows - row_offset).min(effective_chunk_rows);
            let is_last_chunk = row_offset + chunk_height >= total_rows;

            let byte_start = row_offset * width_bytes;
            let byte_end = (row_offset + chunk_height) * width_bytes;
            let chunk_data = data[byte_start..byte_end].to_vec();

            let mut chunk_program = Program::new();

            // Each chunk starts with Init
            chunk_program.push(Op::Init);

            // Add pre-graphics ops (styles, etc.)
            chunk_program.extend(pre_ops.clone());

            // Add the graphics chunk
            if is_band {
                chunk_program.push(Op::Band {
                    width_bytes: (width / 8) as u8,
                    data: chunk_data,
                });
            } else {
                chunk_program.push(Op::Raster {
                    width,
                    height: chunk_height as u16,
                    data: chunk_data,
                });
            }

            // Only add trailing ops (Feed, Cut) to the last chunk
            if is_last_chunk {
                chunk_program.extend(trailing_ops.clone());
            }

            programs.push(chunk_program);
            row_offset += chunk_height;
        }

        programs
    }

    /// Find a splittable graphics operation (Raster or Band) in this program.
    ///
    /// Returns the index and a reference to the operation.
    fn find_splittable_graphics(&self, chunk_rows: usize) -> Option<(usize, &Op)> {
        for (i, op) in self.ops.iter().enumerate() {
            match op {
                Op::Raster { height, .. } if *height as usize > chunk_rows => {
                    return Some((i, op));
                }
                Op::Band { width_bytes, data } => {
                    let height = data.len() / (*width_bytes as usize);
                    if height > chunk_rows {
                        return Some((i, op));
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Find the width in bytes of the first graphics operation.
    fn find_graphics_width_bytes(&self) -> Option<usize> {
        for op in &self.ops {
            match op {
                Op::Raster { width, .. } => {
                    return Some(width.div_ceil(8) as usize);
                }
                Op::Band { width_bytes, .. } => {
                    return Some(*width_bytes as usize);
                }
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_raster_no_split() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 500, // Less than DEFAULT_CHUNK_ROWS
            data: vec![0; 576 / 8 * 500],
        });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();
        assert_eq!(programs.len(), 1);
    }

    #[test]
    #[ignore = "split disabled - tcdrain pacing handles flow control"]
    fn test_large_raster_splits() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 1800, // Should split into 3 programs with 600-row chunks
            data: vec![0; 576 / 8 * 1800],
        });
        program.push(Op::Feed { units: 24 });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();
        assert_eq!(programs.len(), 3);
    }

    #[test]
    #[ignore = "split disabled - tcdrain pacing handles flow control"]
    fn test_trailing_ops_on_last_chunk_only() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 1200, // Should split into 2 programs with 600-row chunks
            data: vec![0; 576 / 8 * 1200],
        });
        program.push(Op::Feed { units: 24 });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();
        assert_eq!(programs.len(), 2);

        // First program should NOT have Feed or Cut
        let first_has_cut = programs[0].ops.iter().any(|op| matches!(op, Op::Cut { .. }));
        let first_has_feed = programs[0].ops.iter().any(|op| matches!(op, Op::Feed { .. }));
        assert!(!first_has_cut, "First program should not have Cut");
        assert!(!first_has_feed, "First program should not have Feed");

        // Last program should have Feed and Cut
        let last_has_cut = programs[1].ops.iter().any(|op| matches!(op, Op::Cut { .. }));
        let last_has_feed = programs[1].ops.iter().any(|op| matches!(op, Op::Feed { .. }));
        assert!(last_has_cut, "Last program should have Cut");
        assert!(last_has_feed, "Last program should have Feed");
    }

    #[test]
    fn test_each_chunk_has_init() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 1200, // Should split into 2 programs with 600-row chunks
            data: vec![0; 576 / 8 * 1200],
        });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();

        for (i, prog) in programs.iter().enumerate() {
            assert!(
                !prog.ops.is_empty() && matches!(prog.ops[0], Op::Init),
                "Program {} should start with Init",
                i
            );
        }
    }

    #[test]
    #[ignore = "split disabled - tcdrain pacing handles flow control"]
    fn test_no_feed_between_chunks() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 1800, // Should split into 3 programs with 600-row chunks
            data: vec![0; 576 / 8 * 1800],
        });
        program.push(Op::Feed { units: 24 });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();
        assert_eq!(programs.len(), 3);

        // Middle chunks should have no Feed
        for prog in &programs[..programs.len() - 1] {
            let has_feed = prog.ops.iter().any(|op| matches!(op, Op::Feed { .. }));
            assert!(!has_feed, "Non-final chunks should not have Feed");
        }
    }

    #[test]
    #[ignore = "split disabled - tcdrain pacing handles flow control"]
    fn test_band_mode_splits_aligned() {
        // Band mode should split at 24-row boundaries
        let width_bytes: u8 = 72;
        let band_size = width_bytes as usize * 24;
        let total_bands = 100; // 100 bands = 2400 rows
        let data = vec![0u8; band_size * total_bands];

        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Band { width_bytes, data });
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();

        // Should split into multiple programs
        assert!(programs.len() > 1);

        // Each chunk's band data should be a multiple of 24 rows
        for prog in &programs {
            for op in &prog.ops {
                if let Op::Band { width_bytes, data } = op {
                    let height = data.len() / (*width_bytes as usize);
                    assert_eq!(
                        height % 24,
                        0,
                        "Band chunk height {} should be multiple of 24",
                        height
                    );
                }
            }
        }
    }

    #[test]
    fn test_program_without_graphics() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Text("Hello".into()));
        program.push(Op::Newline);
        program.push(Op::Cut { partial: false });

        let programs = program.split_for_long_print();

        // Text-only program should return single program unchanged
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].ops.len(), 4);
    }

    #[test]
    fn test_custom_chunk_size() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: 576,
            height: 1000,
            data: vec![0; 576 / 8 * 1000],
        });
        program.push(Op::Cut { partial: false });

        // With chunk size of 300, should split into 4 programs
        let programs = program.split_for_long_print_with_chunk_size(300);
        assert_eq!(programs.len(), 4);
    }
}

//! # IR to PNG Preview Renderer
//!
//! Renders IR programs to PNG images showing what receipts would look like when printed.
//!
//! ## Architecture
//!
//! ```text
//! Program (IR) → PreviewRenderer → PNG bytes
//!                     ↓
//!               Process each Op:
//!               - Track style state (font, bold, align, size...)
//!               - Render text with bitmap font
//!               - Blit graphics (Raster/Band already have pixel data)
//!               - Generate barcodes
//!               - Output final composited image
//! ```
//!
//! ## Example
//!
//! ```
//! use estrella::ir::Program;
//! use estrella::preview::render_preview;
//!
//! let program = Program::new();
//! // ... add ops ...
//! let png_bytes = render_preview(&program).unwrap();
//! ```

mod barcode;
pub mod emoji;
mod font;
mod text;
pub mod ttf_font;

pub use font::{FontMetrics, generate_glyph};

use crate::ir::{BarcodeKind, Op, Program, StyleState};
use crate::protocol::barcode::qr::QrErrorLevel;
use crate::protocol::text::{Alignment, Font};
use image::{GrayImage, Luma};
use std::collections::HashMap;
use thiserror::Error;

use barcode::{encode_code39, encode_code128};
use font::RenderState;

/// Errors that can occur during preview rendering.
#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("Image encoding error: {0}")]
    ImageEncode(String),

    #[error("Invalid operation: {0}")]
    InvalidOp(String),
}

/// Preview renderer for IR programs.
pub struct PreviewRenderer {
    /// Total paper width in dots (including margins)
    paper_width: usize,
    /// Printable area width in dots
    print_width: usize,
    /// Left margin in dots
    left_margin: usize,
    /// Top margin in dots (used to set initial y position)
    #[allow(dead_code)]
    top_margin: usize,
    buffer: Vec<u8>,
    height: usize,
    state: RenderState,
    font_cache: HashMap<(Font, char), Vec<u8>>,
}

impl PreviewRenderer {
    /// Create a new preview renderer with the given dimensions.
    ///
    /// - `paper_width`: Total paper width in dots (e.g., 640 for 80mm paper)
    /// - `print_width`: Printable area width in dots (e.g., 576 for 72mm)
    /// - `left_margin`: Left margin in dots (e.g., 32 for 4mm)
    /// - `top_margin`: Top margin in dots (e.g., 16 for ~2mm)
    pub fn new(
        paper_width: usize,
        print_width: usize,
        left_margin: usize,
        top_margin: usize,
    ) -> Self {
        // Start with a reasonable initial height
        let initial_height = 100;
        let buffer = vec![0u8; paper_width * initial_height];

        let state = RenderState {
            y: top_margin,
            ..Default::default()
        };

        Self {
            paper_width,
            print_width,
            left_margin,
            top_margin,
            buffer,
            height: initial_height,
            state,
            font_cache: HashMap::new(),
        }
    }

    /// Create a renderer for TSP650II (80mm paper, 72mm printable, 4mm margins).
    ///
    /// ## Dimensions
    ///
    /// ```text
    /// ├── 4mm ──┼────── 72mm printable ──────┼── 4mm ──┤
    /// │ 32 dots │         576 dots           │ 32 dots │
    /// └─────────┴────────────────────────────┴─────────┘
    ///                   640 dots total
    /// ```
    pub fn tsp650ii() -> Self {
        // 80mm paper at 203 DPI = 640 dots total
        // 72mm printable = 576 dots
        // 4mm margin each side = 32 dots
        // ~2mm top margin = 16 dots
        Self::new(640, 576, 32, 16)
    }

    /// Create a renderer for an arbitrary print width.
    ///
    /// Scales margins proportionally to the TSP650II ratio (~5.5% of print width).
    pub fn for_width(print_width: usize) -> Self {
        let margin = (print_width as f32 * 0.055).round() as usize;
        Self::new(print_width + margin * 2, print_width, margin, margin / 2)
    }

    /// Ensure buffer has room for the given y position.
    fn ensure_height(&mut self, y: usize) {
        let needed_height = y + 1;
        if needed_height > self.height {
            // Grow by at least 100 rows or to needed height
            let new_height = needed_height.max(self.height + 100);
            self.buffer.resize(self.paper_width * new_height, 0);
            self.height = new_height;
        }
    }

    /// Set a pixel (1 = black, 0 = white).
    /// x is in paper coordinates (0 = left edge of paper).
    fn set_pixel(&mut self, x: usize, y: usize, black: bool) {
        if x >= self.paper_width {
            return;
        }
        self.ensure_height(y);
        let idx = y * self.paper_width + x;
        self.buffer[idx] = if black { 1 } else { 0 };
    }

    /// Set a pixel in print coordinates (0 = left edge of printable area).
    /// Automatically adds the left margin offset.
    fn set_print_pixel(&mut self, x: usize, y: usize, black: bool) {
        self.set_pixel(x + self.left_margin, y, black);
    }

    /// Render the program to PNG bytes.
    pub fn render(&mut self, program: &Program) -> Result<Vec<u8>, PreviewError> {
        for op in &program.ops {
            self.process_op(op)?;
        }

        self.to_png()
    }

    /// Process a single IR operation.
    fn process_op(&mut self, op: &Op) -> Result<(), PreviewError> {
        match op {
            Op::Init => {
                self.state.reset();
            }

            Op::Cut { partial: _ } => {
                // Draw a dashed cut line across the full paper width
                self.newline();
                let y = self.state.y;
                self.ensure_height(y + 4);
                for x in 0..self.paper_width {
                    // Dashed pattern: 8 on, 4 off
                    if (x / 8) % 2 == 0 {
                        self.set_pixel(x, y, true);
                        self.set_pixel(x, y + 1, true);
                    }
                }
                self.state.y += 4;
            }

            Op::Feed { units } => {
                // Units are 1/4mm, printer is 203 DPI (8 dots/mm)
                // So 1 unit = 2 dots
                let dots = (*units as usize) * 2;
                self.state.y += dots;
                self.ensure_height(self.state.y);
            }

            Op::SetAlign(align) => {
                self.state.style.alignment = *align;
            }

            Op::SetFont(font) => {
                self.state.style.font = *font;
            }

            Op::SetBold(enabled) => {
                self.state.style.bold = *enabled;
            }

            Op::SetUnderline(enabled) => {
                self.state.style.underline = *enabled;
            }

            Op::SetInvert(enabled) => {
                self.state.style.invert = *enabled;
            }

            Op::SetSize { height, width } => {
                self.state.style.height_mult = *height;
                self.state.style.width_mult = *width;
            }

            Op::SetExpandedWidth(mult) => {
                self.state.style.expanded_width = *mult;
            }

            Op::SetExpandedHeight(mult) => {
                self.state.style.expanded_height = *mult;
            }

            Op::SetSmoothing(enabled) => {
                self.state.style.smoothing = *enabled;
            }

            Op::SetUpperline(enabled) => {
                self.state.style.upperline = *enabled;
            }

            Op::SetUpsideDown(enabled) => {
                self.state.style.upside_down = *enabled;
            }

            Op::SetReduced(enabled) => {
                self.state.style.reduced = *enabled;
            }

            Op::SetCodepage(_) => {
                // Codepage doesn't affect visual rendering in preview
            }

            Op::ResetStyle => {
                self.state.style = StyleState::default();
            }

            Op::Text(text) => {
                self.render_text(text);
            }

            Op::Newline => {
                self.newline();
            }

            Op::Raw(_) => {
                // Raw bytes are printer-specific, can't preview
            }

            Op::Raster {
                width,
                height,
                data,
            } => {
                self.render_raster(*width as usize, *height as usize, data);
            }

            Op::Band { width_bytes, data } => {
                let width_dots = (*width_bytes as usize) * 8;
                let height = data.len() / (*width_bytes as usize);
                self.render_band(width_dots, height, *width_bytes as usize, data);
            }

            Op::QrCode {
                data,
                cell_size,
                error_level,
            } => {
                self.render_qrcode(data, *cell_size, *error_level)?;
            }

            Op::Pdf417 {
                data,
                module_width,
                ecc_level: _,
            } => {
                self.render_pdf417(data, *module_width)?;
            }

            Op::Barcode1D { kind, data, height } => {
                self.render_barcode1d(*kind, data, *height);
            }

            Op::SetAbsolutePosition(dots) => {
                // Set horizontal position (in print coordinates)
                self.state.x = (*dots as usize).min(self.print_width);
            }

            Op::NvPrint {
                key,
                scale_x,
                scale_y,
            } => {
                // Look up logo from registry for preview
                if let Some(raster) = crate::logos::get_raster(key) {
                    self.render_nv_logo(&raster, *scale_x, *scale_y);
                } else {
                    // Unknown logo - render placeholder box
                    self.render_placeholder(&format!("NV:{}", key), 96, 96);
                }
            }

            Op::NvStore { .. } | Op::NvDelete { .. } => {
                // These don't produce visible output in preview
            }
        }

        Ok(())
    }

    /// Render raster graphics data.
    fn render_raster(&mut self, width: usize, height: usize, data: &[u8]) {
        // Center graphics within print area
        let start_x = if width < self.print_width {
            (self.print_width - width) / 2
        } else {
            0
        };

        let width_bytes = width.div_ceil(8);
        self.ensure_height(self.state.y + height);

        for row in 0..height {
            for col in 0..width {
                let byte_idx = row * width_bytes + col / 8;
                let bit_idx = 7 - (col % 8);

                if byte_idx < data.len() {
                    let pixel_on = (data[byte_idx] >> bit_idx) & 1 == 1;
                    self.set_print_pixel(start_x + col, self.state.y + row, pixel_on);
                }
            }
        }

        self.state.y += height;
        self.state.x = 0;
    }

    /// Render band graphics data.
    fn render_band(&mut self, width_dots: usize, height: usize, width_bytes: usize, data: &[u8]) {
        // Center graphics within print area
        let start_x = if width_dots < self.print_width {
            (self.print_width - width_dots) / 2
        } else {
            0
        };

        self.ensure_height(self.state.y + height);

        for row in 0..height {
            for col in 0..width_dots {
                let byte_idx = row * width_bytes + col / 8;
                let bit_idx = 7 - (col % 8);

                if byte_idx < data.len() {
                    let pixel_on = (data[byte_idx] >> bit_idx) & 1 == 1;
                    self.set_print_pixel(start_x + col, self.state.y + row, pixel_on);
                }
            }
        }

        self.state.y += height;
        self.state.x = 0;
    }

    /// Render an NV logo from the registry.
    ///
    /// Uses current x position (set via SetAbsolutePosition) as the starting point.
    fn render_nv_logo(&mut self, raster: &crate::logos::LogoRaster, scale_x: u8, scale_y: u8) {
        let src_width = raster.width as usize;
        let src_height = raster.height as usize;
        let src_width_bytes = src_width.div_ceil(8);

        let _dest_width = src_width * scale_x as usize;
        let dest_height = src_height * scale_y as usize;

        // Use current x position (which may have been set by SetAbsolutePosition)
        let start_x = self.state.x;

        self.ensure_height(self.state.y + dest_height);

        for sy in 0..src_height {
            for sx in 0..src_width {
                let byte_idx = sy * src_width_bytes + sx / 8;
                let bit_idx = 7 - (sx % 8);

                if byte_idx < raster.data.len() {
                    let pixel_on = (raster.data[byte_idx] >> bit_idx) & 1 == 1;

                    if pixel_on {
                        // Scale up the pixel
                        for dy in 0..(scale_y as usize) {
                            for dx in 0..(scale_x as usize) {
                                let px = start_x + sx * (scale_x as usize) + dx;
                                let py = self.state.y + sy * (scale_y as usize) + dy;
                                self.set_print_pixel(px, py, true);
                            }
                        }
                    }
                }
            }
        }

        self.state.y += dest_height;
        self.state.x = 0;
    }

    /// Render a QR code.
    fn render_qrcode(
        &mut self,
        data: &str,
        cell_size: u8,
        error_level: QrErrorLevel,
    ) -> Result<(), PreviewError> {
        use qrcode::{EcLevel, QrCode};

        let ec_level = match error_level {
            QrErrorLevel::L => EcLevel::L,
            QrErrorLevel::M => EcLevel::M,
            QrErrorLevel::Q => EcLevel::Q,
            QrErrorLevel::H => EcLevel::H,
        };

        let code = QrCode::with_error_correction_level(data, ec_level)
            .map_err(|e| PreviewError::InvalidOp(format!("QR code generation failed: {}", e)))?;

        let cell_size = cell_size.max(1) as usize;
        let qr_size = code.width();
        let pixel_size = qr_size * cell_size;

        // Center the QR code within print area based on alignment
        let start_x = match self.state.style.alignment {
            Alignment::Left => 0,
            Alignment::Center => {
                if pixel_size < self.print_width {
                    (self.print_width - pixel_size) / 2
                } else {
                    0
                }
            }
            Alignment::Right => self.print_width.saturating_sub(pixel_size),
        };

        self.ensure_height(self.state.y + pixel_size);

        // Render QR modules
        for qy in 0..qr_size {
            for qx in 0..qr_size {
                let is_dark = code[(qx, qy)] == qrcode::Color::Dark;

                for cy in 0..cell_size {
                    for cx in 0..cell_size {
                        let px = start_x + qx * cell_size + cx;
                        let py = self.state.y + qy * cell_size + cy;
                        self.set_print_pixel(px, py, is_dark);
                    }
                }
            }
        }

        self.state.y += pixel_size;
        self.state.x = 0;

        Ok(())
    }

    /// Render a PDF417 barcode.
    fn render_pdf417(&mut self, data: &str, module_width: u8) -> Result<(), PreviewError> {
        use pdf417::{END_PATTERN, PDF417, PDF417Encoder, START_PATTERN};

        // Configuration
        const COLS: u8 = 4;
        const ROWS: u8 = 10;
        // PDF417 width = start(17) + left_row_ind(17) + data_cols*17 + right_row_ind(17) + end(18)
        const WIDTH: usize = START_PATTERN.size() as usize
            + 17
            + (COLS as usize * 17)
            + 17
            + END_PATTERN.size() as usize;
        const HEIGHT: usize = ROWS as usize;

        // Encode the text data
        let mut codewords = [0u16; (ROWS * COLS) as usize];
        let (level, filled) = match PDF417Encoder::new(&mut codewords, false)
            .append_ascii(data)
            .fit_seal()
        {
            Some(result) => result,
            None => {
                // Data too long or encoding failed, show placeholder
                self.render_placeholder("PDF417", 200, 60);
                return Ok(());
            }
        };

        // Render to bool array
        let barcode = PDF417::new(filled, ROWS, COLS, level);
        let mut storage = [false; WIDTH * HEIGHT];
        for (i, bit) in barcode.bits().enumerate() {
            if i < storage.len() {
                storage[i] = bit;
            }
        }

        // Scale factors
        let scale_x = module_width.max(2) as usize;
        let scale_y = scale_x * 3; // PDF417 aspect ratio is typically 3:1

        let pixel_width = WIDTH * scale_x;
        let pixel_height = HEIGHT * scale_y;

        // Position based on alignment
        let start_x = match self.state.style.alignment {
            Alignment::Left => 0,
            Alignment::Center => {
                if pixel_width < self.print_width {
                    (self.print_width - pixel_width) / 2
                } else {
                    0
                }
            }
            Alignment::Right => self.print_width.saturating_sub(pixel_width),
        };

        self.ensure_height(self.state.y + pixel_height);

        // Render with scaling
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                let is_dark = storage[row * WIDTH + col];

                for sy in 0..scale_y {
                    for sx in 0..scale_x {
                        let px = start_x + col * scale_x + sx;
                        let py = self.state.y + row * scale_y + sy;
                        self.set_print_pixel(px, py, is_dark);
                    }
                }
            }
        }

        self.state.y += pixel_height;
        self.state.x = 0;

        Ok(())
    }

    /// Render a 1D barcode.
    fn render_barcode1d(&mut self, kind: BarcodeKind, data: &str, height: u8) {
        let bars = match kind {
            BarcodeKind::Code39 => encode_code39(data),
            BarcodeKind::Code128 => encode_code128(data),
            _ => {
                // For unsupported types, show placeholder
                self.render_placeholder(&format!("{:?}", kind), 200, height as usize);
                return;
            }
        };

        let bar_height = height.max(20) as usize;
        let bar_width = bars.len();

        // Center barcode within print area
        let start_x = if bar_width < self.print_width {
            (self.print_width - bar_width) / 2
        } else {
            0
        };

        self.ensure_height(self.state.y + bar_height);

        for (i, &bar) in bars.iter().enumerate() {
            if bar {
                for y in 0..bar_height {
                    self.set_print_pixel(start_x + i, self.state.y + y, true);
                }
            }
        }

        self.state.y += bar_height;
        self.state.x = 0;
    }

    /// Render a placeholder box with text.
    fn render_placeholder(&mut self, _text: &str, width: usize, height: usize) {
        let start_x = if width < self.print_width {
            (self.print_width - width) / 2
        } else {
            0
        };

        self.ensure_height(self.state.y + height);

        // Draw border
        for x in 0..width {
            self.set_print_pixel(start_x + x, self.state.y, true);
            self.set_print_pixel(start_x + x, self.state.y + height - 1, true);
        }
        for y in 0..height {
            self.set_print_pixel(start_x, self.state.y + y, true);
            self.set_print_pixel(start_x + width - 1, self.state.y + y, true);
        }

        // Draw diagonal lines (X pattern)
        for i in 0..width.min(height) {
            let x1 = start_x + i * width / height.max(1);
            let x2 = start_x + width - 1 - i * width / height.max(1);
            self.set_print_pixel(x1, self.state.y + i, true);
            self.set_print_pixel(x2, self.state.y + i, true);
        }

        self.state.y += height;
        self.state.x = 0;
    }

    /// Compute the height after trimming trailing empty rows.
    fn trimmed_height(&self, min: usize) -> usize {
        let mut h = self.height;
        while h > 0 {
            let row_start = (h - 1) * self.paper_width;
            let row_empty = self.buffer[row_start..row_start + self.paper_width]
                .iter()
                .all(|&p| p == 0);
            if row_empty {
                h -= 1;
            } else {
                break;
            }
        }
        h.max(min)
    }

    /// Convert buffer to PNG bytes.
    fn to_png(&self) -> Result<Vec<u8>, PreviewError> {
        use image::ImageEncoder;

        let actual_height = self.trimmed_height(10);

        let mut img = GrayImage::new(self.paper_width as u32, actual_height as u32);

        for y in 0..actual_height {
            for x in 0..self.paper_width {
                let idx = y * self.paper_width + x;
                let is_black = self.buffer.get(idx).copied().unwrap_or(0) != 0;
                let color = if is_black { 0u8 } else { 255u8 };
                img.put_pixel(x as u32, y as u32, Luma([color]));
            }
        }

        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(
                img.as_raw(),
                self.paper_width as u32,
                actual_height as u32,
                image::ExtendedColorType::L8,
            )
            .map_err(|e: image::ImageError| PreviewError::ImageEncode(e.to_string()))?;

        Ok(png_bytes)
    }
}

/// Render a program to PNG bytes using TSP650II dimensions.
pub fn render_preview(program: &Program) -> Result<Vec<u8>, PreviewError> {
    let mut renderer = PreviewRenderer::tsp650ii();
    renderer.render(program)
}

/// Render a program to PNG bytes with a custom print width.
pub fn render_preview_with_width(
    program: &Program,
    print_width: usize,
) -> Result<Vec<u8>, PreviewError> {
    let mut renderer = PreviewRenderer::for_width(print_width);
    renderer.render(program)
}

/// Measure the rendered height of a program using TSP650II preview parameters.
///
/// Returns the same height that `to_preview_png()` would produce, without
/// generating the PNG. Useful for computing the total preview image height.
pub fn measure_preview(program: &Program) -> Result<usize, PreviewError> {
    measure_preview_with_width(program, 576)
}

/// Measure the rendered height of a program with a custom print width.
pub fn measure_preview_with_width(
    program: &Program,
    print_width: usize,
) -> Result<usize, PreviewError> {
    let mut renderer = PreviewRenderer::for_width(print_width);
    for op in &program.ops {
        renderer.process_op(op)?;
    }
    Ok(renderer.trimmed_height(10))
}

/// Measure the Y cursor position after processing a program with TSP650II
/// preview parameters.
///
/// Returns the pixel Y offset where the next component would be rendered.
/// Unlike `measure_preview`, this returns the cursor position (not the trimmed
/// buffer height), so it correctly accounts for whitespace/spacers.
pub fn measure_cursor_y(program: &Program) -> Result<usize, PreviewError> {
    measure_cursor_y_with_width(program, 576)
}

/// Measure the Y cursor position with a custom print width.
pub fn measure_cursor_y_with_width(
    program: &Program,
    print_width: usize,
) -> Result<usize, PreviewError> {
    let mut renderer = PreviewRenderer::for_width(print_width);
    for op in &program.ops {
        renderer.process_op(op)?;
    }
    Ok(renderer.state.y)
}

/// Raw raster output for printing.
pub struct RawRaster {
    /// Width in pixels (576 for TSP650II)
    pub width: usize,
    /// Height in pixels
    pub height: usize,
    /// Packed 1-bit pixel data (MSB first, 1 = black)
    pub data: Vec<u8>,
}

/// Render a program to raw 1-bit raster data with no margins.
///
/// Returns exactly 576 pixels wide (72mm at 203 DPI), packed 1-bit per pixel.
/// This is suitable for direct raster printing via `Op::Raster`.
pub fn render_raw(program: &Program) -> Result<RawRaster, PreviewError> {
    render_raw_with_width(program, 576)
}

/// Render a program to raw 1-bit raster data with a custom width and no margins.
pub fn render_raw_with_width(
    program: &Program,
    print_width: usize,
) -> Result<RawRaster, PreviewError> {
    let mut renderer = PreviewRenderer::new(print_width, print_width, 0, 0);

    for op in &program.ops {
        // Skip Cut ops - we want the content only
        if matches!(op, Op::Cut { .. }) {
            continue;
        }
        renderer.process_op(op)?;
    }

    let actual_height = renderer.trimmed_height(1);

    // Pack into 1-bit format
    let width = renderer.paper_width;
    let width_bytes = width.div_ceil(8);
    let mut data = vec![0u8; width_bytes * actual_height];

    for y in 0..actual_height {
        for x in 0..width {
            let src_idx = y * width + x;
            let is_black = renderer.buffer.get(src_idx).copied().unwrap_or(0) != 0;

            if is_black {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                data[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    Ok(RawRaster {
        width,
        height: actual_height,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = PreviewRenderer::tsp650ii();
        assert_eq!(renderer.paper_width, 640);
        assert_eq!(renderer.print_width, 576);
        assert_eq!(renderer.left_margin, 32);
        assert_eq!(renderer.top_margin, 16);
    }

    #[test]
    fn test_empty_program() {
        let program = Program::new();
        let result = render_preview(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_text() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Text("Hello".to_string()));
        program.push(Op::Newline);

        let result = render_preview(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_text() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::SetBold(true));
        program.push(Op::SetAlign(Alignment::Center));
        program.push(Op::Text("BOLD CENTER".to_string()));
        program.push(Op::Newline);

        let result = render_preview(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_feed() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Feed { units: 10 });

        let result = render_preview(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cut() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Cut { partial: false });

        let result = render_preview(&program);
        assert!(result.is_ok());
    }
}

//! TTF font rendering for custom fonts (IBM Plex Sans).
//!
//! Renders text to an anti-aliased f32 grayscale buffer using ab_glyph.
//! Used by Text and Banner components with `font: "ibm"` to produce
//! raster output with smooth edges (which then gets dithered).

use ab_glyph::{Font, FontArc, ScaleFont};
use std::sync::OnceLock;

static IBM_PLEX_REGULAR: OnceLock<FontArc> = OnceLock::new();
static IBM_PLEX_BOLD: OnceLock<FontArc> = OnceLock::new();

fn ibm_plex_regular() -> &'static FontArc {
    IBM_PLEX_REGULAR.get_or_init(|| {
        FontArc::try_from_slice(include_bytes!("fonts/IBMPlexSans-Regular.ttf"))
            .expect("Failed to load IBM Plex Sans Regular")
    })
}

fn ibm_plex_bold() -> &'static FontArc {
    IBM_PLEX_BOLD.get_or_init(|| {
        FontArc::try_from_slice(include_bytes!("fonts/IBMPlexSans-Bold.ttf"))
            .expect("Failed to load IBM Plex Sans Bold")
    })
}

/// Rendered TTF text as an anti-aliased grayscale buffer.
pub struct TtfRender {
    pub width: usize,
    pub height: usize,
    /// Intensity values: 0.0 = white, 1.0 = black, with intermediate for anti-aliasing.
    pub data: Vec<f32>,
}

/// Map text size parameter to pixel height.
///
/// Matches the existing bitmap font sizes:
/// - 0 → 17px (Font B/C equivalent)
/// - 1 → 24px (Font A equivalent)
/// - 2 → 48px (double)
/// - 3 → 72px (triple)
pub fn size_to_pixel_height(size: [u8; 2]) -> f32 {
    match size[0] {
        0 => 17.0,
        1 => 24.0,
        n => 24.0 * n as f32,
    }
}

/// Render text using a TTF font.
///
/// Returns an anti-aliased grayscale buffer. The caller is responsible for
/// dithering the result to 1-bit before emitting as `Op::Raster`.
pub fn render_ttf_text(
    text: &str,
    font_name: &str,
    bold: bool,
    pixel_height: f32,
    max_width: usize,
) -> TtfRender {
    let font = match font_name {
        "ibm" => {
            if bold {
                ibm_plex_bold()
            } else {
                ibm_plex_regular()
            }
        }
        _ => ibm_plex_regular(), // fallback
    };

    let scaled = font.as_scaled(pixel_height);

    // Layout: compute glyph positions
    let mut glyphs = Vec::new();
    let mut caret_x = 0.0f32;

    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        let advance = scaled.h_advance(glyph_id);

        glyphs.push((glyph_id, caret_x));
        caret_x += advance;
    }

    let text_width = caret_x.ceil() as usize;
    let width = text_width.min(max_width).max(1);

    // Compute line height from font metrics
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_height = (ascent - descent).ceil() as usize;
    let height = line_height.max(1);
    let baseline_y = ascent;

    let mut data = vec![0.0f32; width * height];

    // Rasterize each glyph
    for &(glyph_id, glyph_x) in &glyphs {
        let glyph = glyph_id.with_scale_and_position(
            pixel_height,
            ab_glyph::point(glyph_x, baseline_y),
        );

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|px, py, coverage| {
                let x = px as i32 + bounds.min.x as i32;
                let y = py as i32 + bounds.min.y as i32;

                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let idx = y as usize * width + x as usize;
                    // Accumulate coverage (clamped)
                    data[idx] = (data[idx] + coverage).min(1.0);
                }
            });
        }
    }

    TtfRender {
        width,
        height,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic_text() {
        let result = render_ttf_text("Hello", "ibm", false, 24.0, 576);
        assert!(result.width > 0);
        assert!(result.height > 0);
        assert_eq!(result.data.len(), result.width * result.height);
        // Should have some non-zero pixels (text rendered)
        assert!(result.data.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_render_bold_text() {
        let result = render_ttf_text("Bold", "ibm", true, 24.0, 576);
        assert!(result.width > 0);
        assert!(result.data.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_render_large_text() {
        let result = render_ttf_text("BIG", "ibm", false, 72.0, 576);
        assert!(result.height > 24); // Should be taller than default
    }

    #[test]
    fn test_size_to_pixel_height() {
        assert_eq!(size_to_pixel_height([0, 0]), 17.0);
        assert_eq!(size_to_pixel_height([1, 1]), 24.0);
        assert_eq!(size_to_pixel_height([2, 2]), 48.0);
        assert_eq!(size_to_pixel_height([3, 3]), 72.0);
    }

    #[test]
    fn test_anti_aliased_output() {
        let result = render_ttf_text("Smooth", "ibm", false, 48.0, 576);
        // Anti-aliased output should have intermediate values (not just 0.0 and 1.0)
        let has_intermediate = result.data.iter().any(|&v| v > 0.01 && v < 0.99);
        assert!(has_intermediate, "TTF rendering should produce anti-aliased (intermediate) values");
    }
}

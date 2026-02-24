//! Emit logic for the Canvas component.
//!
//! Renders elements onto an f32 intensity buffer using the same compositing
//! model as `render::composer`, then dithers to 1-bit and emits `Op::Raster`.

use serde::Serialize;

use super::Component;
use super::EmitContext;
use super::types::Canvas;
use crate::ir::{Op, Program};
use crate::preview::render_raw;
use crate::render::composer::BlendMode;
use crate::render::dither::{self, DitheringAlgorithm};
use crate::shader::lerp;

use super::graphics::parse_dither_algorithm;
use super::types::CanvasElement;
use crate::preview::RawRaster;

/// Bounding box of a rendered canvas element (content bounds in canvas space).
#[derive(Debug, Clone, Serialize)]
pub struct ElementLayout {
    /// Content bounds in canvas space (what the overlay displays).
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    /// Offset from content origin to element origin.
    /// element_position = content_position - content_offset
    pub content_offset_x: i32,
    pub content_offset_y: i32,
    /// Full rendered size of the element (before content-bounds cropping).
    pub full_width: usize,
    pub full_height: usize,
}

/// Internal measurement result from rendering an element.
struct ElementMeasurement {
    /// Full element size (from render_raw).
    full_width: usize,
    full_height: usize,
    /// Content bounds relative to element origin.
    /// None means all-white (fall back to full bounds).
    content_bounds: Option<(usize, usize, usize, usize)>, // (min_x, min_y, max_x, max_y) inclusive
}

/// Scan 1-bit packed raster data for the tight bounding box of non-white pixels.
///
/// Returns `Some((min_x, min_y, max_x, max_y))` inclusive, or `None` if all-white.
/// Uses byte-level scanning: skips entire zero bytes (8 white pixels at once).
fn content_bounds(raster: &RawRaster) -> Option<(usize, usize, usize, usize)> {
    let width = raster.width;
    let height = raster.height;
    if width == 0 || height == 0 {
        return None;
    }
    let width_bytes = width.div_ceil(8);

    let mut min_x = width;
    let mut max_x = 0usize;
    let mut min_y = height;
    let mut max_y = 0usize;

    for y in 0..height {
        let row_offset = y * width_bytes;
        for bx in 0..width_bytes {
            let byte = raster.data.get(row_offset + bx).copied().unwrap_or(0);
            if byte == 0 {
                continue;
            }
            // This byte has at least one black pixel
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            // Find leftmost and rightmost set bits in this byte
            let base_x = bx * 8;
            // Leading zeros → leftmost set bit
            let left = base_x + byte.leading_zeros() as usize;
            // Trailing zeros → rightmost set bit
            let right = base_x + 7 - byte.trailing_zeros() as usize;
            // Clamp to actual pixel width
            let right = right.min(width - 1);
            if left < min_x {
                min_x = left;
            }
            if right > max_x {
                max_x = right;
            }
        }
    }

    if min_y > max_y {
        // No black pixels found
        None
    } else {
        Some((min_x, min_y, max_x, max_y))
    }
}

/// Layout metadata for a canvas: overall dimensions and per-element bounding boxes.
#[derive(Debug, Clone, Serialize)]
pub struct CanvasLayout {
    pub width: usize,
    pub height: usize,
    pub elements: Vec<ElementLayout>,
}

impl Canvas {
    /// Emit IR ops for this canvas component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        if self.elements.is_empty() {
            return;
        }

        let canvas_width = self.width.unwrap_or(ctx.print_width);

        // Render each element to an f32 intensity buffer.
        // Elements without position flow top-to-bottom; positioned elements are independent.
        let mut rendered = Vec::new();
        let mut flow_y: i32 = 0;

        for el in &self.elements {
            if let Some(mut r) = render_element(el, canvas_width) {
                if el.position.is_none() {
                    // Flow mode: stack below previous flow elements
                    r.y = flow_y;
                    flow_y += r.height as i32;
                }
                rendered.push(r);
            }
        }

        if rendered.is_empty() {
            return;
        }

        // Determine canvas height: explicit or bounding box of all elements
        let canvas_height = self.height.unwrap_or_else(|| {
            rendered
                .iter()
                .map(|r| (r.y + r.height as i32).max(0) as usize)
                .max()
                .unwrap_or(1)
        });

        if canvas_height == 0 {
            return;
        }

        // Resolve dithering algorithm
        let dither_algo = self.resolve_dither();

        // Composite all elements onto a single f32 intensity buffer
        let raster_data = dither::generate_raster(
            canvas_width,
            canvas_height,
            |px, py, _w, _h| {
                let mut result: f32 = 0.0; // white background

                for el in &rendered {
                    let local_x = px as i32 - el.x;
                    let local_y = py as i32 - el.y;

                    if local_x < 0
                        || local_y < 0
                        || local_x >= el.width as i32
                        || local_y >= el.height as i32
                    {
                        continue;
                    }

                    let idx = local_y as usize * el.width + local_x as usize;
                    let intensity = el.intensity.get(idx).copied().unwrap_or(0.0);

                    let blended = el.blend_mode.apply(result, intensity);
                    result = lerp(result, blended, el.opacity);
                }

                result.clamp(0.0, 1.0)
            },
            dither_algo,
        );

        ctx.push(Op::Raster {
            width: canvas_width as u16,
            height: canvas_height as u16,
            data: raster_data,
        });
    }

    /// Compute the layout of all elements without compositing.
    ///
    /// Returns the canvas dimensions and each element's bounding box, using
    /// the same emit → render_raw → measure pipeline as `emit()`.
    ///
    /// The primary x/y/width/height in each `ElementLayout` represent the
    /// tight content bounds (non-white pixels). The `content_offset_x/y` fields
    /// give the offset from content origin to element origin, so the frontend
    /// can map drag positions back: `element_position = content_position - offset`.
    pub fn compute_layout(&self, print_width: usize) -> CanvasLayout {
        let canvas_width = self.width.unwrap_or(print_width);
        let mut layouts = Vec::new();
        // Track full element bottom edges for canvas height (must match emit())
        let mut full_bottoms: Vec<usize> = Vec::new();
        let mut flow_y: i32 = 0;

        for el in &self.elements {
            if let Some((mut elem_x, mut elem_y, measurement)) = measure_element(el, canvas_width) {
                if el.position.is_none() {
                    elem_x = 0;
                    elem_y = flow_y;
                    flow_y += measurement.full_height as i32;
                }

                // Track full element bounds for canvas height calculation
                full_bottoms.push((elem_y + measurement.full_height as i32).max(0) as usize);

                match measurement.content_bounds {
                    Some((min_x, min_y, max_x, max_y)) => {
                        // Content bounds in canvas space
                        let content_x = elem_x + min_x as i32;
                        let content_y = elem_y + min_y as i32;
                        let content_w = max_x - min_x + 1;
                        let content_h = max_y - min_y + 1;

                        // Offset: how far content origin is from element origin
                        let offset_x = content_x - elem_x;
                        let offset_y = content_y - elem_y;

                        layouts.push(ElementLayout {
                            x: content_x,
                            y: content_y,
                            width: content_w,
                            height: content_h,
                            content_offset_x: offset_x,
                            content_offset_y: offset_y,
                            full_width: measurement.full_width,
                            full_height: measurement.full_height,
                        });
                    }
                    None => {
                        // All-white element — fall back to full bounds
                        layouts.push(ElementLayout {
                            x: elem_x,
                            y: elem_y,
                            width: measurement.full_width,
                            height: measurement.full_height,
                            content_offset_x: 0,
                            content_offset_y: 0,
                            full_width: measurement.full_width,
                            full_height: measurement.full_height,
                        });
                    }
                }
            } else {
                // Element produced no output — zero-size placeholder
                layouts.push(ElementLayout {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                    content_offset_x: 0,
                    content_offset_y: 0,
                    full_width: 0,
                    full_height: 0,
                });
            }
        }

        // Canvas height uses full element bounds (must match emit() behavior)
        let canvas_height = self
            .height
            .unwrap_or_else(|| full_bottoms.iter().copied().max().unwrap_or(1));

        CanvasLayout {
            width: canvas_width,
            height: canvas_height,
            elements: layouts,
        }
    }

    /// Resolve the dithering algorithm for this canvas.
    fn resolve_dither(&self) -> DitheringAlgorithm {
        let dither_str = self.dither.as_deref().unwrap_or("auto");
        if dither_str == "auto" {
            if has_continuous_tone_content(&self.elements) {
                DitheringAlgorithm::Atkinson
            } else {
                DitheringAlgorithm::None
            }
        } else {
            parse_dither_algorithm(dither_str).unwrap_or(DitheringAlgorithm::None)
        }
    }
}

/// Detect if any elements produce continuous-tone (non-binary) content
/// that benefits from dithering.
fn has_continuous_tone_content(elements: &[CanvasElement]) -> bool {
    elements.iter().any(|e| match &e.component {
        Component::Pattern(_) | Component::Image(_) | Component::Chart(_) => true,
        Component::Text(t) => t.font.is_some(),
        Component::Banner(b) => b.font.is_some(),
        Component::Canvas(c) => has_continuous_tone_content(&c.elements),
        _ => false,
    })
}

/// A rendered element ready for compositing.
struct RenderedElement {
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    intensity: Vec<f32>,
    blend_mode: BlendMode,
    opacity: f32,
}

/// Measure a single canvas element: emit → render_raw → scan content bounds.
///
/// Returns the element's position, full size, and content bounds.
fn measure_element(element: &CanvasElement, canvas_width: usize) -> Option<(i32, i32, ElementMeasurement)> {
    let mut ctx = EmitContext::new(canvas_width);
    element.component.emit(&mut ctx);
    if ctx.ops.is_empty() {
        return None;
    }
    let program = Program { ops: ctx.ops };
    let raw = render_raw(&program).ok()?;
    let (x, y) = element.position.map(|p| (p.x, p.y)).unwrap_or((0, 0));
    let cb = content_bounds(&raw);
    Some((
        x,
        y,
        ElementMeasurement {
            full_width: raw.width,
            full_height: raw.height,
            content_bounds: cb,
        },
    ))
}

/// Render a single canvas element to an f32 intensity buffer.
///
/// Uses the standard path: emit IR ops → render_raw() → convert 1-bit to f32.
/// Position is set from the element's `position` field; flow positioning is
/// handled by the caller for elements without explicit position.
/// Returns None if the element produces no output.
fn render_element(element: &CanvasElement, canvas_width: usize) -> Option<RenderedElement> {
    let mut ctx = EmitContext::new(canvas_width);
    element.component.emit(&mut ctx);

    if ctx.ops.is_empty() {
        return None;
    }

    let program = Program { ops: ctx.ops };
    let raw = render_raw(&program).ok()?;

    // Convert 1-bit packed data to f32 intensity buffer
    let width = raw.width;
    let height = raw.height;
    let width_bytes = width.div_ceil(8);
    let mut intensity = vec![0.0f32; width * height];

    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (raw.data.get(byte_idx).copied().unwrap_or(0) >> bit_idx) & 1 == 1;
            if is_black {
                intensity[y * width + x] = 1.0;
            }
        }
    }

    let (x, y) = element.position.map(|p| (p.x, p.y)).unwrap_or((0, 0));

    Some(RenderedElement {
        x,
        y,
        width,
        height,
        intensity,
        blend_mode: element.blend_mode,
        opacity: element.opacity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Position;
    use crate::preview::RawRaster;

    /// Build a RawRaster with 1-bit packed data from a list of (x, y) black pixels.
    fn make_raster(width: usize, height: usize, black_pixels: &[(usize, usize)]) -> RawRaster {
        let width_bytes = width.div_ceil(8);
        let mut data = vec![0u8; width_bytes * height];
        for &(x, y) in black_pixels {
            if x < width && y < height {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                data[byte_idx] |= 1 << bit_idx;
            }
        }
        RawRaster {
            width,
            height,
            data,
        }
    }

    // ── content_bounds ──────────────────────────────────────────────────

    #[test]
    fn content_bounds_empty_raster() {
        let raster = make_raster(16, 8, &[]);
        assert_eq!(content_bounds(&raster), None);
    }

    #[test]
    fn content_bounds_zero_dimensions() {
        let raster = RawRaster {
            width: 0,
            height: 0,
            data: vec![],
        };
        assert_eq!(content_bounds(&raster), None);
    }

    #[test]
    fn content_bounds_single_pixel() {
        let raster = make_raster(16, 8, &[(5, 3)]);
        assert_eq!(content_bounds(&raster), Some((5, 3, 5, 3)));
    }

    #[test]
    fn content_bounds_full_row() {
        // Black pixels spanning a full 16px row
        let pixels: Vec<_> = (0..16).map(|x| (x, 2)).collect();
        let raster = make_raster(16, 4, &pixels);
        let bounds = content_bounds(&raster).unwrap();
        assert_eq!(bounds, (0, 2, 15, 2));
    }

    #[test]
    fn content_bounds_tight_box() {
        // Draw a small rectangle: x=10..20, y=5..15 in a 576x100 raster
        let mut pixels = Vec::new();
        for y in 5..=15 {
            for x in 10..=20 {
                pixels.push((x, y));
            }
        }
        let raster = make_raster(576, 100, &pixels);
        let bounds = content_bounds(&raster).unwrap();
        assert_eq!(bounds, (10, 5, 20, 15));
    }

    #[test]
    fn content_bounds_corners_only() {
        // Pixels only at opposite corners
        let raster = make_raster(576, 200, &[(0, 0), (575, 199)]);
        let bounds = content_bounds(&raster).unwrap();
        assert_eq!(bounds, (0, 0, 575, 199));
    }

    #[test]
    fn content_bounds_non_byte_aligned_width() {
        // Width=10 (not a multiple of 8): 2 bytes per row, last byte partial
        let raster = make_raster(10, 4, &[(9, 1), (0, 2)]);
        let bounds = content_bounds(&raster).unwrap();
        assert_eq!(bounds, (0, 1, 9, 2));
    }

    #[test]
    fn content_bounds_single_byte_boundaries() {
        // Test pixel at bit positions 0 and 7 within a byte
        let raster = make_raster(8, 1, &[(0, 0), (7, 0)]);
        let bounds = content_bounds(&raster).unwrap();
        assert_eq!(bounds, (0, 0, 7, 0));
    }

    // ── compute_layout ──────────────────────────────────────────────────

    fn text_element(content: &str, position: Option<Position>) -> CanvasElement {
        CanvasElement {
            component: Component::Text(super::super::types::Text {
                content: content.into(),
                ..Default::default()
            }),
            position,
            blend_mode: Default::default(),
            opacity: 1.0,
        }
    }

    fn centered_text_element(content: &str, position: Option<Position>) -> CanvasElement {
        CanvasElement {
            component: Component::Text(super::super::types::Text {
                content: content.into(),
                center: true,
                ..Default::default()
            }),
            position,
            blend_mode: Default::default(),
            opacity: 1.0,
        }
    }

    #[test]
    fn layout_content_bounds_narrower_than_full() {
        // Centered text renders at 576px wide but content is narrower
        let canvas = Canvas {
            elements: vec![centered_text_element("Hi", None)],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        assert_eq!(layout.elements.len(), 1);
        let el = &layout.elements[0];
        // Content bounds should be narrower than full width
        assert!(
            el.width < el.full_width,
            "content width {} should be < full width {}",
            el.width,
            el.full_width
        );
        assert_eq!(el.full_width, 576);
        // Content offset should reflect the left whitespace
        assert!(
            el.content_offset_x > 0,
            "centered text should have positive x offset, got {}",
            el.content_offset_x
        );
    }

    #[test]
    fn layout_offsets_map_back_to_element_position() {
        // An element at position (50, 100) should be recoverable from content bounds - offset
        let canvas = Canvas {
            elements: vec![centered_text_element("X", Some(Position { x: 50, y: 100 }))],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        let el = &layout.elements[0];
        // element_position = content_position - content_offset
        let recovered_x = el.x - el.content_offset_x;
        let recovered_y = el.y - el.content_offset_y;
        assert_eq!(recovered_x, 50);
        assert_eq!(recovered_y, 100);
    }

    #[test]
    fn layout_flow_elements_stack() {
        // Two flow elements (no position) should stack vertically
        let canvas = Canvas {
            elements: vec![text_element("First", None), text_element("Second", None)],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        assert_eq!(layout.elements.len(), 2);
        let first = &layout.elements[0];
        let second = &layout.elements[1];
        // Second element starts below first element's full height in the flow.
        // Its content y includes its own content_offset_y on top of the flow position.
        assert!(
            second.y >= first.full_height as i32,
            "second.y ({}) should be >= first.full_height ({})",
            second.y,
            first.full_height
        );
    }

    #[test]
    fn layout_full_dimensions_populated() {
        let canvas = Canvas {
            elements: vec![text_element("Hello", None)],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        let el = &layout.elements[0];
        // Full dimensions should be non-zero and >= content dimensions
        assert!(el.full_width > 0);
        assert!(el.full_height > 0);
        assert!(el.full_width >= el.width);
        assert!(el.full_height >= el.height);
    }

    #[test]
    fn layout_empty_canvas() {
        let canvas = Canvas {
            elements: vec![],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        assert!(layout.elements.is_empty());
    }

    #[test]
    fn layout_canvas_height_uses_full_bounds() {
        // Canvas auto-height should encompass full element bounds, not just content
        let canvas = Canvas {
            elements: vec![centered_text_element("Test", None)],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        let el = &layout.elements[0];
        // Canvas height must be at least full_height (not just content height)
        assert!(
            layout.height >= el.full_height,
            "canvas height ({}) should be >= element full_height ({})",
            layout.height,
            el.full_height
        );
    }

    #[test]
    fn layout_positioned_element_preserves_offset() {
        let canvas = Canvas {
            elements: vec![centered_text_element(
                "AB",
                Some(Position { x: 100, y: 50 }),
            )],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        let el = &layout.elements[0];
        // Content x should be element x (100) + some content offset
        assert_eq!(el.x, 100 + el.content_offset_x);
        assert_eq!(el.y, 50 + el.content_offset_y);
    }

    #[test]
    fn layout_allwhite_element_falls_back_to_full_bounds() {
        // A spacer produces whitespace — all-white output
        let canvas = Canvas {
            elements: vec![CanvasElement {
                component: Component::Spacer(super::super::types::Spacer {
                    mm: Some(5.0),
                    ..Default::default()
                }),
                position: None,
                blend_mode: Default::default(),
                opacity: 1.0,
            }],
            ..Default::default()
        };
        let layout = canvas.compute_layout(576);
        let el = &layout.elements[0];
        // All-white: content bounds = full bounds, offset = (0, 0)
        assert_eq!(el.content_offset_x, 0);
        assert_eq!(el.content_offset_y, 0);
        assert_eq!(el.width, el.full_width);
        assert_eq!(el.height, el.full_height);
    }
}

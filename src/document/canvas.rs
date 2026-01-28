//! Emit logic for the Canvas component.
//!
//! Renders elements onto an f32 intensity buffer using the same compositing
//! model as `render::composer`, then dithers to 1-bit and emits `Op::Raster`.

use serde::Serialize;

use super::types::Canvas;
use super::Component;
use crate::ir::{Op, Program};
use crate::preview::render_raw;
use crate::render::composer::BlendMode;
use crate::render::dither::{self, DitheringAlgorithm};
use crate::shader::lerp;

use super::graphics::parse_dither_algorithm;
use super::types::CanvasElement;

/// Bounding box of a rendered canvas element.
#[derive(Debug, Clone, Serialize)]
pub struct ElementLayout {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
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
    pub fn emit(&self, ops: &mut Vec<Op>) {
        if self.elements.is_empty() {
            return;
        }

        let canvas_width = self.width.unwrap_or(576);

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

        ops.push(Op::Raster {
            width: canvas_width as u16,
            height: canvas_height as u16,
            data: raster_data,
        });
    }

    /// Compute the layout of all elements without compositing.
    ///
    /// Returns the canvas dimensions and each element's bounding box, using
    /// the same emit → render_raw → measure pipeline as `emit()`.
    pub fn compute_layout(&self) -> CanvasLayout {
        let canvas_width = self.width.unwrap_or(576);
        let mut layouts = Vec::new();
        let mut flow_y: i32 = 0;

        for el in &self.elements {
            if let Some((mut x, mut y, w, h)) = measure_element(el) {
                if el.position.is_none() {
                    x = 0;
                    y = flow_y;
                    flow_y += h as i32;
                }
                layouts.push(ElementLayout {
                    x,
                    y,
                    width: w,
                    height: h,
                });
            } else {
                // Element produced no output — zero-size placeholder
                layouts.push(ElementLayout {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                });
            }
        }

        let canvas_height = self.height.unwrap_or_else(|| {
            layouts
                .iter()
                .map(|l| (l.y + l.height as i32).max(0) as usize)
                .max()
                .unwrap_or(1)
        });

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

/// Measure a single canvas element: emit → render_raw → return (x, y, width, height).
/// Lightweight version of `render_element` that skips the intensity buffer.
fn measure_element(element: &CanvasElement) -> Option<(i32, i32, usize, usize)> {
    let mut sub_ops = Vec::new();
    element.component.emit(&mut sub_ops);
    if sub_ops.is_empty() {
        return None;
    }
    let program = Program { ops: sub_ops };
    let raw = render_raw(&program).ok()?;
    let (x, y) = element.position.map(|p| (p.x, p.y)).unwrap_or((0, 0));
    Some((x, y, raw.width, raw.height))
}

/// Render a single canvas element to an f32 intensity buffer.
///
/// Uses the standard path: emit IR ops → render_raw() → convert 1-bit to f32.
/// Position is set from the element's `position` field; flow positioning is
/// handled by the caller for elements without explicit position.
/// Returns None if the element produces no output.
fn render_element(element: &CanvasElement, _canvas_width: usize) -> Option<RenderedElement> {
    let mut sub_ops = Vec::new();
    element.component.emit(&mut sub_ops);

    if sub_ops.is_empty() {
        return None;
    }

    let program = Program { ops: sub_ops };
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

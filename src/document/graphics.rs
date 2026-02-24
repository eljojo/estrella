//! Emit logic for graphics components: Image, Pattern, NvLogo.

use super::types::{Chart, Image, NvLogo, Pattern};
use super::EmitContext;
use crate::ir::Op;
use crate::render::{chart, dither, patterns};

/// Parse a dithering algorithm string.
pub(crate) fn parse_dither_algorithm(s: &str) -> Option<dither::DitheringAlgorithm> {
    match s.to_lowercase().as_str() {
        "bayer" => Some(dither::DitheringAlgorithm::Bayer),
        "floyd-steinberg" | "floyd_steinberg" | "fs" => {
            Some(dither::DitheringAlgorithm::FloydSteinberg)
        }
        "atkinson" => Some(dither::DitheringAlgorithm::Atkinson),
        "jarvis" | "jjn" => Some(dither::DitheringAlgorithm::Jarvis),
        "none" | "threshold" => Some(dither::DitheringAlgorithm::None),
        _ => None,
    }
}

impl Image {
    /// Emit IR ops for this image component.
    ///
    /// Requires that `resolved_data` has been populated by calling
    /// `Document::resolve()` before compilation.
    pub fn emit(&self, ctx: &mut EmitContext) {
        if let Some(ref resolved) = self.resolved_data {
            let print_width: u16 = ctx.print_width as u16;
            if resolved.width < print_width {
                let align = self.align.as_deref().unwrap_or("center");
                let position = match align {
                    "left" => 0,
                    "right" => print_width - resolved.width,
                    _ => (print_width - resolved.width) / 2,
                };
                if position > 0 {
                    ctx.push(Op::SetAbsolutePosition(position));
                }
            }
            ctx.push(Op::Raster {
                width: resolved.width,
                height: resolved.height,
                data: resolved.raster_data.clone(),
            });
        }
    }
}

impl Pattern {
    /// Emit IR ops for this pattern component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        // Look up pattern by name
        let Some(mut pattern_impl) = patterns::by_name(&self.name) else {
            return; // Unknown pattern — emit nothing
        };

        // Apply custom params
        for (key, value) in &self.params {
            let _ = pattern_impl.set_param(key, value);
        }

        let height = self.height.unwrap_or(500);
        let width = ctx.print_width;

        // Parse dithering algorithm
        let dithering = self
            .dither
            .as_deref()
            .and_then(parse_dither_algorithm)
            .unwrap_or(dither::DitheringAlgorithm::Bayer);

        let data = patterns::render(pattern_impl.as_ref(), width, height, dithering);

        // Emit raster graphics
        ctx.push(Op::Raster {
            width: width as u16,
            height: height as u16,
            data,
        });
    }
}

impl Chart {
    /// Emit IR ops for this chart component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        if self.values.is_empty() {
            return;
        }

        let width = ctx.print_width;

        let dithering = self
            .dither
            .as_deref()
            .and_then(parse_dither_algorithm)
            .unwrap_or(dither::DitheringAlgorithm::Bayer);

        let (data, w, h) = chart::render(self, width, dithering);

        if !data.is_empty() {
            ctx.push(Op::Raster {
                width: w,
                height: h,
                data,
            });
        }
    }
}

impl NvLogo {
    /// Emit IR ops for this NV logo component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        // Resolve scale: scale_x/scale_y take precedence over uniform scale
        let scale_x = self.scale_x.or(self.scale).unwrap_or(1).clamp(1, 2);
        let scale_y = self.scale_y.or(self.scale).unwrap_or(1).clamp(1, 2);

        // If centering is enabled, look up logo dimensions and position
        if self.center
            && let Some(raster) = crate::logos::get_raster(&self.key)
        {
            let print_width: u32 = ctx.print_width as u32;
            let scaled_width = (raster.width as u32) * (scale_x as u32);
            if scaled_width < print_width {
                let position = (print_width - scaled_width) / 2;
                ctx.push(Op::SetAbsolutePosition(position as u16));
            }
        }

        ctx.push(Op::NvPrint {
            key: self.key.clone(),
            scale_x,
            scale_y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EmitContext {
        EmitContext::new(576)
    }

    #[test]
    fn test_pattern_ripple() {
        let pattern = Pattern {
            name: "ripple".into(),
            height: Some(100),
            ..Default::default()
        };
        let mut ctx = ctx();
        pattern.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::Raster {
                width: 576,
                height: 100,
                ..
            }
        )));
    }

    #[test]
    fn test_pattern_unknown() {
        let pattern = Pattern {
            name: "unknown_pattern".into(),
            height: Some(100),
            ..Default::default()
        };
        let mut ctx = ctx();
        pattern.emit(&mut ctx);
        assert!(ctx.ops.is_empty());
    }

    #[test]
    fn test_nv_logo_default() {
        let logo = NvLogo {
            key: "A0".into(),
            ..Default::default()
        };
        let mut ctx = ctx();
        logo.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::NvPrint {
                key,
                scale_x: 1,
                scale_y: 1,
            } if key == "A0"
        )));
    }

    #[test]
    fn test_nv_logo_scaled() {
        let logo = NvLogo {
            key: "LG".into(),
            scale: Some(2),
            ..Default::default()
        };
        let mut ctx = ctx();
        logo.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::NvPrint {
                key,
                scale_x: 2,
                scale_y: 2,
            } if key == "LG"
        )));
    }

    #[test]
    fn test_image_unresolved() {
        let img = Image {
            url: "https://example.com/img.png".into(),
            ..Default::default()
        };
        let mut ctx = ctx();
        img.emit(&mut ctx);
        // Unresolved images emit nothing
        assert!(ctx.ops.is_empty());
    }
}

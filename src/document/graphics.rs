//! Emit logic for graphics components: Image, Pattern, NvLogo.

use super::types::{Image, NvLogo, Pattern};
use crate::ir::Op;
use crate::render::{dither, patterns};

/// Parse a dithering algorithm string.
fn parse_dither_algorithm(s: &str) -> Option<dither::DitheringAlgorithm> {
    match s.to_lowercase().as_str() {
        "bayer" => Some(dither::DitheringAlgorithm::Bayer),
        "floyd-steinberg" | "floyd_steinberg" | "fs" => Some(dither::DitheringAlgorithm::FloydSteinberg),
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
    pub fn emit(&self, ops: &mut Vec<Op>) {
        if let Some(ref resolved) = self.resolved_data {
            ops.push(Op::Raster {
                width: resolved.width,
                height: resolved.height,
                data: resolved.raster_data.clone(),
            });
        }
        // If not resolved, emit nothing (caller must resolve first)
    }
}

impl Pattern {
    /// Emit IR ops for this pattern component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        // Look up pattern by name
        let Some(mut pattern_impl) = patterns::by_name(&self.name) else {
            return; // Unknown pattern — emit nothing
        };

        // Apply custom params
        for (key, value) in &self.params {
            let _ = pattern_impl.set_param(key, value);
        }

        let height = self.height.unwrap_or(500);
        let width = 576; // default printer width

        // Parse dithering algorithm
        let dithering = self
            .dither
            .as_deref()
            .and_then(parse_dither_algorithm)
            .unwrap_or(dither::DitheringAlgorithm::Bayer);

        let data = patterns::render(pattern_impl.as_ref(), width, height, dithering);

        // Emit raster graphics
        ops.push(Op::Raster {
            width: width as u16,
            height: height as u16,
            data,
        });
    }
}

impl NvLogo {
    /// Emit IR ops for this NV logo component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        // Resolve scale: scale_x/scale_y take precedence over uniform scale
        let scale_x = self
            .scale_x
            .or(self.scale)
            .unwrap_or(1)
            .clamp(1, 2);
        let scale_y = self
            .scale_y
            .or(self.scale)
            .unwrap_or(1)
            .clamp(1, 2);

        // If centering is enabled, look up logo dimensions and position
        if self.center {
            if let Some(raster) = crate::logos::get_raster(&self.key) {
                let print_width: u32 = 576; // DEFAULT_PRINT_WIDTH
                let scaled_width = (raster.width as u32) * (scale_x as u32);
                if scaled_width < print_width {
                    let position = (print_width - scaled_width) / 2;
                    ops.push(Op::SetAbsolutePosition(position as u16));
                }
            }
        }

        ops.push(Op::NvPrint {
            key: self.key.clone(),
            scale_x,
            scale_y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_ripple() {
        let pattern = Pattern {
            name: "ripple".into(),
            height: Some(100),
            ..Default::default()
        };
        let mut ops = Vec::new();
        pattern.emit(&mut ops);
        assert!(ops.iter().any(|op| matches!(
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
        let mut ops = Vec::new();
        pattern.emit(&mut ops);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_nv_logo_default() {
        let logo = NvLogo {
            key: "A0".into(),
            ..Default::default()
        };
        let mut ops = Vec::new();
        logo.emit(&mut ops);
        assert!(ops.iter().any(|op| matches!(
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
        let mut ops = Vec::new();
        logo.emit(&mut ops);
        assert!(ops.iter().any(|op| matches!(
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
        let mut ops = Vec::new();
        img.emit(&mut ops);
        // Unresolved images emit nothing
        assert!(ops.is_empty());
    }
}

//! # Graphics Components
//!
//! Components for rendering images and patterns.

use super::Component;
use crate::ir::{GraphicsMode, Op};
use crate::render::patterns;

/// A raster image component.
///
/// ## Example
///
/// ```ignore
/// use estrella::components::Image;
///
/// // From raw raster data
/// let img = Image::from_raster(576, 100, data);
///
/// // Use band mode instead of raster mode
/// let img = Image::from_raster(576, 96, data).band_mode();
/// ```
pub struct Image {
    width: u16,
    height: u16,
    data: Vec<u8>,
    mode: GraphicsMode,
}

impl Image {
    /// Create an image from raw raster data.
    ///
    /// Data should be packed bits (1 bit per pixel, MSB first).
    /// Length must be `ceil(width/8) * height`.
    pub fn from_raster(width: u16, height: u16, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            mode: GraphicsMode::Raster,
        }
    }

    /// Use band mode (ESC k) instead of raster mode.
    ///
    /// Band mode sends graphics in 24-row chunks. More efficient for
    /// streaming, but height should be a multiple of 24.
    pub fn band_mode(mut self) -> Self {
        self.mode = GraphicsMode::Band;
        self
    }

    /// Use raster mode (ESC GS S) - the default.
    pub fn raster_mode(mut self) -> Self {
        self.mode = GraphicsMode::Raster;
        self
    }

    /// Get the graphics mode.
    pub fn mode(&self) -> GraphicsMode {
        self.mode
    }
}

impl Component for Image {
    fn emit(&self, ops: &mut Vec<Op>) {
        match self.mode {
            GraphicsMode::Raster => {
                ops.push(Op::Raster {
                    width: self.width,
                    height: self.height,
                    data: self.data.clone(),
                });
            }
            GraphicsMode::Band => {
                let width_bytes = self.width.div_ceil(8) as u8;
                ops.push(Op::Band {
                    width_bytes,
                    data: self.data.clone(),
                });
            }
        }
    }
}

/// A pattern component that renders a named pattern.
///
/// ## Example
///
/// ```
/// use estrella::components::Pattern;
///
/// let ripple = Pattern::new("ripple", 500);
/// let waves = Pattern::new("waves", 300).band_mode();
/// ```
pub struct Pattern {
    name: String,
    width: usize,
    height: usize,
    mode: GraphicsMode,
}

impl Pattern {
    /// Create a pattern with a name and height.
    ///
    /// Uses the default printer width (576 dots).
    pub fn new(name: impl Into<String>, height: usize) -> Self {
        Self {
            name: name.into(),
            width: 576,
            height,
            mode: GraphicsMode::Raster,
        }
    }

    /// Set the width in dots.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Use band mode (ESC k) instead of raster mode.
    pub fn band_mode(mut self) -> Self {
        self.mode = GraphicsMode::Band;
        self
    }

    /// Use raster mode (ESC GS S) - the default.
    pub fn raster_mode(mut self) -> Self {
        self.mode = GraphicsMode::Raster;
        self
    }
}

impl Component for Pattern {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Look up the pattern by name
        let pattern = match patterns::by_name(&self.name) {
            Some(p) => p,
            None => {
                // Unknown pattern - emit nothing
                // Could also emit an error or placeholder
                return;
            }
        };

        // Render the pattern to raster data
        let data = pattern.render(self.width, self.height);

        match self.mode {
            GraphicsMode::Raster => {
                ops.push(Op::Raster {
                    width: self.width as u16,
                    height: self.height as u16,
                    data,
                });
            }
            GraphicsMode::Band => {
                let width_bytes = (self.width as u16).div_ceil(8) as u8;
                ops.push(Op::Band { width_bytes, data });
            }
        }
    }
}

/// A component for printing NV (non-volatile) graphics.
///
/// NV graphics are stored in the printer's flash memory and can be
/// recalled quickly. Use this component to print logos or other
/// frequently-used images that have been stored with `estrella logo store`.
///
/// ## Example
///
/// ```ignore
/// use estrella::components::{Receipt, NvLogo, Text};
///
/// let receipt = Receipt::new()
///     .child(NvLogo::new("LG"))  // Print logo stored with key "LG"
///     .child(Text::new("Thank you!").center())
///     .cut();
/// ```
pub struct NvLogo {
    key: String,
    scale_x: u8,
    scale_y: u8,
}

impl NvLogo {
    /// Create an NV logo component with a 2-character key.
    ///
    /// The key must match a logo previously stored with `estrella logo store`.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            scale_x: 1,
            scale_y: 1,
        }
    }

    /// Set horizontal scale (1 = 1x, 2 = 2x).
    pub fn scale_x(mut self, scale: u8) -> Self {
        self.scale_x = scale.clamp(1, 2);
        self
    }

    /// Set vertical scale (1 = 1x, 2 = 2x).
    pub fn scale_y(mut self, scale: u8) -> Self {
        self.scale_y = scale.clamp(1, 2);
        self
    }

    /// Set both horizontal and vertical scale.
    pub fn scale(mut self, scale: u8) -> Self {
        self.scale_x = scale.clamp(1, 2);
        self.scale_y = scale.clamp(1, 2);
        self
    }

    /// Double the size (2x scale in both dimensions).
    pub fn double(self) -> Self {
        self.scale(2)
    }
}

impl Component for NvLogo {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::NvPrint {
            key: self.key.clone(),
            scale_x: self.scale_x,
            scale_y: self.scale_y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentExt;

    #[test]
    fn test_image_raster_mode() {
        let data = vec![0xFF; 72 * 10]; // 576 dots wide, 10 rows
        let img = Image::from_raster(576, 10, data.clone());
        let ir = img.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Raster {
                width: 576,
                height: 10,
                ..
            }
        )));
    }

    #[test]
    fn test_image_band_mode() {
        let data = vec![0xFF; 72 * 24]; // 576 dots wide, 24 rows (one band)
        let img = Image::from_raster(576, 24, data.clone()).band_mode();
        let ir = img.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Band {
                width_bytes: 72,
                ..
            }
        )));
    }

    #[test]
    fn test_pattern_ripple() {
        let pattern = Pattern::new("ripple", 100);
        let ir = pattern.compile();

        // Should emit a raster op
        assert!(ir.ops.iter().any(|op| matches!(
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
        let pattern = Pattern::new("unknown_pattern", 100);
        let ir = pattern.compile();

        // Should only have Init (no graphics for unknown pattern)
        assert_eq!(ir.len(), 1);
        assert_eq!(ir.ops[0], Op::Init);
    }

    #[test]
    fn test_pattern_band_mode() {
        let pattern = Pattern::new("waves", 96).band_mode(); // 96 = 4 bands of 24
        let ir = pattern.compile();

        assert!(ir.ops.iter().any(|op| matches!(op, Op::Band { .. })));
    }

    #[test]
    fn test_nv_logo_default() {
        let logo = NvLogo::new("A0");
        let ir = logo.compile();

        assert!(ir.ops.iter().any(|op| matches!(
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
        let logo = NvLogo::new("LG").scale(2);
        let ir = logo.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::NvPrint {
                key,
                scale_x: 2,
                scale_y: 2,
            } if key == "LG"
        )));
    }

    #[test]
    fn test_nv_logo_double() {
        let logo = NvLogo::new("A0").double();
        let ir = logo.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::NvPrint {
                scale_x: 2,
                scale_y: 2,
                ..
            }
        )));
    }
}

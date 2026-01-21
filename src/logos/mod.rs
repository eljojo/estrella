//! # Logo Registry
//!
//! A curated collection of logos maintained by Estrella. These logos can be:
//! - Rendered in PNG preview
//! - Synced to the printer's NV (non-volatile) memory
//! - Used in receipts via `NvLogo::new("KEY")`
//!
//! ## Usage
//!
//! ```ignore
//! use estrella::logos;
//!
//! // List all available logos
//! for logo in logos::all() {
//!     println!("{} - {}", logo.key, logo.name);
//! }
//!
//! // Get raster data for preview rendering
//! if let Some(raster) = logos::get_raster("A1") {
//!     println!("Star logo: {}x{}", raster.width, raster.height);
//! }
//! ```

pub mod ripple;
pub mod star;

pub use ripple::RippleLogo;
pub use star::Star;

/// Raster data for a logo.
///
/// Data is packed bits (1 bit per pixel, MSB first).
/// Length is `ceil(width/8) * height`.
#[derive(Debug, Clone)]
pub struct LogoRaster {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

/// A managed logo in the registry.
pub struct Logo {
    /// 2-character NV key (e.g., "A1", "LG")
    pub key: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Function that generates the raster data
    raster_fn: fn() -> LogoRaster,
}

impl Logo {
    /// Create a new logo definition.
    pub const fn new(key: &'static str, name: &'static str, raster_fn: fn() -> LogoRaster) -> Self {
        Self {
            key,
            name,
            raster_fn,
        }
    }

    /// Generate the raster data for this logo.
    pub fn raster(&self) -> LogoRaster {
        (self.raster_fn)()
    }
}

/// All registered logos.
static LOGOS: &[Logo] = &[
    Logo::new("A0", "ripple", RippleLogo::raster),
    Logo::new("A1", "star", Star::raster),
];

/// Get all registered logos.
pub fn all() -> &'static [Logo] {
    LOGOS
}

/// Look up a logo by key.
pub fn by_key(key: &str) -> Option<&'static Logo> {
    LOGOS.iter().find(|logo| logo.key == key)
}

/// Get raster data for a logo by key.
///
/// This is a convenience function for preview rendering.
pub fn get_raster(key: &str) -> Option<LogoRaster> {
    by_key(key).map(|logo| logo.raster())
}

/// List all registered logo keys.
pub fn list_keys() -> Vec<&'static str> {
    LOGOS.iter().map(|logo| logo.key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_star() {
        let star = by_key("A1");
        assert!(star.is_some());
        assert_eq!(star.unwrap().name, "star");
    }

    #[test]
    fn test_get_raster() {
        let raster = get_raster("A1").unwrap();
        assert_eq!(raster.width, 96);
        assert_eq!(raster.height, 96);
        assert!(!raster.data.is_empty());
    }

    #[test]
    fn test_unknown_key() {
        assert!(by_key("XX").is_none());
        assert!(get_raster("XX").is_none());
    }
}

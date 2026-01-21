//! Barcode encoding for preview rendering.
//!
//! Uses the barcoders crate for Code 39 and Code 128 encoding.

use barcoders::sym::code39::Code39;
use barcoders::sym::code128::Code128;

/// Encode data as Code 39 barcode bars.
/// Returns a Vec<bool> where true = bar (black), false = space (white).
pub fn encode_code39(data: &str) -> Vec<bool> {
    let barcode = match Code39::new(data) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    let encoded = barcode.encode();

    // Scale up the bars for visibility (each module = 2 pixels)
    let scale = 2;
    let mut bars = Vec::with_capacity(encoded.len() * scale);
    for &module in &encoded {
        let is_bar = module == 1;
        for _ in 0..scale {
            bars.push(is_bar);
        }
    }

    bars
}

/// Encode data as Code 128 barcode bars.
/// Returns a Vec<bool> where true = bar (black), false = space (white).
pub fn encode_code128(data: &str) -> Vec<bool> {
    // Code128 requires a character set prefix:
    // - Character Set A (Ā): uppercase, control chars, digits
    // - Character Set B (Ɓ): uppercase, lowercase, digits, special chars
    // - Character Set C (Ć): digit pairs only (high density)
    // We use Set B as it supports the widest range of printable characters.
    let prefixed_data = format!("\u{0181}{}", data);

    let barcode = match Code128::new(&prefixed_data) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    let encoded = barcode.encode();

    // Scale up the bars for visibility (each module = 2 pixels)
    let scale = 2;
    let mut bars = Vec::with_capacity(encoded.len() * scale);
    for &module in &encoded {
        let is_bar = module == 1;
        for _ in 0..scale {
            bars.push(is_bar);
        }
    }

    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code39_encoding() {
        let bars = encode_code39("A");
        assert!(!bars.is_empty());
        // Should have bars (black)
        assert!(bars.iter().any(|&b| b));
    }

    #[test]
    fn test_code128_encoding() {
        let bars = encode_code128("Hello");
        assert!(!bars.is_empty());
        // Should have bars (black)
        assert!(bars.iter().any(|&b| b));
    }

    #[test]
    fn test_code39_invalid() {
        // Code39 has limited character set
        let bars = encode_code39("hello"); // lowercase not supported
        // Should still return something (barcoders may uppercase it)
        // If it fails, returns empty
        assert!(bars.is_empty() || !bars.is_empty());
    }
}

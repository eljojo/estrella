//! # Code Page 437 Encoding
//!
//! Converts Unicode strings to CP437 single-byte encoding for StarPRNT printers.
//!
//! The printer must be set to Code Page 437 (`ESC GS t 1`) for these bytes
//! to render correctly. ASCII (U+0000–U+007F) passes through unchanged.
//! Characters not in CP437 are replaced with `?` and a warning is printed.

/// Encode a Unicode string as CP437 bytes.
///
/// - ASCII (U+0000–U+007F): passed through as-is
/// - CP437 upper half (128 mapped Unicode code points): single CP437 byte
/// - Unmapped characters: replaced with `?`, warning printed to stderr
pub fn encode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        if (ch as u32) < 0x80 {
            out.push(ch as u8);
        } else if let Some(byte) = unicode_to_cp437(ch) {
            out.push(byte);
        } else {
            eprintln!(
                "cp437: unmapped character '{}' (U+{:04X}), replacing with '?'",
                ch, ch as u32
            );
            out.push(b'?');
        }
    }
    out
}

/// Map a Unicode code point to its CP437 byte value (0x80–0xFF).
///
/// Returns `None` if the character has no CP437 representation.
/// Reference: IBM Code Page 437 character set.
fn unicode_to_cp437(ch: char) -> Option<u8> {
    // CP437 upper half: 128 entries mapping Unicode → byte 0x80–0xFF
    let byte = match ch {
        // 0x80–0x8F: Accented uppercase/lowercase
        'Ç' => 0x80, // U+00C7
        'ü' => 0x81, // U+00FC
        'é' => 0x82, // U+00E9
        'â' => 0x83, // U+00E2
        'ä' => 0x84, // U+00E4
        'à' => 0x85, // U+00E0
        'å' => 0x86, // U+00E5
        'ç' => 0x87, // U+00E7
        'ê' => 0x88, // U+00EA
        'ë' => 0x89, // U+00EB
        'è' => 0x8A, // U+00E8
        'ï' => 0x8B, // U+00EF
        'î' => 0x8C, // U+00EE
        'ì' => 0x8D, // U+00EC
        'Ä' => 0x8E, // U+00C4
        'Å' => 0x8F, // U+00C5

        // 0x90–0x9F: More accented, currency, ƒ
        'É' => 0x90, // U+00C9
        'æ' => 0x91, // U+00E6
        'Æ' => 0x92, // U+00C6
        'ô' => 0x93, // U+00F4
        'ö' => 0x94, // U+00F6
        'ò' => 0x95, // U+00F2
        'û' => 0x96, // U+00FB
        'ù' => 0x97, // U+00F9
        'ÿ' => 0x98, // U+00FF
        'Ö' => 0x99, // U+00D6
        'Ü' => 0x9A, // U+00DC
        '¢' => 0x9B, // U+00A2
        '£' => 0x9C, // U+00A3
        '¥' => 0x9D, // U+00A5
        '₧' => 0x9E, // U+20A7
        'ƒ' => 0x9F, // U+0192

        // 0xA0–0xAF: Spanish, fractions, punctuation
        'á' => 0xA0, // U+00E1
        'í' => 0xA1, // U+00ED
        'ó' => 0xA2, // U+00F3
        'ú' => 0xA3, // U+00FA
        'ñ' => 0xA4, // U+00F1
        'Ñ' => 0xA5, // U+00D1
        'ª' => 0xA6, // U+00AA
        'º' => 0xA7, // U+00BA
        '¿' => 0xA8, // U+00BF
        '⌐' => 0xA9, // U+2310
        '¬' => 0xAA, // U+00AC
        '½' => 0xAB, // U+00BD
        '¼' => 0xAC, // U+00BC
        '¡' => 0xAD, // U+00A1
        '«' => 0xAE, // U+00AB
        '»' => 0xAF, // U+00BB

        // 0xB0–0xB2: Shade blocks
        '░' => 0xB0, // U+2591
        '▒' => 0xB1, // U+2592
        '▓' => 0xB2, // U+2593

        // 0xB3–0xDA: Box drawing (single and double line)
        '│' => 0xB3, // U+2502
        '┤' => 0xB4, // U+2524
        '╡' => 0xB5, // U+2561
        '╢' => 0xB6, // U+2562
        '╖' => 0xB7, // U+2556
        '╕' => 0xB8, // U+2555
        '╣' => 0xB9, // U+2563
        '║' => 0xBA, // U+2551
        '╗' => 0xBB, // U+2557
        '╝' => 0xBC, // U+255D
        '╜' => 0xBD, // U+255C
        '╛' => 0xBE, // U+255B
        '┐' => 0xBF, // U+2510
        '└' => 0xC0, // U+2514
        '┴' => 0xC1, // U+2534
        '┬' => 0xC2, // U+252C
        '├' => 0xC3, // U+251C
        '─' => 0xC4, // U+2500
        '┼' => 0xC5, // U+253C
        '╞' => 0xC6, // U+255E
        '╟' => 0xC7, // U+255F
        '╚' => 0xC8, // U+255A
        '╔' => 0xC9, // U+2554
        '╩' => 0xCA, // U+2569
        '╦' => 0xCB, // U+2566
        '╠' => 0xCC, // U+2560
        '═' => 0xCD, // U+2550
        '╬' => 0xCE, // U+256C
        '╧' => 0xCF, // U+2567
        '╨' => 0xD0, // U+2568
        '╤' => 0xD1, // U+2564
        '╥' => 0xD2, // U+2565
        '╙' => 0xD3, // U+2559
        '╘' => 0xD4, // U+2558
        '╒' => 0xD5, // U+2552
        '╓' => 0xD6, // U+2553
        '╫' => 0xD7, // U+256B
        '╪' => 0xD8, // U+256A
        '┘' => 0xD9, // U+2518
        '┌' => 0xDA, // U+250C

        // 0xDB–0xDF: Block elements
        '█' => 0xDB, // U+2588
        '▄' => 0xDC, // U+2584
        '▌' => 0xDD, // U+258C
        '▐' => 0xDE, // U+2590
        '▀' => 0xDF, // U+2580

        // 0xE0–0xEF: Greek letters and math
        'α' => 0xE0, // U+03B1
        'ß' => 0xE1, // U+00DF
        'Γ' => 0xE2, // U+0393
        'π' => 0xE3, // U+03C0
        'Σ' => 0xE4, // U+03A3
        'σ' => 0xE5, // U+03C3
        'µ' => 0xE6, // U+00B5
        'τ' => 0xE7, // U+03C4
        'Φ' => 0xE8, // U+03A6
        'Θ' => 0xE9, // U+0398
        'Ω' => 0xEA, // U+03A9
        'δ' => 0xEB, // U+03B4
        '∞' => 0xEC, // U+221E
        'φ' => 0xED, // U+03C6
        'ε' => 0xEE, // U+03B5
        '∩' => 0xEF, // U+2229

        // 0xF0–0xFF: Math symbols, degree, etc.
        '≡' => 0xF0, // U+2261
        '±' => 0xF1, // U+00B1
        '≥' => 0xF2, // U+2265
        '≤' => 0xF3, // U+2264
        '⌠' => 0xF4, // U+2320
        '⌡' => 0xF5, // U+2321
        '÷' => 0xF6, // U+00F7
        '≈' => 0xF7, // U+2248
        '°' => 0xF8, // U+00B0
        '∙' => 0xF9, // U+2219
        '·' => 0xFA, // U+00B7
        '√' => 0xFB, // U+221A
        'ⁿ' => 0xFC, // U+207F
        '²' => 0xFD, // U+00B2
        '■' => 0xFE, // U+25A0
        '\u{00A0}' => 0xFF, // non-breaking space

        _ => return None,
    };
    Some(byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_passthrough() {
        assert_eq!(encode("Hello, world!"), b"Hello, world!");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(encode(""), b"");
    }

    #[test]
    fn test_accented_latin() {
        assert_eq!(encode("ñ"), vec![0xA4]);
        assert_eq!(encode("Ñ"), vec![0xA5]);
        assert_eq!(encode("é"), vec![0x82]);
        assert_eq!(encode("ü"), vec![0x81]);
        assert_eq!(encode("á"), vec![0xA0]);
        assert_eq!(encode("í"), vec![0xA1]);
        assert_eq!(encode("ó"), vec![0xA2]);
        assert_eq!(encode("ú"), vec![0xA3]);
    }

    #[test]
    fn test_spanish_text() {
        // "Año" → A=0x41, ñ=0xA4, o=0x6F
        assert_eq!(encode("Año"), vec![0x41, 0xA4, 0x6F]);
        // "¿Qué?" → ¿=0xA8, Q=0x51, u=0x75, é=0x82, ?=0x3F
        assert_eq!(encode("¿Qué?"), vec![0xA8, 0x51, 0x75, 0x82, 0x3F]);
    }

    #[test]
    fn test_box_drawing_single() {
        assert_eq!(encode("┌"), vec![0xDA]);
        assert_eq!(encode("─"), vec![0xC4]);
        assert_eq!(encode("┐"), vec![0xBF]);
        assert_eq!(encode("│"), vec![0xB3]);
        assert_eq!(encode("└"), vec![0xC0]);
        assert_eq!(encode("┘"), vec![0xD9]);
    }

    #[test]
    fn test_box_drawing_double() {
        assert_eq!(encode("╔"), vec![0xC9]);
        assert_eq!(encode("═"), vec![0xCD]);
        assert_eq!(encode("╗"), vec![0xBB]);
        assert_eq!(encode("║"), vec![0xBA]);
        assert_eq!(encode("╚"), vec![0xC8]);
        assert_eq!(encode("╝"), vec![0xBC]);
    }

    #[test]
    fn test_box_frame() {
        // A small box: ┌──┐
        let encoded = encode("┌──┐");
        assert_eq!(encoded, vec![0xDA, 0xC4, 0xC4, 0xBF]);
    }

    #[test]
    fn test_block_elements() {
        assert_eq!(encode("█"), vec![0xDB]);
        assert_eq!(encode("▄"), vec![0xDC]);
        assert_eq!(encode("▀"), vec![0xDF]);
    }

    #[test]
    fn test_math_symbols() {
        assert_eq!(encode("°"), vec![0xF8]);
        assert_eq!(encode("±"), vec![0xF1]);
        assert_eq!(encode("²"), vec![0xFD]);
        assert_eq!(encode("π"), vec![0xE3]);
    }

    #[test]
    fn test_unmapped_char_becomes_question_mark() {
        // Emoji has no CP437 representation
        assert_eq!(encode("★"), vec![b'?']);
    }

    #[test]
    fn test_mixed_ascii_and_extended() {
        // "Café" → C=0x43, a=0x61, f=0x66, é=0x82
        assert_eq!(encode("Café"), vec![0x43, 0x61, 0x66, 0x82]);
    }
}

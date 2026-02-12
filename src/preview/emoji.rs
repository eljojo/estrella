//! DoCoMo emoji sprite sheet rendering for thermal printing.
//!
//! Provides emoji rendering by extracting sprites from an embedded sprite sheet
//! and compositing with text. Supports the original DoCoMo emoji set.

use image::Pixel;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Embedded DoCoMo emoji sprite sheet (1-bit PNG, 267x267 pixels).
/// 18x18 grid with 12x12 pixel cells and 3px spacing.
const DOCOMO_SPRITE_PNG: &[u8] = include_bytes!("emoji/docomo.png");

/// Sprite sheet dimensions.
const SHEET_WIDTH: usize = 267;
const SHEET_HEIGHT: usize = 267;

/// Grid parameters.
const CELL_SIZE: usize = 12;
const CELL_SPACING: usize = 3;
const STRIDE: usize = CELL_SIZE + CELL_SPACING; // 15px

/// Cached decoded sprite sheet pixels (0 = white, 1 = black).
static SPRITE_PIXELS: OnceLock<Vec<u8>> = OnceLock::new();

/// Cached emoji mapping (Unicode char → grid position).
static EMOJI_MAP: OnceLock<HashMap<char, (usize, usize)>> = OnceLock::new();

/// Cached keycap sequence mapping (e.g., "1️⃣" → grid position).
static KEYCAP_MAP: OnceLock<HashMap<&'static str, (usize, usize)>> = OnceLock::new();

/// Emoji bitmap extracted from sprite sheet.
pub struct EmojiBitmap {
    pub width: usize,
    pub height: usize,
    /// Pixel data: 0 = white, 1 = black.
    pub data: Vec<u8>,
}

/// Emoji grayscale buffer for anti-aliased compositing.
pub struct EmojiGrayscale {
    pub width: usize,
    pub height: usize,
    /// Intensity values: 0.0 = white, 1.0 = black.
    pub data: Vec<f32>,
}

/// Decode and cache the sprite sheet pixels.
fn get_sprite_pixels() -> &'static Vec<u8> {
    SPRITE_PIXELS.get_or_init(|| {
        let img = image::load_from_memory(DOCOMO_SPRITE_PNG)
            .expect("Failed to decode embedded emoji sprite sheet");

        let gray = img.to_luma8();
        let mut pixels = vec![0u8; SHEET_WIDTH * SHEET_HEIGHT];

        for (x, y, pixel) in gray.enumerate_pixels() {
            if x < SHEET_WIDTH as u32 && y < SHEET_HEIGHT as u32 {
                let idx = (y as usize) * SHEET_WIDTH + (x as usize);
                // Threshold: anything darker than 128 is black
                pixels[idx] = if pixel.channels()[0] < 128 { 1 } else { 0 };
            }
        }

        pixels
    })
}

/// Build and cache the emoji mapping table.
/// Maps Unicode codepoints to (row, col) grid positions.
fn get_emoji_map() -> &'static HashMap<char, (usize, usize)> {
    EMOJI_MAP.get_or_init(|| {
        let mut map = HashMap::new();

        // Row 0: Faces (happy)
        map.insert('😃', (0, 0));
        map.insert('😁', (0, 1));
        map.insert('😆', (0, 2));
        map.insert('😅', (0, 3));
        map.insert('😉', (0, 4));
        map.insert('😍', (0, 5));
        map.insert('😋', (0, 6));
        map.insert('😜', (0, 7));
        map.insert('😏', (0, 8));
        map.insert('😒', (0, 9));
        map.insert('😌', (0, 10));
        map.insert('😔', (0, 11));
        map.insert('😵', (0, 12));
        map.insert('😢', (0, 13));
        map.insert('😭', (0, 14));
        map.insert('😱', (0, 15));
        map.insert('😖', (0, 16));
        map.insert('😣', (0, 17));

        // Row 1: More faces
        map.insert('😞', (1, 0));
        map.insert('😓', (1, 1));
        map.insert('😡', (1, 2));
        map.insert('😠', (1, 3));

        // Row 2: Hands and people
        map.insert('✋', (2, 0));
        map.insert('✌', (2, 1));
        map.insert('👍', (2, 2));
        map.insert('✊', (2, 3));
        map.insert('👊', (2, 4));
        map.insert('👂', (2, 5));
        map.insert('👀', (2, 6));
        map.insert('🏃', (2, 7));
        map.insert('🏂', (2, 8));
        map.insert('👤', (2, 9));
        map.insert('👣', (2, 10));

        // Row 3: Animals and nature
        map.insert('🐶', (3, 0));
        map.insert('🐱', (3, 1));
        map.insert('🐴', (3, 2));
        map.insert('🐷', (3, 3));
        map.insert('🐤', (3, 4));
        map.insert('🐧', (3, 5));
        map.insert('🐟', (3, 6));
        map.insert('🐌', (3, 7));
        map.insert('🌸', (3, 8));
        map.insert('🌷', (3, 9));
        map.insert('🌱', (3, 10));
        map.insert('🍀', (3, 11));
        map.insert('🍁', (3, 12));
        map.insert('🌑', (3, 13));
        map.insert('🌓', (3, 14));
        map.insert('🌔', (3, 15));
        map.insert('🌕', (3, 16));
        map.insert('🌙', (3, 17));

        // Row 4: Weather
        map.insert('☀', (4, 0));
        map.insert('☁', (4, 1));
        map.insert('🌀', (4, 2));
        map.insert('🌂', (4, 3));
        map.insert('☔', (4, 4));
        map.insert('⚡', (4, 5));
        map.insert('⛄', (4, 6));
        map.insert('💧', (4, 7));
        map.insert('🌊', (4, 8));

        // Row 5: Food
        map.insert('🍌', (5, 0));
        map.insert('🍎', (5, 1));
        map.insert('🍒', (5, 2));
        map.insert('🍞', (5, 3));
        map.insert('🍔', (5, 4));
        map.insert('🍙', (5, 5));
        map.insert('🍜', (5, 6));
        map.insert('🎂', (5, 7));
        map.insert('🍰', (5, 8));
        map.insert('☕', (5, 9));
        map.insert('🍵', (5, 10));
        map.insert('🍶', (5, 11));
        map.insert('🍷', (5, 12));
        map.insert('🍸', (5, 13));
        map.insert('🍺', (5, 14));
        map.insert('🍴', (5, 15));

        // Row 6: Activities and card suits
        map.insert('🎄', (6, 0));
        map.insert('✨', (6, 1));
        map.insert('🎁', (6, 2));
        map.insert('🎫', (6, 3));
        map.insert('⚽', (6, 4));
        map.insert('⚾', (6, 5));
        map.insert('🏀', (6, 6));
        map.insert('🎾', (6, 7));
        map.insert('⛳', (6, 8));
        map.insert('🎽', (6, 9));
        map.insert('🎿', (6, 10));
        map.insert('🎮', (6, 11));
        map.insert('♠', (6, 12));
        map.insert('♥', (6, 13));
        map.insert('♦', (6, 14));
        map.insert('♣', (6, 15));
        map.insert('🎨', (6, 16));

        // Row 7: Places
        map.insert('🗻', (7, 0));
        map.insert('🏠', (7, 1));
        map.insert('🏢', (7, 2));
        map.insert('🏣', (7, 3));
        map.insert('🏥', (7, 4));
        map.insert('🏦', (7, 5));
        map.insert('🏨', (7, 6));
        map.insert('🏪', (7, 7));
        map.insert('🏫', (7, 8));
        map.insert('🌁', (7, 9));
        map.insert('🌃', (7, 10));
        map.insert('♨', (7, 11));
        map.insert('🎠', (7, 12));
        map.insert('🎪', (7, 13));
        map.insert('🚃', (7, 14));
        map.insert('🚄', (7, 15));
        map.insert('🚌', (7, 16));
        map.insert('🚗', (7, 17));

        // Row 8: Transport
        map.insert('🚙', (8, 0));
        map.insert('🚲', (8, 1));
        map.insert('⛽', (8, 2));
        map.insert('🚥', (8, 3));
        map.insert('⛵', (8, 4));
        map.insert('🚢', (8, 5));
        map.insert('✈', (8, 6));
        map.insert('💺', (8, 7));

        // Row 9: Objects
        map.insert('⏳', (9, 0));
        map.insert('⌚', (9, 1));
        map.insert('⏰', (9, 2));
        map.insert('🎀', (9, 3));
        map.insert('👓', (9, 4));
        map.insert('👕', (9, 5));
        map.insert('👖', (9, 6));
        map.insert('👛', (9, 7));
        map.insert('👜', (9, 8));
        map.insert('👝', (9, 9));
        map.insert('👟', (9, 10));
        map.insert('👠', (9, 11));
        map.insert('👑', (9, 12));
        map.insert('🎩', (9, 13));
        map.insert('💄', (9, 14));
        map.insert('💍', (9, 15));
        map.insert('🔔', (9, 16));
        map.insert('🎵', (9, 17));

        // Row 10: Media and tech
        map.insert('🎶', (10, 0));
        map.insert('🎤', (10, 1));
        map.insert('🎧', (10, 2));
        map.insert('📱', (10, 3));
        map.insert('📲', (10, 4));
        map.insert('☎', (10, 5));
        map.insert('📟', (10, 6));
        map.insert('📠', (10, 7));
        map.insert('💻', (10, 8));
        map.insert('💿', (10, 9));
        map.insert('🎥', (10, 10));
        map.insert('🎬', (10, 11));
        map.insert('📺', (10, 12));
        map.insert('📷', (10, 13));
        map.insert('🔍', (10, 14));
        map.insert('💡', (10, 15));
        map.insert('📖', (10, 16));
        map.insert('💰', (10, 17));

        // Row 11: Office items
        map.insert('💴', (11, 0));
        map.insert('✉', (11, 1));
        map.insert('📩', (11, 2));
        map.insert('✏', (11, 3));
        map.insert('✒', (11, 4));
        map.insert('📝', (11, 5));
        map.insert('📎', (11, 6));
        map.insert('✂', (11, 7));
        map.insert('🔑', (11, 8));
        map.insert('💣', (11, 9));
        map.insert('🔧', (11, 10));
        map.insert('🚪', (11, 11));
        map.insert('🚬', (11, 12));

        // Row 12: Hearts and symbols
        map.insert('💌', (12, 0));
        map.insert('💓', (12, 1));
        map.insert('💕', (12, 2));
        map.insert('💔', (12, 3));
        map.insert('❤', (12, 4));
        map.insert('💋', (12, 5));
        map.insert('💢', (12, 6));
        map.insert('💥', (12, 7));
        map.insert('💦', (12, 8));
        map.insert('💨', (12, 9));
        map.insert('💤', (12, 10));
        map.insert('🏧', (12, 11));
        map.insert('♿', (12, 12));
        map.insert('🚻', (12, 13));
        map.insert('⚠', (12, 14));
        map.insert('🚭', (12, 15));
        map.insert('↗', (12, 16));
        map.insert('↘', (12, 17));

        // Row 13: Arrows and zodiac
        map.insert('↙', (13, 0));
        map.insert('↖', (13, 1));
        map.insert('↕', (13, 2));
        map.insert('↔', (13, 3));
        map.insert('↩', (13, 4));
        map.insert('⤴', (13, 5));
        map.insert('⤵', (13, 6));
        map.insert('🔚', (13, 7));
        map.insert('🔛', (13, 8));
        map.insert('🔜', (13, 9));
        map.insert('♈', (13, 10));
        map.insert('♉', (13, 11));
        map.insert('♊', (13, 12));
        map.insert('♋', (13, 13));
        map.insert('♌', (13, 14));
        map.insert('♍', (13, 15));
        map.insert('♎', (13, 16));
        map.insert('♏', (13, 17));

        // Row 14: More zodiac and symbols
        map.insert('♐', (14, 0));
        map.insert('♑', (14, 1));
        map.insert('♒', (14, 2));
        map.insert('♓', (14, 3));
        map.insert('‼', (14, 4));
        map.insert('⁉', (14, 5));
        map.insert('❗', (14, 6));
        map.insert('〰', (14, 7));
        map.insert('♻', (14, 8));
        map.insert('➰', (14, 9));
        map.insert('➿', (14, 10));
        map.insert('©', (14, 11));
        map.insert('®', (14, 12));
        map.insert('™', (14, 13));
        // Note: Keycap emoji (0️⃣-9️⃣, #️⃣) are multi-codepoint sequences
        // handled separately in KEYCAP_MAP.

        // Row 15: Letter symbols (starting at col 7)
        map.insert('🆑', (15, 7));
        map.insert('🆓', (15, 8));
        map.insert('🆔', (15, 9));
        map.insert('Ⓜ', (15, 10));
        map.insert('🆕', (15, 11));
        map.insert('🆖', (15, 12));
        map.insert('🆗', (15, 13));
        map.insert('🅿', (15, 14));
        map.insert('🈲', (15, 15));
        map.insert('🈴', (15, 16));
        map.insert('🈳', (15, 17));

        // Row 16: Japanese symbols
        map.insert('㊙', (16, 0));
        map.insert('🈵', (16, 1));
        map.insert('💠', (16, 2));

        // Row 17: Flags
        map.insert('🏁', (17, 0));
        map.insert('🚩', (17, 1));

        map
    })
}

/// Build and cache the keycap sequence mapping.
/// Maps multi-codepoint keycap emoji (like "1️⃣") to grid positions.
fn get_keycap_map() -> &'static HashMap<&'static str, (usize, usize)> {
    KEYCAP_MAP.get_or_init(|| {
        let mut map = HashMap::new();

        // Keycap sequences: digit/symbol + U+FE0F + U+20E3
        // Row 14 cols 14-17, Row 15 cols 0-6
        map.insert("#️⃣", (14, 14));
        map.insert("0️⃣", (14, 15));
        map.insert("1️⃣", (14, 16));
        map.insert("2️⃣", (14, 17));
        map.insert("3️⃣", (15, 0));
        map.insert("4️⃣", (15, 1));
        map.insert("5️⃣", (15, 2));
        map.insert("6️⃣", (15, 3));
        map.insert("7️⃣", (15, 4));
        map.insert("8️⃣", (15, 5));
        map.insert("9️⃣", (15, 6));

        map
    })
}

/// Check if text contains any emoji from the supported set.
pub fn contains_emoji(text: &str) -> bool {
    let char_map = get_emoji_map();
    let keycap_map = get_keycap_map();

    // Check for keycap sequences first
    for seq in keycap_map.keys() {
        if text.contains(seq) {
            return true;
        }
    }

    // Check for single-char emoji
    text.chars().any(|c| char_map.contains_key(&c))
}

/// Check if a character is a supported single-char emoji.
pub fn is_emoji(ch: char) -> bool {
    get_emoji_map().contains_key(&ch)
}

/// Parsed segment from emoji-aware text parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum TextSegment {
    /// Regular text (no emoji)
    Text(String),
    /// Single-char emoji
    Emoji(char),
    /// Multi-codepoint keycap sequence (e.g., "1️⃣")
    KeycapEmoji(String),
}

/// Parse text into segments, detecting both single-char and keycap emoji.
pub fn parse_text(text: &str) -> Vec<TextSegment> {
    let char_map = get_emoji_map();
    let keycap_map = get_keycap_map();

    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut chars = text.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        // Check if this starts a keycap sequence
        let remaining = &text[i..];
        let mut found_keycap = false;

        for &seq in keycap_map.keys() {
            if remaining.starts_with(seq) {
                // Found a keycap sequence
                if !current_text.is_empty() {
                    segments.push(TextSegment::Text(std::mem::take(&mut current_text)));
                }
                segments.push(TextSegment::KeycapEmoji(seq.to_string()));

                // Skip the remaining chars in the sequence
                let seq_char_count = seq.chars().count();
                for _ in 1..seq_char_count {
                    chars.next();
                }
                found_keycap = true;
                break;
            }
        }

        if found_keycap {
            continue;
        }

        // Check for single-char emoji
        if char_map.contains_key(&ch) {
            if !current_text.is_empty() {
                segments.push(TextSegment::Text(std::mem::take(&mut current_text)));
            }
            segments.push(TextSegment::Emoji(ch));
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        segments.push(TextSegment::Text(current_text));
    }

    segments
}

/// Get emoji sprite as 1-bit bitmap for the bitmap font rendering path.
/// Returns sprite scaled to the target height (maintaining aspect ratio).
pub fn get_emoji_bitmap(ch: char, target_height: usize) -> Option<EmojiBitmap> {
    let map = get_emoji_map();
    let &(row, col) = map.get(&ch)?;

    get_sprite_at_position(row, col, target_height)
}

/// Get emoji sprite as f32 grayscale buffer for TTF rendering path.
pub fn get_emoji_grayscale(ch: char, target_height: usize) -> Option<EmojiGrayscale> {
    let bitmap = get_emoji_bitmap(ch, target_height)?;
    let data: Vec<f32> = bitmap.data.iter().map(|&p| p as f32).collect();

    Some(EmojiGrayscale {
        width: bitmap.width,
        height: bitmap.height,
        data,
    })
}

/// Get keycap emoji sprite as 1-bit bitmap.
pub fn get_keycap_bitmap(seq: &str, target_height: usize) -> Option<EmojiBitmap> {
    let map = get_keycap_map();
    let &(row, col) = map.get(seq)?;

    get_sprite_at_position(row, col, target_height)
}

/// Get keycap emoji sprite as f32 grayscale buffer.
pub fn get_keycap_grayscale(seq: &str, target_height: usize) -> Option<EmojiGrayscale> {
    let bitmap = get_keycap_bitmap(seq, target_height)?;
    let data: Vec<f32> = bitmap.data.iter().map(|&p| p as f32).collect();

    Some(EmojiGrayscale {
        width: bitmap.width,
        height: bitmap.height,
        data,
    })
}

/// Extract and scale sprite from a specific grid position.
fn get_sprite_at_position(row: usize, col: usize, target_height: usize) -> Option<EmojiBitmap> {
    let pixels = get_sprite_pixels();

    // Calculate pixel position in sprite sheet
    let src_x = col * STRIDE;
    let src_y = row * STRIDE;

    // Extract the 12x12 cell
    let mut cell = [0u8; CELL_SIZE * CELL_SIZE];
    for dy in 0..CELL_SIZE {
        for dx in 0..CELL_SIZE {
            let sx = src_x + dx;
            let sy = src_y + dy;
            if sx < SHEET_WIDTH && sy < SHEET_HEIGHT {
                let src_idx = sy * SHEET_WIDTH + sx;
                let dst_idx = dy * CELL_SIZE + dx;
                cell[dst_idx] = pixels[src_idx];
            }
        }
    }

    // Check if cell has any content
    if !cell.iter().any(|&p| p != 0) {
        return None;
    }

    // Scale to target height (maintain square aspect ratio)
    let scale = target_height as f32 / CELL_SIZE as f32;
    let target_width = target_height; // Keep square

    let mut scaled = vec![0u8; target_width * target_height];

    // Nearest-neighbor scaling
    for ty in 0..target_height {
        for tx in 0..target_width {
            let sx = (tx as f32 / scale) as usize;
            let sy = (ty as f32 / scale) as usize;

            if sx < CELL_SIZE && sy < CELL_SIZE {
                let src_idx = sy * CELL_SIZE + sx;
                let dst_idx = ty * target_width + tx;
                scaled[dst_idx] = cell[src_idx];
            }
        }
    }

    Some(EmojiBitmap {
        width: target_width,
        height: target_height,
        data: scaled,
    })
}

/// List all supported emoji characters.
pub fn supported_emoji() -> Vec<char> {
    let map = get_emoji_map();
    let mut chars: Vec<char> = map.keys().copied().collect();
    chars.sort();
    chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_loads() {
        let pixels = get_sprite_pixels();
        assert_eq!(pixels.len(), SHEET_WIDTH * SHEET_HEIGHT);
        assert!(pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_emoji_map_populated() {
        let map = get_emoji_map();
        assert!(!map.is_empty());
        assert!(map.contains_key(&'😃'));
        assert!(map.contains_key(&'❤'));
    }

    #[test]
    fn test_contains_emoji() {
        assert!(contains_emoji("Hello 😃 World"));
        assert!(contains_emoji("❤"));
        assert!(!contains_emoji("Hello World"));
    }

    #[test]
    fn test_get_emoji_bitmap() {
        let bitmap = get_emoji_bitmap('😃', 24);
        assert!(bitmap.is_some());

        let bitmap = bitmap.unwrap();
        assert_eq!(bitmap.width, 24);
        assert_eq!(bitmap.height, 24);
        assert!(bitmap.data.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_get_emoji_grayscale() {
        let gray = get_emoji_grayscale('❤', 24);
        assert!(gray.is_some());

        let gray = gray.unwrap();
        assert!(gray.width > 0);
        assert!(gray.data.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_unknown_emoji_returns_none() {
        assert!(get_emoji_bitmap('中', 24).is_none());
    }

    #[test]
    fn test_supported_emoji_list() {
        let supported = supported_emoji();
        assert!(!supported.is_empty());
        assert!(supported.contains(&'😃'));
    }

    #[test]
    fn test_keycap_emoji_detection() {
        // Keycap sequences should be detected
        assert!(contains_emoji("Press 1️⃣ to continue"));
        assert!(contains_emoji("Dial #️⃣"));

        // Plain digits should NOT be detected as emoji
        assert!(!contains_emoji("Press 1 to continue"));
        assert!(!contains_emoji("123"));
    }

    #[test]
    fn test_keycap_emoji_bitmap() {
        let bitmap = get_keycap_bitmap("1️⃣", 24);
        assert!(bitmap.is_some());

        let bitmap = bitmap.unwrap();
        assert_eq!(bitmap.width, 24);
        assert_eq!(bitmap.height, 24);
        assert!(bitmap.data.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_parse_text_with_keycap() {
        let segments = parse_text("Press 1️⃣ for help");

        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], TextSegment::Text(s) if s == "Press "));
        assert!(matches!(&segments[1], TextSegment::KeycapEmoji(s) if s == "1️⃣"));
        assert!(matches!(&segments[2], TextSegment::Text(s) if s == " for help"));
    }

    #[test]
    fn test_parse_text_mixed() {
        let segments = parse_text("Call ☎ or press 1️⃣");

        assert_eq!(segments.len(), 4);
        assert!(matches!(&segments[0], TextSegment::Text(s) if s == "Call "));
        assert!(matches!(&segments[1], TextSegment::Emoji('☎')));
        assert!(matches!(&segments[2], TextSegment::Text(s) if s == " or press "));
        assert!(matches!(&segments[3], TextSegment::KeycapEmoji(s) if s == "1️⃣"));
    }

    #[test]
    fn test_plain_digits_not_emoji() {
        let segments = parse_text("Size 1: Hello");

        // Should be a single text segment, no emoji
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], TextSegment::Text(s) if s == "Size 1: Hello"));
    }
}

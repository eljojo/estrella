//! JSON schema types for the document API.

use serde::Deserialize;
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

/// Top-level JSON document.
#[derive(Debug, Deserialize)]
pub struct JsonDocument {
    /// List of components to render.
    pub document: Vec<JsonComponent>,
    /// Whether to cut paper after printing (default: true).
    #[serde(default = "default_true")]
    pub cut: bool,
}

/// A single component in the document.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonComponent {
    Text(JsonText),
    Header(JsonHeader),
    LineItem(JsonLineItem),
    Total(JsonTotal),
    Divider(JsonDivider),
    Spacer(JsonSpacer),
    BlankLine(JsonBlankLine),
    Columns(JsonColumns),
    Markdown(JsonMarkdown),
    QrCode(JsonQrCode),
    Pdf417(JsonPdf417),
    Barcode(JsonBarcode),
    Pattern(JsonPattern),
    NvLogo(JsonNvLogo),
}

/// Text component with full styling support.
#[derive(Debug, Deserialize)]
pub struct JsonText {
    pub content: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub upperline: bool,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub upside_down: bool,
    #[serde(default)]
    pub reduced: bool,
    /// Explicit smoothing control. `null` = auto (enabled for scaled text).
    #[serde(default)]
    pub smoothing: Option<bool>,
    /// Text alignment: "left", "center", "right".
    #[serde(default)]
    pub align: Option<String>,
    /// Shorthand: `"center": true` sets alignment to center.
    #[serde(default)]
    pub center: bool,
    /// Shorthand: `"right": true` sets alignment to right.
    #[serde(default)]
    pub right: bool,
    /// Font: "A", "B", "C".
    #[serde(default)]
    pub font: Option<String>,
    /// Character size multiplier via ESC i (single number for uniform, or [h, w]).
    #[serde(default, deserialize_with = "deserialize_size_or_scale")]
    pub size: Option<[u8; 2]>,
    /// Character scale via ESC W / ESC h (single number for uniform, or [h, w]).
    #[serde(default, deserialize_with = "deserialize_size_or_scale")]
    pub scale: Option<[u8; 2]>,
    #[serde(default)]
    pub double_width: bool,
    #[serde(default)]
    pub double_height: bool,
    /// If true, no trailing newline.
    #[serde(default, rename = "inline")]
    pub is_inline: bool,
}

/// Header component: centered, bold, large text.
#[derive(Debug, Deserialize)]
pub struct JsonHeader {
    pub content: String,
    /// "normal" (default, 2x2) or "small" (1x1).
    #[serde(default)]
    pub variant: Option<String>,
}

/// Line item: name on left, price on right.
#[derive(Debug, Deserialize)]
pub struct JsonLineItem {
    pub name: String,
    pub price: f64,
    #[serde(default)]
    pub width: Option<usize>,
}

/// Total: label + amount, right-aligned by default.
#[derive(Debug, Deserialize)]
pub struct JsonTotal {
    pub amount: f64,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub double_width: bool,
    /// "right" (default) or "left".
    #[serde(default)]
    pub align: Option<String>,
}

/// Horizontal divider.
#[derive(Debug, Deserialize)]
pub struct JsonDivider {
    /// "dashed" (default), "solid", "double", "equals".
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub width: Option<usize>,
}

/// Vertical spacer.
#[derive(Debug, Deserialize)]
pub struct JsonSpacer {
    /// Space in millimeters.
    pub mm: Option<f32>,
    /// Space in lines (~3mm per line).
    pub lines: Option<u8>,
    /// Space in raw 1/4mm units.
    pub units: Option<u8>,
}

/// Empty line.
#[derive(Debug, Deserialize)]
pub struct JsonBlankLine {}

/// Two-column layout.
#[derive(Debug, Deserialize)]
pub struct JsonColumns {
    pub left: String,
    pub right: String,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub invert: bool,
}

/// Markdown content.
#[derive(Debug, Deserialize)]
pub struct JsonMarkdown {
    pub content: String,
    #[serde(default)]
    pub show_urls: bool,
}

/// QR code.
#[derive(Debug, Deserialize)]
pub struct JsonQrCode {
    pub data: String,
    #[serde(default)]
    pub cell_size: Option<u8>,
    /// "L", "M" (default), "Q", "H".
    #[serde(default)]
    pub error_level: Option<String>,
    /// "left", "center" (default), "right".
    #[serde(default)]
    pub align: Option<String>,
}

/// PDF417 2D barcode.
#[derive(Debug, Deserialize)]
pub struct JsonPdf417 {
    pub data: String,
    #[serde(default)]
    pub module_width: Option<u8>,
    #[serde(default)]
    pub ecc_level: Option<u8>,
    /// "left", "center" (default), "right".
    #[serde(default)]
    pub align: Option<String>,
}

/// 1D barcode.
#[derive(Debug, Deserialize)]
pub struct JsonBarcode {
    /// "code39", "code128", "ean13", "upca", "itf".
    pub format: String,
    pub data: String,
    #[serde(default)]
    pub height: Option<u8>,
}

/// Pattern (generative art).
#[derive(Debug, Deserialize)]
pub struct JsonPattern {
    pub name: String,
    #[serde(default)]
    pub height: Option<usize>,
    /// Pattern-specific parameters.
    #[serde(default)]
    pub params: HashMap<String, String>,
    /// Dithering algorithm: "bayer" (default), "floyd-steinberg", "atkinson", "jarvis", "none".
    #[serde(default)]
    pub dither: Option<String>,
}

/// NV (non-volatile) logo stored in printer memory.
#[derive(Debug, Deserialize)]
pub struct JsonNvLogo {
    /// 2-character key identifying the stored logo.
    pub key: String,
    #[serde(default)]
    pub center: bool,
    /// Uniform scale (1 or 2). Overridden by scale_x/scale_y if set.
    #[serde(default)]
    pub scale: Option<u8>,
    #[serde(default)]
    pub scale_x: Option<u8>,
    #[serde(default)]
    pub scale_y: Option<u8>,
}

/// Custom deserializer for size/scale: accepts a single number (uniform) or [h, w] array.
fn deserialize_size_or_scale<'de, D>(deserializer: D) -> Result<Option<[u8; 2]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SizeValue {
        Uniform(u8),
        Pair([u8; 2]),
    }

    let opt: Option<SizeValue> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(SizeValue::Uniform(n)) => Ok(Some([n, n])),
        Some(SizeValue::Pair(pair)) => Ok(Some(pair)),
    }
}

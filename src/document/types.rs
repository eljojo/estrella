//! Component struct types for the unified document model.
//!
//! All types derive `Serialize + Deserialize` so the same types work for
//! both Rust API construction and JSON deserialization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Custom deserializer for optional size/scale: accepts a single number (uniform) or [h, w] array.
pub(crate) fn deserialize_size_or_scale<'de, D>(deserializer: D) -> Result<Option<[u8; 2]>, D::Error>
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

fn default_text_size() -> [u8; 2] {
    [1, 1]
}

/// Custom deserializer for text size: accepts a single number or [h, w] array.
/// Size semantics: 0 = Font B, 1 = Font A (default), N = Font A + ESC i [N-1, N-1].
pub(crate) fn deserialize_text_size<'de, D>(deserializer: D) -> Result<[u8; 2], D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SizeValue {
        Uniform(u8),
        Pair([u8; 2]),
    }

    let v = SizeValue::deserialize(deserializer)?;
    match v {
        SizeValue::Uniform(n) => Ok([n, n]),
        SizeValue::Pair(pair) => Ok(pair),
    }
}

// ============================================================================
// TEXT COMPONENTS
// ============================================================================

/// Text component with full styling support.
///
/// ## Size
///
/// The `size` field controls both font selection and character expansion:
/// - `0` or `[0, 0]`: Font B (small, 64 chars/line)
/// - `1` or `[1, 1]`: Font A (normal, 48 chars/line) — **default**
/// - `N` or `[N, N]`: Font A + ESC i expansion (N-1 multiplier)
/// - `[H, W]`: Font A + ESC i with independent height/width
///
/// Examples: `2` = double size, `3` = triple, `[3, 1]` = triple height / normal width.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
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
    /// Character size: 0 = Font B, 1 = Font A (default), N = Font A + expansion.
    #[serde(default = "default_text_size", deserialize_with = "deserialize_text_size")]
    pub size: [u8; 2],
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

impl Default for Text {
    fn default() -> Self {
        Self {
            content: String::new(),
            bold: false,
            underline: false,
            upperline: false,
            invert: false,
            upside_down: false,
            reduced: false,
            smoothing: None,
            align: None,
            center: false,
            right: false,
            size: [1, 1],
            scale: None,
            double_width: false,
            double_height: false,
            is_inline: false,
        }
    }
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }
}

/// Header component: centered, bold, large text.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Header {
    pub content: String,
    /// "normal" (default, 2x2) or "small" (1x1).
    #[serde(default)]
    pub variant: Option<String>,
}

impl Header {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }
}

/// Border style for Banner and Table components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    #[default]
    Single,
    Double,
    Heavy,
    Shade,
    Shadow,
    /// Single borders with double-line header separator (tables only; banners treat as single).
    Mixed,
}

fn default_banner_size() -> u8 {
    3
}

fn default_banner_bold() -> bool {
    true
}

fn default_banner_padding() -> u8 {
    1
}

/// Framed banner component with auto-sizing.
///
/// Renders text in a box-drawing frame, auto-cascading from the biggest
/// size that fits down to Font B as a fallback.
///
/// ## Size cascade
///
/// Given `size: 3` (default) and content "HELLO":
/// - Try size `[3, 3]` (16 chars/line) — fits? Use it.
/// - Try size `[3, 2]` (24 chars/line) — fits? Use it.
/// - Try size `[3, 1]` (48 chars/line) — fits? Use it.
/// - Fallback: size `[0, 0]` Font B (64 chars/line).
///
/// The height dimension stays at `size` for maximum visual impact;
/// only the width cascades down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Banner {
    pub content: String,
    /// Max size (0–3). The banner picks the largest width that fits. Default: 3.
    #[serde(default = "default_banner_size")]
    pub size: u8,
    /// Border style: "single" (default), "double", "heavy", "shade", or "shadow".
    #[serde(default)]
    pub border: BorderStyle,
    /// Whether the content text is bold. Default: true.
    #[serde(default = "default_banner_bold")]
    pub bold: bool,
    /// Blank lines of padding above and below the content inside the frame. Default: 1.
    #[serde(default = "default_banner_padding")]
    pub padding: u8,
}

impl Default for Banner {
    fn default() -> Self {
        Self {
            content: String::new(),
            size: 3,
            border: BorderStyle::Single,
            bold: true,
            padding: 1,
        }
    }
}

impl Banner {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }
}

impl Interpolatable for Banner {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.content, vars);
    }
}

/// Line item: name on left, price on right.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineItem {
    pub name: String,
    pub price: f64,
    #[serde(default)]
    pub width: Option<usize>,
}

impl LineItem {
    pub fn new(name: impl Into<String>, price: f64) -> Self {
        Self {
            name: name.into(),
            price,
            ..Default::default()
        }
    }
}

/// Total: label + amount, right-aligned by default.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Total {
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

impl Total {
    pub fn new(amount: f64) -> Self {
        Self {
            amount,
            bold: Some(true),
            ..Default::default()
        }
    }

    pub fn labeled(label: impl Into<String>, amount: f64) -> Self {
        Self {
            amount,
            label: Some(label.into()),
            ..Default::default()
        }
    }
}

// ============================================================================
// LAYOUT COMPONENTS
// ============================================================================

/// Divider style options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DividerStyle {
    #[default]
    Dashed,
    Solid,
    Double,
    Equals,
}

/// Horizontal divider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Divider {
    #[serde(default)]
    pub style: DividerStyle,
    #[serde(default)]
    pub width: Option<usize>,
}

impl Default for Divider {
    fn default() -> Self {
        Self {
            style: DividerStyle::Dashed,
            width: None,
        }
    }
}

/// Vertical spacer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Spacer {
    /// Space in millimeters.
    #[serde(default)]
    pub mm: Option<f32>,
    /// Space in lines (~3mm per line).
    #[serde(default)]
    pub lines: Option<u8>,
    /// Space in raw 1/4mm units.
    #[serde(default)]
    pub units: Option<u8>,
}

impl Spacer {
    pub fn mm(mm: f32) -> Self {
        Self {
            mm: Some(mm),
            ..Default::default()
        }
    }

    pub fn lines(lines: u8) -> Self {
        Self {
            lines: Some(lines),
            ..Default::default()
        }
    }
}

/// Empty line.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlankLine {}

/// Two-column layout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Columns {
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

impl Columns {
    pub fn new(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            ..Default::default()
        }
    }
}

/// Column alignment for Table cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Table with box-drawing borders, optional headers, and per-column alignment.
///
/// Columns are auto-sized proportionally to their maximum content width.
///
/// ## Border styles
///
/// - `single` (default): single-line box drawing (┌─┬┐│├┼┤└┴┘)
/// - `double`: double-line box drawing (╔═╦╗║╠╬╣╚╩╝)
/// - `mixed`: single borders + double header separator (╞═╪═╡)
/// - `heavy`: full block character (█)
/// - `shade`: medium shade character (▒)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Optional header row. If present, rendered bold with a separator below.
    #[serde(default)]
    pub headers: Option<Vec<String>>,
    /// Data rows. Each inner Vec is one row of cell values.
    pub rows: Vec<Vec<String>>,
    /// Border style (default: single).
    #[serde(default)]
    pub border: BorderStyle,
    /// Per-column alignment. Columns beyond this list default to left.
    #[serde(default)]
    pub align: Vec<ColumnAlign>,
    /// Draw separator lines between data rows (default: false).
    #[serde(default)]
    pub row_separator: bool,
    /// Override total width in characters (default: 48 for Font A).
    #[serde(default)]
    pub width: Option<usize>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            headers: None,
            rows: Vec::new(),
            border: BorderStyle::Single,
            align: Vec::new(),
            row_separator: false,
            width: None,
        }
    }
}

impl Table {
    pub fn new(rows: Vec<Vec<String>>) -> Self {
        Self {
            rows,
            ..Default::default()
        }
    }
}

// ============================================================================
// CONTENT COMPONENTS
// ============================================================================

/// Markdown content.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Markdown {
    pub content: String,
    #[serde(default)]
    pub show_urls: bool,
}

impl Markdown {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }
}

// ============================================================================
// BARCODE COMPONENTS
// ============================================================================

/// QR code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QrCode {
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

impl QrCode {
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            ..Default::default()
        }
    }
}

/// PDF417 2D barcode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pdf417 {
    pub data: String,
    #[serde(default)]
    pub module_width: Option<u8>,
    #[serde(default)]
    pub ecc_level: Option<u8>,
    /// "left", "center" (default), "right".
    #[serde(default)]
    pub align: Option<String>,
}

impl Pdf417 {
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            ..Default::default()
        }
    }
}

/// 1D barcode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Barcode {
    /// "code39", "code128", "ean13", "upca", "itf".
    #[serde(default)]
    pub format: String,
    pub data: String,
    #[serde(default)]
    pub height: Option<u8>,
}

// ============================================================================
// CHART COMPONENT
// ============================================================================

/// Chart visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartStyle {
    /// Line chart with thick connected lines and filled markers.
    #[default]
    Line,
    /// Area chart: filled region below the data line with line on top.
    Area,
    /// Bar chart: vertical bars for each data point.
    Bar,
    /// Dot chart: scatter plot with filled circles.
    Dot,
}

/// Chart component: renders data as a visual graph image.
///
/// Produces a raster image with axes, labels, grid lines, and data
/// visualization. Designed for thermal printing with thick lines, small
/// bitmap fonts, and high-contrast fills that dither well.
///
/// ## Example (JSON)
///
/// ```json
/// {
///   "type": "chart",
///   "style": "line",
///   "labels": ["09:00", "10:00", "11:00", "12:00"],
///   "values": [-16, -14, -13, -12],
///   "height": 200,
///   "y_suffix": "°C"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    /// Visual style of the chart.
    #[serde(default)]
    pub style: ChartStyle,
    /// X-axis labels (one per data point).
    #[serde(default)]
    pub labels: Vec<String>,
    /// Data values (one per data point).
    pub values: Vec<f64>,
    /// Total chart height in pixels (default: 200).
    #[serde(default)]
    pub height: Option<usize>,
    /// Suffix appended to Y-axis tick labels (e.g., "°C", "%").
    #[serde(default)]
    pub y_suffix: Option<String>,
    /// Prefix prepended to Y-axis tick labels (e.g., "$", "€").
    #[serde(default)]
    pub y_prefix: Option<String>,
    /// Optional title rendered above the chart.
    #[serde(default)]
    pub title: Option<String>,
    /// Dithering algorithm: "bayer" (default), "floyd-steinberg", "atkinson", "jarvis", "none".
    #[serde(default)]
    pub dither: Option<String>,
}

impl Default for Chart {
    fn default() -> Self {
        Self {
            style: ChartStyle::Line,
            labels: Vec::new(),
            values: Vec::new(),
            height: None,
            y_suffix: None,
            y_prefix: None,
            title: None,
            dither: None,
        }
    }
}

// ============================================================================
// GRAPHICS COMPONENTS
// ============================================================================

/// Image from URL (resolved at compile time).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Image {
    pub url: String,
    /// Dithering algorithm: "bayer", "floyd-steinberg", "atkinson", "jarvis".
    #[serde(default)]
    pub dither: Option<String>,
    /// Target width in dots (default: 576).
    #[serde(default)]
    pub width: Option<usize>,
    /// Optional max height constraint.
    #[serde(default)]
    pub height: Option<usize>,
    /// Image alignment when narrower than paper: "left", "center" (default), "right".
    #[serde(default)]
    pub align: Option<String>,
    /// Resolved image data (populated by `Document::resolve()`).
    #[serde(skip)]
    pub resolved_data: Option<ResolvedImage>,
}

/// Resolved image data ready for emit.
#[derive(Debug, Clone)]
pub struct ResolvedImage {
    pub raster_data: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

/// Pattern (generative art).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pattern {
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvLogo {
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

// ============================================================================
// HELPER: parse text fields for variable interpolation
// ============================================================================

/// Fields that support template variable interpolation.
pub trait Interpolatable {
    /// Replace `{{key}}` placeholders with values from the variables map.
    fn interpolate(&mut self, vars: &HashMap<String, String>);
}

fn interpolate_string(s: &mut String, vars: &HashMap<String, String>) {
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        if s.contains(&placeholder) {
            *s = s.replace(&placeholder, value);
        }
    }
}

impl Interpolatable for Text {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.content, vars);
    }
}

impl Interpolatable for Header {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.content, vars);
    }
}

impl Interpolatable for LineItem {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.name, vars);
    }
}

impl Interpolatable for Total {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        if let Some(ref mut label) = self.label {
            interpolate_string(label, vars);
        }
    }
}

impl Interpolatable for Columns {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.left, vars);
        interpolate_string(&mut self.right, vars);
    }
}

impl Interpolatable for Markdown {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.content, vars);
    }
}

impl Interpolatable for QrCode {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.data, vars);
    }
}

impl Interpolatable for Pdf417 {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.data, vars);
    }
}

impl Interpolatable for Barcode {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        interpolate_string(&mut self.data, vars);
    }
}

impl Interpolatable for Table {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        if let Some(ref mut headers) = self.headers {
            for h in headers.iter_mut() {
                interpolate_string(h, vars);
            }
        }
        for row in &mut self.rows {
            for cell in row.iter_mut() {
                interpolate_string(cell, vars);
            }
        }
    }
}

// Types without text content are no-ops
impl Interpolatable for Divider {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}
impl Interpolatable for Spacer {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}
impl Interpolatable for BlankLine {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}
impl Interpolatable for Image {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}
impl Interpolatable for Chart {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        for label in &mut self.labels {
            interpolate_string(label, vars);
        }
        if let Some(ref mut title) = self.title {
            interpolate_string(title, vars);
        }
        if let Some(ref mut suffix) = self.y_suffix {
            interpolate_string(suffix, vars);
        }
        if let Some(ref mut prefix) = self.y_prefix {
            interpolate_string(prefix, vars);
        }
    }
}
impl Interpolatable for Pattern {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}
impl Interpolatable for NvLogo {
    fn interpolate(&mut self, _vars: &HashMap<String, String>) {}
}

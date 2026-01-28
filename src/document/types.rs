//! Component struct types for the unified document model.
//!
//! All types derive `Serialize + Deserialize` so the same types work for
//! both Rust API construction and JSON deserialization.
//!
//! Each component implements [`ComponentMeta`] to declare its display label
//! and editor default. This metadata is used by the web editor and API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::render::composer::BlendMode;

/// Metadata that every component struct must provide.
///
/// The label and editor default live next to each struct definition,
/// so adding a new component type is self-contained — implement this
/// trait and the compiler will guide you to the remaining exhaustive
/// matches in `Component`.
pub trait ComponentMeta: Sized {
    /// Human-readable display label (e.g. "QR Code", "Line Item").
    fn label() -> &'static str;

    /// Sensible starter value for the web editor.
    ///
    /// Distinct from `Default` — editor defaults have example content
    /// so new components are immediately useful, not empty.
    fn editor_default() -> Self;
}

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
    /// Optional custom font: "ibm" for IBM Plex Sans. When set, text renders as raster.
    #[serde(default)]
    pub font: Option<String>,
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
            font: None,
        }
    }
}

impl ComponentMeta for Text {
    fn label() -> &'static str { "Text" }
    fn editor_default() -> Self {
        Self { content: "Hello World".into(), ..Default::default() }
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

impl ComponentMeta for Header {
    fn label() -> &'static str { "Header" }
    fn editor_default() -> Self {
        Self { content: "HEADER".into(), ..Default::default() }
    }
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
    /// Inline rule: `──── TEXT ──────────` (single line, compact).
    Rule,
    /// Section heading: bold text + full-width rule below (2 lines, compact).
    Heading,
    /// Tagged label: `■ TEXT` (single line, most compact).
    Tag,
}

fn default_banner_size() -> u8 {
    3
}

fn default_banner_bold() -> bool {
    true
}

fn default_banner_padding() -> u8 {
    0
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
    /// Blank lines of padding above and below the content inside the frame. Default: 0.
    #[serde(default = "default_banner_padding")]
    pub padding: u8,
    /// Optional custom font: "ibm" for IBM Plex Sans. When set, banner renders as raster.
    #[serde(default)]
    pub font: Option<String>,
}

impl Default for Banner {
    fn default() -> Self {
        Self {
            content: String::new(),
            size: 3,
            border: BorderStyle::Single,
            bold: true,
            padding: 0,
            font: None,
        }
    }
}

impl ComponentMeta for Banner {
    fn label() -> &'static str { "Banner" }
    fn editor_default() -> Self {
        Self { content: "BANNER".into(), size: 2, ..Default::default() }
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

impl ComponentMeta for LineItem {
    fn label() -> &'static str { "Line Item" }
    fn editor_default() -> Self {
        Self { name: "Item".into(), ..Default::default() }
    }
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

impl ComponentMeta for Total {
    fn label() -> &'static str { "Total" }
    fn editor_default() -> Self { Self::default() }
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

impl ComponentMeta for Divider {
    fn label() -> &'static str { "Divider" }
    fn editor_default() -> Self { Self::default() }
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

impl ComponentMeta for Spacer {
    fn label() -> &'static str { "Spacer" }
    fn editor_default() -> Self {
        Self { mm: Some(2.0), ..Default::default() }
    }
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

impl ComponentMeta for BlankLine {
    fn label() -> &'static str { "Blank Line" }
    fn editor_default() -> Self { Self {} }
}

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

impl ComponentMeta for Columns {
    fn label() -> &'static str { "Columns" }
    fn editor_default() -> Self {
        Self { left: "Left".into(), right: "Right".into(), ..Default::default() }
    }
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

impl ComponentMeta for Table {
    fn label() -> &'static str { "Table" }
    fn editor_default() -> Self {
        Self {
            headers: Some(vec!["Col 1".into(), "Col 2".into()]),
            rows: vec![vec!["A".into(), "B".into()]],
            ..Default::default()
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

impl ComponentMeta for Markdown {
    fn label() -> &'static str { "Markdown" }
    fn editor_default() -> Self {
        Self { content: "## Heading\n\nParagraph text.".into(), ..Default::default() }
    }
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

impl ComponentMeta for QrCode {
    fn label() -> &'static str { "QR Code" }
    fn editor_default() -> Self {
        Self { data: "https://example.com".into(), ..Default::default() }
    }
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

impl ComponentMeta for Pdf417 {
    fn label() -> &'static str { "PDF417" }
    fn editor_default() -> Self {
        Self { data: "PDF417-DATA".into(), ..Default::default() }
    }
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

impl ComponentMeta for Barcode {
    fn label() -> &'static str { "Barcode" }
    fn editor_default() -> Self {
        Self { format: "code128".into(), data: "ABC-123".into(), height: Some(60) }
    }
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

impl ComponentMeta for Chart {
    fn label() -> &'static str { "Chart" }
    fn editor_default() -> Self {
        Self {
            style: ChartStyle::Bar,
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![1.0, 2.0, 3.0],
            height: Some(100),
            ..Default::default()
        }
    }
}

// ============================================================================
// GRAPHICS COMPONENTS
// ============================================================================

impl ComponentMeta for Image {
    fn label() -> &'static str { "Image" }
    fn editor_default() -> Self { Self::default() }
}

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

impl ComponentMeta for Pattern {
    fn label() -> &'static str { "Pattern" }
    fn editor_default() -> Self {
        Self { name: "estrella".into(), height: Some(80), ..Default::default() }
    }
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

impl ComponentMeta for NvLogo {
    fn label() -> &'static str { "NV Logo" }
    fn editor_default() -> Self {
        Self { key: "A1".into(), center: true, ..Default::default() }
    }
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
// CANVAS COMPONENT
// ============================================================================

/// Position for absolute placement of canvas elements.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Position {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

fn default_opacity() -> f32 {
    1.0
}

/// A canvas element wrapping any Component with positioning and compositing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    /// The inner component (custom deserialization in mod.rs handles this).
    pub component: super::Component,
    /// Absolute position within the canvas. If absent, element flows top-to-bottom.
    #[serde(default)]
    pub position: Option<Position>,
    /// Blend mode for compositing onto the canvas.
    #[serde(default)]
    pub blend_mode: BlendMode,
    /// Opacity (0.0 = transparent, 1.0 = fully opaque).
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

/// Canvas component: absolute-positioned raster compositing surface.
///
/// Renders elements onto a pixel buffer with blend modes, opacity, and
/// optional dithering. Elements without a `position` flow top-to-bottom.
///
/// ## Example (JSON)
///
/// ```json
/// {
///   "type": "canvas",
///   "height": 200,
///   "elements": [
///     {"type": "pattern", "name": "ripple", "height": 200, "position": {"x": 0, "y": 0}},
///     {"text": "OVERLAY", "bold": true, "center": true, "size": 2, "position": {"x": 0, "y": 80}}
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    /// Canvas width in dots (default: 576).
    #[serde(default)]
    pub width: Option<usize>,
    /// Canvas height in dots. Auto-detected from elements if absent.
    #[serde(default)]
    pub height: Option<usize>,
    /// Dithering: "auto" (default), "none", "bayer", "atkinson", "floyd-steinberg", "jarvis".
    /// "auto" uses Atkinson if any element has continuous-tone content, otherwise None.
    #[serde(default)]
    pub dither: Option<String>,
    /// Elements to composite onto the canvas.
    #[serde(default, deserialize_with = "super::deserialize_canvas_elements")]
    pub elements: Vec<CanvasElement>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            dither: None,
            elements: Vec::new(),
        }
    }
}

impl ComponentMeta for Canvas {
    fn label() -> &'static str { "Canvas" }
    fn editor_default() -> Self {
        Self { height: Some(100), ..Default::default() }
    }
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
impl Interpolatable for Canvas {
    fn interpolate(&mut self, vars: &HashMap<String, String>) {
        for element in &mut self.elements {
            element.component.interpolate(vars);
        }
    }
}

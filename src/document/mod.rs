//! # Unified Document Model
//!
//! A single type hierarchy that is both the Rust API and the JSON API.
//! `Document` is constructible in Rust and deserializable from JSON.
//!
//! ```ignore
//! use estrella::document::*;
//!
//! // Rust construction
//! let doc = Document {
//!     document: vec![
//!         Component::Text(Text::new("Hello")),
//!         Component::Divider(Divider::default()),
//!     ],
//!     cut: true,
//!     ..Default::default()
//! };
//!
//! // JSON deserialization
//! let doc: Document = serde_json::from_str(r#"{"document":[{"type":"text","content":"Hello"}]}"#).unwrap();
//!
//! // Both produce bytes the same way
//! let bytes = doc.build();
//! ```

pub mod types;

mod barcode;
mod graphics;
mod layout;
mod markdown;
mod text;

pub use types::*;

use crate::ir::{Op, Program};
use crate::printer::PrinterConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

/// A printable document: a sequence of components with options.
///
/// This is the unified type for both the Rust API and the JSON API.
/// Construct it in Rust or deserialize it from JSON — the same type works for both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// The components that make up this document.
    pub document: Vec<Component>,
    /// Whether to cut the paper after printing (default: true).
    #[serde(default = "default_true")]
    pub cut: bool,
    /// User-defined variables for `{{template}}` interpolation.
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Whether to interpolate `{{variables}}` in text content (default: true).
    #[serde(default = "default_true")]
    pub interpolate: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            document: Vec::new(),
            cut: true,
            variables: HashMap::new(),
            interpolate: true,
        }
    }
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a component to the document.
    pub fn push(&mut self, component: Component) {
        self.document.push(component);
    }

    /// Compile the document to an IR program.
    ///
    /// This performs template variable interpolation (if enabled),
    /// emits IR ops for each component, and adds Init/Cut ops.
    pub fn compile(&self) -> Program {
        let mut doc = self.clone();

        // Interpolate template variables
        if doc.interpolate {
            let vars = doc.build_variable_map();
            for component in &mut doc.document {
                component.interpolate(&vars);
            }
        }

        let mut ops = vec![Op::Init, Op::SetCodepage(1)];

        for component in &doc.document {
            component.emit(&mut ops);
        }

        if doc.cut {
            ops.push(Op::Cut { partial: true });
        }

        Program { ops }
    }

    /// Compile, optimize, and generate StarPRNT bytes.
    pub fn build(&self) -> Vec<u8> {
        self.build_with_config(&PrinterConfig::TSP650II)
    }

    /// Compile, optimize, and generate bytes with a specific printer config.
    pub fn build_with_config(&self, config: &PrinterConfig) -> Vec<u8> {
        self.compile().optimize().to_bytes_with_config(config)
    }

    /// Build the merged variable map: built-in datetime helpers + user overrides.
    fn build_variable_map(&self) -> HashMap<String, String> {
        let mut vars = builtin_variables();
        // User variables override builtins
        vars.extend(self.variables.clone());
        vars
    }
}

/// The unified component enum.
///
/// Each variant corresponds to a document component type. The `#[serde(tag = "type")]`
/// attribute enables JSON like `{"type": "text", "content": "Hello"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    Text(Text),
    Header(Header),
    Banner(Banner),
    LineItem(LineItem),
    Total(Total),
    Divider(Divider),
    Spacer(Spacer),
    BlankLine(BlankLine),
    Columns(Columns),
    Table(Table),
    Markdown(Markdown),
    QrCode(QrCode),
    Pdf417(Pdf417),
    Barcode(Barcode),
    Image(Image),
    Pattern(Pattern),
    NvLogo(NvLogo),
}

impl Component {
    /// Emit IR ops for this component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        match self {
            Component::Text(c) => c.emit(ops),
            Component::Header(c) => c.emit(ops),
            Component::Banner(c) => c.emit(ops),
            Component::LineItem(c) => c.emit(ops),
            Component::Total(c) => c.emit(ops),
            Component::Divider(c) => c.emit(ops),
            Component::Spacer(c) => c.emit(ops),
            Component::BlankLine(c) => c.emit(ops),
            Component::Columns(c) => c.emit(ops),
            Component::Table(c) => c.emit(ops),
            Component::Markdown(c) => c.emit(ops),
            Component::QrCode(c) => c.emit(ops),
            Component::Pdf417(c) => c.emit(ops),
            Component::Barcode(c) => c.emit(ops),
            Component::Image(c) => c.emit(ops),
            Component::Pattern(c) => c.emit(ops),
            Component::NvLogo(c) => c.emit(ops),
        }
    }

    /// Interpolate template variables in this component's text fields.
    pub fn interpolate(&mut self, vars: &HashMap<String, String>) {
        match self {
            Component::Text(c) => c.interpolate(vars),
            Component::Header(c) => c.interpolate(vars),
            Component::Banner(c) => c.interpolate(vars),
            Component::LineItem(c) => c.interpolate(vars),
            Component::Total(c) => c.interpolate(vars),
            Component::Divider(c) => c.interpolate(vars),
            Component::Spacer(c) => c.interpolate(vars),
            Component::BlankLine(c) => c.interpolate(vars),
            Component::Columns(c) => c.interpolate(vars),
            Component::Table(c) => c.interpolate(vars),
            Component::Markdown(c) => c.interpolate(vars),
            Component::QrCode(c) => c.interpolate(vars),
            Component::Pdf417(c) => c.interpolate(vars),
            Component::Barcode(c) => c.interpolate(vars),
            Component::Image(c) => c.interpolate(vars),
            Component::Pattern(c) => c.interpolate(vars),
            Component::NvLogo(c) => c.interpolate(vars),
        }
    }
}

/// Generate built-in datetime template variables.
fn builtin_variables() -> HashMap<String, String> {
    use chrono::Local;

    let now = Local::now();
    let mut vars = HashMap::new();

    vars.insert("date".into(), now.format("%B %-d, %Y").to_string());       // January 27, 2026
    vars.insert("date_short".into(), now.format("%b %-d").to_string());     // Jan 27
    vars.insert("day".into(), now.format("%A").to_string());                 // Monday
    vars.insert("time".into(), now.format("%H:%M").to_string());             // 09:30
    vars.insert("time_12h".into(), now.format("%-I:%M %p").to_string());     // 9:30 AM
    vars.insert("datetime".into(), now.format("%a, %b %-d %H:%M").to_string()); // Mon, Jan 27 09:30
    vars.insert("year".into(), now.format("%Y").to_string());                // 2026
    vars.insert("iso_date".into(), now.format("%Y-%m-%d").to_string());      // 2026-01-27

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_document() {
        let json = r#"{"document": [{"type": "text", "content": "hi"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        assert!(doc.cut);
        let bytes = doc.build();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_cut_false() {
        let json = r#"{"document": [{"type": "text", "content": "hi"}], "cut": false}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        assert!(!doc.cut);
        let ir = doc.compile();
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::Cut { .. })));
    }

    #[test]
    fn test_text_bold_center() {
        let json = r#"{"document": [{"type": "text", "content": "Hello", "bold": true, "center": true}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetAlign(crate::protocol::text::Alignment::Center))));
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s == "Hello")));
    }

    #[test]
    fn test_text_size_uniform() {
        // size 2 → ESC i [1, 1] (double expansion)
        let json = r#"{"document": [{"type": "text", "content": "x", "size": 2}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetSize { height: 1, width: 1 })));
    }

    #[test]
    fn test_text_size_array() {
        // size [2, 3] → ESC i [1, 2]
        let json = r#"{"document": [{"type": "text", "content": "x", "size": [2, 3]}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetSize { height: 1, width: 2 })));
    }

    #[test]
    fn test_text_size_0_font_b() {
        // size 0 → Font B, no SetSize
        let json = r#"{"document": [{"type": "text", "content": "x", "size": 0}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetFont(crate::protocol::text::Font::B))));
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_text_default_size_font_a() {
        // No size field → default [1, 1] → Font A, no SetSize
        let json = r#"{"document": [{"type": "text", "content": "x"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetFont(crate::protocol::text::Font::A))));
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_text_inline() {
        let json = r#"{"document": [{"type": "text", "content": "x", "inline": true}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        let text_idx = ir.ops.iter().position(|op| matches!(op, Op::Text(s) if s == "x")).unwrap();
        if text_idx + 1 < ir.ops.len() {
            assert!(!matches!(ir.ops[text_idx + 1], Op::Newline));
        }
    }

    #[test]
    fn test_header_normal() {
        let json = r#"{"document": [{"type": "header", "content": "STORE"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetSize { height: 1, width: 1 })));
    }

    #[test]
    fn test_banner_json() {
        let json = r#"{"document": [{"type": "banner", "content": "SALE"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        // Should contain the content text
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("SALE"))));
        // Should have bold (default)
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        // Should have box-drawing characters (top-left corner)
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.starts_with('\u{250C}'))));
    }

    #[test]
    fn test_divider_default() {
        let json = r#"{"document": [{"type": "divider"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        let has_dashes = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with("---")
            } else {
                false
            }
        });
        assert!(has_dashes);
    }

    #[test]
    fn test_spacer_mm() {
        let json = r#"{"document": [{"type": "spacer", "mm": 5.0}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Feed { units: 20 })));
    }

    #[test]
    fn test_qr_code() {
        let json = r#"{"document": [{"type": "qr_code", "data": "https://example.com"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::QrCode { .. })));
    }

    #[test]
    fn test_pattern() {
        let json = r#"{"document": [{"type": "pattern", "name": "ripple", "height": 100}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Raster { width: 576, height: 100, .. })));
    }

    #[test]
    fn test_all_component_types() {
        let json = r#"{
            "document": [
                {"type": "text", "content": "hello"},
                {"type": "header", "content": "TITLE"},
                {"type": "header", "content": "small", "variant": "small"},
                {"type": "banner", "content": "FRAMED"},
                {"type": "banner", "content": "DOUBLE", "border": "double", "size": 2},
                {"type": "banner", "content": "HEAVY", "border": "heavy"},
                {"type": "banner", "content": "SHADE", "border": "shade"},
                {"type": "banner", "content": "SHADOW", "border": "shadow"},
                {"type": "line_item", "name": "item", "price": 1.0},
                {"type": "total", "amount": 1.0},
                {"type": "total", "label": "TAX:", "amount": 0.1, "bold": false},
                {"type": "divider"},
                {"type": "divider", "style": "solid"},
                {"type": "spacer", "mm": 2.0},
                {"type": "spacer", "lines": 1},
                {"type": "spacer", "units": 10},
                {"type": "blank_line"},
                {"type": "columns", "left": "L", "right": "R"},
                {"type": "table", "rows": [["A", "B"], ["C", "D"]]},
                {"type": "markdown", "content": "**bold**"},
                {"type": "qr_code", "data": "test"},
                {"type": "pdf417", "data": "test"},
                {"type": "barcode", "format": "code128", "data": "TEST"},
                {"type": "nv_logo", "key": "A1"}
            ]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.len() > 10);
    }

    #[test]
    fn test_table_json() {
        let json = r#"{"document": [{
            "type": "table",
            "headers": ["Item", "Qty", "Price"],
            "rows": [["Espresso", "2", "$6.00"], ["Croissant", "1", "$3.50"]],
            "border": "mixed",
            "align": ["left", "right", "right"],
            "row_separator": false
        }]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();

        // Should contain header text
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("Item"))));
        // Should contain data
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("Espresso"))));
        // Should have bold for headers
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        // Should have mixed header separator (╞)
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains('\u{255E}'))));
    }

    #[test]
    fn test_table_variable_interpolation() {
        let json = r#"{
            "variables": {"item": "Latte"},
            "document": [{
                "type": "table",
                "headers": ["Item"],
                "rows": [["{{item}}"]]
            }]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("Latte"))));
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("{{item}}"))));
    }

    #[test]
    fn test_variable_interpolation() {
        let json = r#"{
            "variables": {"name": "Jojo", "temp": "6°C"},
            "document": [
                {"type": "text", "content": "Hello {{name}}!"},
                {"type": "columns", "left": "Now", "right": "{{temp}}"}
            ]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("Jojo"))));
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("6°C"))));
    }

    #[test]
    fn test_interpolation_disabled() {
        let json = r#"{
            "variables": {"name": "Jojo"},
            "interpolate": false,
            "document": [
                {"type": "text", "content": "Hello {{name}}!"}
            ]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        // Should keep the literal {{name}}
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("{{name}}"))));
    }

    #[test]
    fn test_builtin_variables() {
        let vars = builtin_variables();
        assert!(vars.contains_key("date"));
        assert!(vars.contains_key("day"));
        assert!(vars.contains_key("time"));
        assert!(vars.contains_key("year"));
        assert!(vars.contains_key("iso_date"));
        assert!(vars.contains_key("datetime"));
        assert!(vars.contains_key("date_short"));
        assert!(vars.contains_key("time_12h"));
    }

    #[test]
    fn test_builtin_date_interpolation() {
        let json = r#"{
            "document": [
                {"type": "text", "content": "Year: {{year}}"}
            ]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        // Should have interpolated {{year}} with current year
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.starts_with("Year: 20"))));
        // Should NOT contain the template placeholder
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("{{year}}"))));
    }

    #[test]
    fn test_user_variable_overrides_builtin() {
        let json = r#"{
            "variables": {"year": "CUSTOM"},
            "document": [
                {"type": "text", "content": "Year: {{year}}"}
            ]
        }"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(s) if s.contains("CUSTOM"))));
    }

    #[test]
    fn test_rust_api_construction() {
        let doc = Document {
            document: vec![
                Component::Header(Header::new("CHURRA MART")),
                Component::Divider(Divider::default()),
                Component::LineItem(LineItem::new("Espresso", 4.50)),
                Component::Total(Total::new(4.50)),
                Component::QrCode(QrCode::new("https://example.com")),
            ],
            cut: true,
            ..Default::default()
        };
        let bytes = doc.build();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let doc = Document {
            document: vec![
                Component::Text(Text::new("Hello")),
                Component::Divider(Divider::default()),
            ],
            cut: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&doc).unwrap();
        let doc2: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc2.document.len(), 2);
        assert!(doc2.cut);
    }
}

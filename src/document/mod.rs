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
pub mod canvas;
mod graphics;
mod layout;
mod markdown;
pub mod resolve;
mod text;

pub use resolve::{ImageResolver, fetch_image, fetch_image_with_ctx};
pub use types::*;

use crate::ir::{Op, Program};
use crate::printer::PrinterConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// EMIT CONTEXT
// ============================================================================

/// Context passed to component `emit()` methods, carrying the ops buffer
/// and the target print width. Eliminates hardcoded `576` and `48` throughout
/// the document system.
pub struct EmitContext {
    /// The IR ops buffer being built.
    pub ops: Vec<Op>,
    /// Target print width in dots (e.g. 576 for TSP650II, 1200 for a canvas).
    pub print_width: usize,
}

impl EmitContext {
    /// Create a new context with the given print width.
    pub fn new(print_width: usize) -> Self {
        Self {
            ops: Vec::new(),
            print_width,
        }
    }

    /// Push a single op.
    pub fn push(&mut self, op: Op) {
        self.ops.push(op);
    }

    /// Extend with multiple ops.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = Op>) {
        self.ops.extend(ops);
    }

    /// Characters per line with Font A (12 dots/char).
    pub fn chars_per_line(&self) -> usize {
        self.print_width / 12
    }

    /// Characters per line with Font B (9 dots/char).
    pub fn chars_per_line_b(&self) -> usize {
        self.print_width / 9
    }
}

fn default_true() -> bool {
    true
}

// ============================================================================
// SHORTHAND DESERIALIZATION
// ============================================================================

/// Shorthand keys: (shorthand_key, type_name, target_field).
///
/// When a component JSON object has no `"type"` field, these shorthands are
/// checked in order. The shorthand key's value is moved to `target_field`,
/// and `"type"` is set to `type_name`.
///
/// Example: `{"text": "hello", "bold": true}` → `{"type": "text", "content": "hello", "bold": true}`
const SHORTHANDS: &[(&str, &str, &str)] = &[
    ("text", "text", "content"),
    ("banner", "banner", "content"),
    ("line_item", "line_item", "name"),
    ("total", "total", "amount"),
    ("divider", "divider", "style"),
    ("spacer_mm", "spacer", "mm"),
    ("image", "image", "url"),
    ("qr_code", "qr_code", "data"),
    ("markdown", "markdown", "content"),
    ("canvas", "canvas", "elements"),
];

/// Rewrite a shorthand JSON object to canonical `{"type": ...}` form.
/// Only called when the map has no `"type"` key.
fn normalize_shorthand(map: &mut serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    for &(key, type_name, field) in SHORTHANDS {
        if let Some(val) = map.remove(key) {
            map.insert("type".into(), serde_json::Value::String(type_name.into()));
            map.insert(field.into(), val);
            return Ok(());
        }
    }
    Err(format!(
        "component object has no 'type' field and no shorthand key ({})",
        SHORTHANDS
            .iter()
            .map(|(k, _, _)| *k)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Deserialize a `Vec<Component>` with shorthand support.
///
/// Each element is first parsed as raw JSON. If it lacks a `"type"` field,
/// shorthand normalization rewrites it to canonical form before passing it
/// to `Component`'s derived deserializer.
fn deserialize_components<'de, D>(deserializer: D) -> Result<Vec<Component>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let mut obj = match v {
                serde_json::Value::Object(map) => map,
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "document[{}]: expected object, got {}",
                        i, other
                    )));
                }
            };

            if !obj.contains_key("type") {
                normalize_shorthand(&mut obj)
                    .map_err(|e| serde::de::Error::custom(format!("document[{}]: {}", i, e)))?;
            }

            serde_json::from_value(serde_json::Value::Object(obj))
                .map_err(|e| serde::de::Error::custom(format!("document[{}]: {}", i, e)))
        })
        .collect()
}

/// Deserialize a `Vec<CanvasElement>` with shorthand support for the inner component.
///
/// Each element is first parsed as raw JSON. Canvas-specific keys (`position`,
/// `blend_mode`, `opacity`) are extracted, then the remaining object is
/// deserialized as a `Component` (with shorthand normalization).
fn deserialize_canvas_elements<'de, D>(deserializer: D) -> Result<Vec<CanvasElement>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use crate::render::composer::BlendMode;

    let values: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let mut obj = match v {
                serde_json::Value::Object(map) => map,
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "canvas.elements[{}]: expected object, got {}",
                        i, other
                    )));
                }
            };

            // Extract canvas-element-specific fields before Component deser
            let position: Option<Position> = obj
                .remove("position")
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .transpose()?;

            let blend_mode: BlendMode = obj
                .remove("blend_mode")
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .transpose()?
                .unwrap_or_default();

            let opacity: f32 = obj
                .remove("opacity")
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .transpose()?
                .unwrap_or(1.0);

            // Normalize shorthand if no "type" key
            if !obj.contains_key("type") {
                normalize_shorthand(&mut obj).map_err(|e| {
                    serde::de::Error::custom(format!("canvas.elements[{}]: {}", i, e))
                })?;
            }

            let component: Component = serde_json::from_value(serde_json::Value::Object(obj))
                .map_err(|e| serde::de::Error::custom(format!("canvas.elements[{}]: {}", i, e)))?;

            Ok(CanvasElement {
                component,
                position,
                blend_mode,
                opacity,
            })
        })
        .collect()
}

/// A printable document: a sequence of components with options.
///
/// This is the unified type for both the Rust API and the JSON API.
/// Construct it in Rust or deserialize it from JSON — the same type works for both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// The components that make up this document.
    ///
    /// Supports shorthand syntax: `{"text": "hi"}` instead of `{"type": "text", "content": "hi"}`.
    #[serde(deserialize_with = "deserialize_components")]
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
    /// Print entire document as a raster image instead of text commands.
    /// Renders everything through the bitmap preview engine first, then sends
    /// the result as a single raster image. Experimental.
    #[serde(default)]
    pub raster: bool,
    /// Target print width in dots. Default: 576 (TSP650II).
    /// Set this to render at a different width (e.g. 1200 for a virtual canvas).
    #[serde(default)]
    pub width: Option<usize>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            document: Vec::new(),
            cut: true,
            variables: HashMap::new(),
            interpolate: true,
            raster: false,
            width: None,
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

    /// Compile the document to an optimized IR program.
    ///
    /// This performs template variable interpolation (if enabled),
    /// emits IR ops for each component, adds Init/Cut ops, and
    /// runs the optimizer (word-wrapping, redundancy elimination, etc.).
    pub fn compile(&self) -> Program {
        let mut doc = self.clone();

        // Interpolate template variables
        if doc.interpolate {
            let vars = doc.build_variable_map();
            for component in &mut doc.document {
                component.interpolate(&vars);
            }
        }

        let print_width = doc.width.unwrap_or(576);
        let mut ctx = EmitContext::new(print_width);
        ctx.push(Op::Init);
        ctx.push(Op::SetCodepage(1));

        for component in &doc.document {
            component.emit(&mut ctx);
        }

        if doc.cut {
            ctx.push(Op::Cut { partial: true });
        }

        let program = Program { ops: ctx.ops };
        program.optimize()
    }

    /// Compile and generate StarPRNT bytes.
    pub fn build(&self) -> Vec<u8> {
        self.build_with_config(&PrinterConfig::TSP650II)
    }

    /// Compile and generate bytes with a specific printer config.
    ///
    /// When `raster` is true, renders the entire document through the bitmap
    /// preview engine and sends it as a single raster image.
    pub fn build_with_config(&self, config: &PrinterConfig) -> Vec<u8> {
        if self.raster {
            let program = self.compile();
            let raw = crate::preview::render_raw(&program).expect("raster render failed");
            let mut raster_program = Program::new();
            raster_program.push(Op::Init);
            raster_program.push(Op::Raster {
                width: raw.width as u16,
                height: raw.height as u16,
                data: raw.data,
            });
            if self.cut {
                raster_program.push(Op::Feed { units: 24 });
                raster_program.push(Op::Cut { partial: true });
            }
            raster_program.to_bytes_with_config(config)
        } else {
            self.compile().to_bytes_with_config(config)
        }
    }

    /// Build the merged variable map: built-in datetime helpers + user overrides.
    fn build_variable_map(&self) -> HashMap<String, String> {
        let mut vars = builtin_variables();
        // User variables override builtins
        vars.extend(self.variables.clone());
        vars
    }
}

/// Define the Component enum and all dispatch methods from a single list.
///
/// Adding a new component: add one line here, then define the struct in
/// `types.rs` with `impl ComponentMeta`. That's it.
macro_rules! define_components {
    ($($variant:ident($inner:ty)),+ $(,)?) => {
        /// The unified component enum.
        ///
        /// Each variant corresponds to a document component type. The `#[serde(tag = "type")]`
        /// attribute enables JSON like `{"type": "text", "content": "Hello"}`.
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum Component {
            $($variant($inner),)+
        }

        impl Component {
            /// Emit IR ops for this component.
            pub fn emit(&self, ctx: &mut EmitContext) {
                match self { $(Component::$variant(c) => c.emit(ctx),)+ }
            }

            /// Interpolate template variables in this component's text fields.
            pub fn interpolate(&mut self, vars: &HashMap<String, String>) {
                match self { $(Component::$variant(c) => c.interpolate(vars),)+ }
            }

            /// Human-readable display label (from [`ComponentMeta::label`]).
            pub fn label(&self) -> &'static str {
                match self { $(Component::$variant(_) => <$inner>::label(),)+ }
            }

            /// Editor defaults for every component type (from [`ComponentMeta::editor_default`]).
            ///
            /// Single source of truth — [`component_types`] and [`default_component`]
            /// both derive from this.
            pub fn all_editor_defaults() -> Vec<Self> {
                vec![$(Component::$variant(<$inner>::editor_default()),)+]
            }
        }
    };
}

define_components! {
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
    Chart(Chart),
    Canvas(Canvas),
}

/// Generate built-in datetime template variables.
fn builtin_variables() -> HashMap<String, String> {
    use chrono::Local;

    let now = Local::now();
    let mut vars = HashMap::new();

    vars.insert("date".into(), now.format("%B %-d, %Y").to_string()); // January 27, 2026
    vars.insert("date_short".into(), now.format("%b %-d").to_string()); // Jan 27
    vars.insert("day".into(), now.format("%A").to_string()); // Monday
    vars.insert("time".into(), now.format("%H:%M").to_string()); // 09:30
    vars.insert("time_12h".into(), now.format("%-I:%M %p").to_string()); // 9:30 AM
    vars.insert(
        "datetime".into(),
        now.format("%a, %b %-d %H:%M").to_string(),
    ); // Mon, Jan 27 09:30
    vars.insert("year".into(), now.format("%Y").to_string()); // 2026
    vars.insert("iso_date".into(), now.format("%Y-%m-%d").to_string()); // 2026-01-27

    vars
}

/// Component type metadata for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentTypeMeta {
    #[serde(rename = "type")]
    pub type_name: String,
    pub label: String,
}

/// Extract the serde type tag from a Component (the `"type"` field).
fn serde_type_name(comp: &Component) -> String {
    serde_json::to_value(comp).unwrap()["type"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Component type metadata for the frontend.
///
/// Derived from [`Component::all_editor_defaults`] — type names come from
/// serde serialization, labels from [`Component::label`]. Both are
/// exhaustive matches on the enum, so the compiler catches new variants.
pub fn component_types() -> Vec<ComponentTypeMeta> {
    Component::all_editor_defaults()
        .iter()
        .map(|c| ComponentTypeMeta {
            type_name: serde_type_name(c),
            label: c.label().to_string(),
        })
        .collect()
}

/// Create a component with sensible editor defaults by type name.
///
/// Returns `None` for unknown type names. These defaults are tuned for the
/// web editor — each component is immediately useful when added, not empty.
pub fn default_component(type_name: &str) -> Option<Component> {
    Component::all_editor_defaults()
        .into_iter()
        .find(|c| serde_type_name(c) == type_name)
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
        let json =
            r#"{"document": [{"type": "text", "content": "Hello", "bold": true, "center": true}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::SetAlign(crate::protocol::text::Alignment::Center)))
        );
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s == "Hello"))
        );
    }

    #[test]
    fn test_text_size_uniform() {
        // size 2 → ESC i [1, 1] (double expansion)
        let json = r#"{"document": [{"type": "text", "content": "x", "size": 2}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::SetSize {
                height: 1,
                width: 1
            }
        )));
    }

    #[test]
    fn test_text_size_array() {
        // size [2, 3] → ESC i [1, 2]
        let json = r#"{"document": [{"type": "text", "content": "x", "size": [2, 3]}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::SetSize {
                height: 1,
                width: 2
            }
        )));
    }

    #[test]
    fn test_text_size_0_font_b() {
        // size 0 → Font B, no SetSize
        let json = r#"{"document": [{"type": "text", "content": "x", "size": 0}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::SetFont(crate::protocol::text::Font::B)))
        );
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_text_default_size_font_a() {
        // No size field → default [1, 1] → Font A, no SetSize
        // Font A is default after Init, so optimizer removes the redundant SetFont(Font::A)
        let json = r#"{"document": [{"type": "text", "content": "x"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s == "x"))
        );
    }

    #[test]
    fn test_text_inline() {
        let json = r#"{"document": [{"type": "text", "content": "x", "inline": true}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        let text_idx = ir
            .ops
            .iter()
            .position(|op| matches!(op, Op::Text(s) if s == "x"))
            .unwrap();
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
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::SetSize {
                height: 1,
                width: 1
            }
        )));
    }

    #[test]
    fn test_banner_json() {
        let json = r#"{"document": [{"type": "banner", "content": "SALE"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        // Should contain the content text
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("SALE")))
        );
        // Should have bold (default)
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        // Should have box-drawing characters (top-left corner)
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.starts_with('\u{250C}')))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("Item")))
        );
        // Should contain data
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("Espresso")))
        );
        // Should have bold for headers
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        // Should have mixed header separator (╞)
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains('\u{255E}')))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("Latte")))
        );
        assert!(
            !ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("{{item}}")))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("Jojo")))
        );
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("6°C")))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("{{name}}")))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.starts_with("Year: 20")))
        );
        // Should NOT contain the template placeholder
        assert!(
            !ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("{{year}}")))
        );
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
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("CUSTOM")))
        );
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

    // ========================================================================
    // SHORTHAND DESERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_shorthand_text() {
        let json = r#"{"document": [{"text": "hello"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s == "hello"))
        );
    }

    #[test]
    fn test_shorthand_text_with_options() {
        let json = r#"{"document": [{"text": "hi", "bold": true, "size": 2}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s == "hi"))
        );
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::SetSize {
                height: 1,
                width: 1
            }
        )));
    }

    #[test]
    fn test_shorthand_banner() {
        let json = r#"{"document": [{"banner": "SALE", "border": "heavy"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("SALE")))
        );
    }

    #[test]
    fn test_shorthand_line_item() {
        let json = r#"{"document": [{"line_item": "Coffee", "price": 4.50}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("Coffee") && s.contains("4.50")))
        );
    }

    #[test]
    fn test_shorthand_total() {
        let json = r#"{"document": [{"total": 9.99}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("9.99")))
        );
    }

    #[test]
    fn test_shorthand_divider() {
        let json = r#"{"document": [{"divider": "double"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s.contains("═")))
        );
    }

    #[test]
    fn test_shorthand_spacer_mm() {
        let json = r#"{"document": [{"spacer_mm": 5.0}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Feed { units: 20 })));
    }

    #[test]
    fn test_shorthand_image() {
        let json = r#"{"document": [{"image": "https://example.com/photo.jpg"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        assert_eq!(doc.document.len(), 1);
        assert!(
            matches!(&doc.document[0], Component::Image(img) if img.url == "https://example.com/photo.jpg")
        );
    }

    #[test]
    fn test_shorthand_qr_code() {
        let json = r#"{"document": [{"qr_code": "https://example.com"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::QrCode { .. })));
    }

    #[test]
    fn test_shorthand_markdown() {
        let json = r#"{"document": [{"markdown": "**bold**"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
    }

    #[test]
    fn test_shorthand_ignored_when_type_present() {
        // "type" takes precedence; "text" key is just an unknown field (ignored by serde)
        let json = r#"{"document": [{"type": "text", "content": "real", "text": "ignored"}]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        let ir = doc.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| matches!(op, Op::Text(s) if s == "real"))
        );
    }

    #[test]
    fn test_shorthand_mixed_with_canonical() {
        let json = r#"{"document": [
            {"text": "shorthand"},
            {"type": "text", "content": "canonical"},
            {"banner": "HELLO"},
            {"type": "divider"}
        ]}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        assert_eq!(doc.document.len(), 4);
    }

    #[test]
    fn test_editor_defaults_complete() {
        let types = component_types();
        let defaults = Component::all_editor_defaults();

        // Same count
        assert_eq!(types.len(), defaults.len());

        // All type names are unique
        let mut seen = std::collections::HashSet::new();
        for meta in &types {
            assert!(
                seen.insert(&meta.type_name),
                "Duplicate type: {}",
                meta.type_name
            );
        }

        // Every type name round-trips through default_component
        for meta in &types {
            let comp = default_component(&meta.type_name);
            assert!(comp.is_some(), "No default for type: {}", meta.type_name);

            // Serialized type tag matches
            let json = serde_json::to_value(comp.unwrap()).unwrap();
            assert_eq!(json["type"].as_str().unwrap(), meta.type_name);
        }
    }
}

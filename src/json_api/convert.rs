//! Conversion from JSON schema types to Estrella components.

use std::fmt;

use crate::components::*;
use crate::protocol::text::{Alignment, Font};
use crate::render::dither::DitheringAlgorithm;

use super::schema::*;

/// Errors from JSON → Component conversion.
#[derive(Debug)]
pub enum JsonApiError {
    /// A field value is invalid.
    InvalidField {
        component: &'static str,
        field: &'static str,
        message: String,
    },
    /// A required field is missing.
    MissingField {
        component: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for JsonApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonApiError::InvalidField {
                component,
                field,
                message,
            } => write!(f, "{}: invalid {}: {}", component, field, message),
            JsonApiError::MissingField { component, field } => {
                write!(f, "{}: missing required field '{}'", component, field)
            }
        }
    }
}

impl std::error::Error for JsonApiError {}

impl JsonDocument {
    /// Convert this JSON document to a Receipt component.
    pub fn to_receipt(&self) -> Result<Receipt, JsonApiError> {
        let mut receipt = Receipt::new();
        for component in &self.document {
            receipt = receipt.child(component.to_boxed()?);
        }
        if self.cut {
            receipt = receipt.cut();
        }
        Ok(receipt)
    }
}

impl JsonComponent {
    /// Convert to a boxed Component trait object.
    fn to_boxed(&self) -> Result<Box<dyn Component>, JsonApiError> {
        match self {
            JsonComponent::Text(t) => Ok(Box::new(t.to_component()?)),
            JsonComponent::Header(h) => Ok(Box::new(h.to_component()?)),
            JsonComponent::LineItem(li) => Ok(Box::new(li.to_component()?)),
            JsonComponent::Total(t) => Ok(Box::new(t.to_component()?)),
            JsonComponent::Divider(d) => Ok(Box::new(d.to_component()?)),
            JsonComponent::Spacer(s) => Ok(Box::new(s.to_component()?)),
            JsonComponent::BlankLine(_) => Ok(Box::new(BlankLine::new())),
            JsonComponent::Columns(c) => Ok(Box::new(c.to_component()?)),
            JsonComponent::Markdown(m) => Ok(Box::new(m.to_component()?)),
            JsonComponent::QrCode(q) => Ok(Box::new(q.to_component()?)),
            JsonComponent::Pdf417(p) => Ok(Box::new(p.to_component()?)),
            JsonComponent::Barcode(b) => Ok(Box::new(b.to_component()?)),
            JsonComponent::Pattern(p) => Ok(Box::new(p.to_component()?)),
            JsonComponent::NvLogo(n) => Ok(Box::new(n.to_component()?)),
        }
    }
}

// ============ Component Conversions ============

impl JsonText {
    fn to_component(&self) -> Result<Text, JsonApiError> {
        let mut text = if self.is_inline {
            Text::inline(&self.content)
        } else {
            Text::new(&self.content)
        };

        if self.bold {
            text = text.bold();
        }
        if self.underline {
            text = text.underline();
        }
        if self.upperline {
            text = text.upperline();
        }
        if self.invert {
            text = text.invert();
        }
        if self.upside_down {
            text = text.upside_down();
        }
        if self.reduced {
            text = text.reduced();
        }
        if self.double_width {
            text = text.double_width();
        }
        if self.double_height {
            text = text.double_height();
        }

        match self.smoothing {
            Some(true) => text = text.smoothing(),
            Some(false) => text = text.no_smoothing(),
            None => {}
        }

        // Alignment: explicit "align" field takes precedence over shorthand bools
        if let Some(ref align) = self.align {
            text = match align.as_str() {
                "center" => text.center(),
                "right" => text.right(),
                "left" => text.left(),
                other => {
                    return Err(JsonApiError::InvalidField {
                        component: "text",
                        field: "align",
                        message: format!(
                            "expected \"left\", \"center\", or \"right\", got \"{}\"",
                            other
                        ),
                    })
                }
            };
        } else if self.center {
            text = text.center();
        } else if self.right {
            text = text.right();
        }

        if let Some(ref font_str) = self.font {
            text = match font_str.to_uppercase().as_str() {
                "A" => text.font(Font::A),
                "B" => text.font(Font::B),
                "C" => text.font(Font::C),
                other => {
                    return Err(JsonApiError::InvalidField {
                        component: "text",
                        field: "font",
                        message: format!("expected \"A\", \"B\", or \"C\", got \"{}\"", other),
                    })
                }
            };
        }

        if let Some([h, w]) = self.size {
            text = text.size(h, w);
        }
        if let Some([h, w]) = self.scale {
            text = text.scale(h, w);
        }

        Ok(text)
    }
}

impl JsonHeader {
    fn to_component(&self) -> Result<Header, JsonApiError> {
        let variant = self.variant.as_deref().unwrap_or("normal");
        match variant {
            "normal" => Ok(Header::new(&self.content)),
            "small" => Ok(Header::small(&self.content)),
            other => Err(JsonApiError::InvalidField {
                component: "header",
                field: "variant",
                message: format!("expected \"normal\" or \"small\", got \"{}\"", other),
            }),
        }
    }
}

impl JsonLineItem {
    fn to_component(&self) -> Result<LineItem, JsonApiError> {
        let mut item = LineItem::new(&self.name, self.price);
        if let Some(w) = self.width {
            item = item.width(w);
        }
        Ok(item)
    }
}

impl JsonTotal {
    fn to_component(&self) -> Result<Total, JsonApiError> {
        let mut total = if let Some(ref label) = self.label {
            Total::labeled(label, self.amount)
        } else {
            Total::new(self.amount)
        };

        // bold: explicit true/false overrides. Default depends on constructor:
        // Total::new() defaults bold=true, Total::labeled() defaults bold=false.
        // If the user specifies bold explicitly, apply it.
        match self.bold {
            Some(true) => total = total.bold(),
            Some(false) => total = total.not_bold(),
            None => {} // keep constructor default
        }

        if self.double_width {
            total = total.double_width();
        }

        if let Some(ref align) = self.align {
            if align == "left" {
                total = total.left();
            }
        }

        Ok(total)
    }
}

impl JsonDivider {
    fn to_component(&self) -> Result<Divider, JsonApiError> {
        let style_str = self.style.as_deref().unwrap_or("dashed");
        let mut divider = match style_str {
            "dashed" => Divider::dashed(),
            "solid" => Divider::solid(),
            "double" => Divider::double(),
            "equals" => Divider::equals(),
            other => {
                return Err(JsonApiError::InvalidField {
                    component: "divider",
                    field: "style",
                    message: format!(
                        "expected \"dashed\", \"solid\", \"double\", or \"equals\", got \"{}\"",
                        other
                    ),
                })
            }
        };
        if let Some(w) = self.width {
            divider = divider.width(w);
        }
        Ok(divider)
    }
}

impl JsonSpacer {
    fn to_component(&self) -> Result<Spacer, JsonApiError> {
        if let Some(mm) = self.mm {
            Ok(Spacer::mm(mm))
        } else if let Some(lines) = self.lines {
            Ok(Spacer::lines(lines))
        } else if let Some(units) = self.units {
            Ok(Spacer::units(units))
        } else {
            Err(JsonApiError::MissingField {
                component: "spacer",
                field: "mm, lines, or units",
            })
        }
    }
}

impl JsonColumns {
    fn to_component(&self) -> Result<Columns, JsonApiError> {
        let mut cols = Columns::new(&self.left, &self.right);
        if let Some(w) = self.width {
            cols = cols.width(w);
        }
        if self.bold {
            cols = cols.bold();
        }
        if self.underline {
            cols = cols.underline();
        }
        if self.invert {
            cols = cols.invert();
        }
        Ok(cols)
    }
}

impl JsonMarkdown {
    fn to_component(&self) -> Result<Markdown, JsonApiError> {
        let mut md = Markdown::new(&self.content);
        if self.show_urls {
            md = md.show_urls();
        }
        Ok(md)
    }
}

impl JsonQrCode {
    fn to_component(&self) -> Result<QrCode, JsonApiError> {
        let mut qr = QrCode::new(&self.data);
        if let Some(size) = self.cell_size {
            qr = qr.cell_size(size);
        }
        if let Some(ref level) = self.error_level {
            qr = match level.to_uppercase().as_str() {
                "L" => qr.error_level_low(),
                "M" => qr.error_level_medium(),
                "Q" => qr.error_level_quartile(),
                "H" => qr.error_level_high(),
                other => {
                    return Err(JsonApiError::InvalidField {
                        component: "qr_code",
                        field: "error_level",
                        message: format!(
                            "expected \"L\", \"M\", \"Q\", or \"H\", got \"{}\"",
                            other
                        ),
                    })
                }
            };
        }
        if let Some(ref align) = self.align {
            qr = parse_alignment_onto(qr, "qr_code", align, |q, a| match a {
                Alignment::Left => q.left(),
                Alignment::Center => q.center(),
                Alignment::Right => q.right(),
            })?;
        }
        Ok(qr)
    }
}

impl JsonPdf417 {
    fn to_component(&self) -> Result<Pdf417, JsonApiError> {
        let mut pdf = Pdf417::new(&self.data);
        if let Some(w) = self.module_width {
            pdf = pdf.module_width(w);
        }
        if let Some(l) = self.ecc_level {
            pdf = pdf.ecc_level(l);
        }
        if let Some(ref align) = self.align {
            pdf = parse_alignment_onto(pdf, "pdf417", align, |p, a| match a {
                Alignment::Left => p.left(),
                Alignment::Center => p.center(),
                Alignment::Right => p.right(),
            })?;
        }
        Ok(pdf)
    }
}

impl JsonBarcode {
    fn to_component(&self) -> Result<Barcode, JsonApiError> {
        let mut barcode = match self.format.to_lowercase().as_str() {
            "code39" => Barcode::code39(&self.data),
            "code128" => Barcode::code128(&self.data),
            "ean13" => Barcode::ean13(&self.data),
            "upca" => Barcode::upca(&self.data),
            "itf" => Barcode::itf(&self.data),
            other => {
                return Err(JsonApiError::InvalidField {
                    component: "barcode",
                    field: "format",
                    message: format!(
                        "expected \"code39\", \"code128\", \"ean13\", \"upca\", or \"itf\", got \"{}\"",
                        other
                    ),
                })
            }
        };
        if let Some(h) = self.height {
            barcode = barcode.height(h);
        }
        Ok(barcode)
    }
}

impl JsonPattern {
    fn to_component(&self) -> Result<Pattern, JsonApiError> {
        // Look up pattern by name to validate and apply params
        let mut pattern_impl =
            crate::render::patterns::by_name(&self.name).ok_or(JsonApiError::InvalidField {
                component: "pattern",
                field: "name",
                message: format!("unknown pattern \"{}\"", self.name),
            })?;

        // Apply custom params
        for (key, value) in &self.params {
            pattern_impl.set_param(key, value).map_err(|e| JsonApiError::InvalidField {
                component: "pattern",
                field: "params",
                message: e,
            })?;
        }

        let height = self.height.unwrap_or(500);
        let mut component = Pattern::from_impl(pattern_impl, height);

        // Parse dithering algorithm
        if let Some(ref dither_str) = self.dither {
            let algo = parse_dither_algorithm(dither_str).ok_or(JsonApiError::InvalidField {
                component: "pattern",
                field: "dither",
                message: format!(
                    "expected \"bayer\", \"floyd-steinberg\", \"atkinson\", \"jarvis\", or \"none\", got \"{}\"",
                    dither_str
                ),
            })?;
            component = component.dithering(algo);
        }

        Ok(component)
    }
}

impl JsonNvLogo {
    fn to_component(&self) -> Result<NvLogo, JsonApiError> {
        let mut logo = NvLogo::new(&self.key);
        if self.center {
            logo = logo.center();
        }
        // scale_x/scale_y take precedence over uniform scale
        if let Some(sx) = self.scale_x {
            logo = logo.scale_x(sx);
        } else if let Some(s) = self.scale {
            logo = logo.scale_x(s);
        }
        if let Some(sy) = self.scale_y {
            logo = logo.scale_y(sy);
        } else if let Some(s) = self.scale {
            logo = logo.scale_y(s);
        }
        Ok(logo)
    }
}

// ============ Helpers ============

/// Parse an alignment string and apply it to a value via a closure.
fn parse_alignment_onto<T>(
    value: T,
    component: &'static str,
    align: &str,
    apply: impl FnOnce(T, Alignment) -> T,
) -> Result<T, JsonApiError> {
    let alignment = match align.to_lowercase().as_str() {
        "left" => Alignment::Left,
        "center" => Alignment::Center,
        "right" => Alignment::Right,
        other => {
            return Err(JsonApiError::InvalidField {
                component,
                field: "align",
                message: format!(
                    "expected \"left\", \"center\", or \"right\", got \"{}\"",
                    other
                ),
            })
        }
    };
    Ok(apply(value, alignment))
}

/// Parse a dithering algorithm string.
fn parse_dither_algorithm(s: &str) -> Option<DitheringAlgorithm> {
    match s.to_lowercase().as_str() {
        "bayer" => Some(DitheringAlgorithm::Bayer),
        "floyd-steinberg" | "floyd_steinberg" | "fs" => Some(DitheringAlgorithm::FloydSteinberg),
        "atkinson" => Some(DitheringAlgorithm::Atkinson),
        "jarvis" | "jjn" => Some(DitheringAlgorithm::Jarvis),
        "none" | "threshold" => Some(DitheringAlgorithm::None),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentExt;
    use crate::ir::{BarcodeKind, Op};
    use crate::protocol::barcode::qr::QrErrorLevel;

    #[test]
    fn test_minimal_document() {
        let json = r#"{"document": [{"type": "text", "content": "hi"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        assert!(doc.cut); // default true
        let receipt = doc.to_receipt().unwrap();
        let bytes = receipt.build();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_cut_false() {
        let json = r#"{"document": [{"type": "text", "content": "hi"}], "cut": false}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        assert!(!doc.cut);
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::Cut { .. })));
    }

    #[test]
    fn test_text_bold_center() {
        let json = r#"{"document": [{"type": "text", "content": "Hello", "bold": true, "center": true}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetAlign(Alignment::Center))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::Text(s) if s == "Hello")));
    }

    #[test]
    fn test_text_align_field() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "align": "right"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetAlign(Alignment::Right))));
    }

    #[test]
    fn test_text_font() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "font": "B"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetFont(Font::B))));
    }

    #[test]
    fn test_text_size_uniform() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "size": 2}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetSize { height: 2, width: 2 })));
    }

    #[test]
    fn test_text_size_array() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "size": [2, 3]}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetSize { height: 2, width: 3 })));
    }

    #[test]
    fn test_text_scale_uniform() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "scale": 1}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetExpandedHeight(1))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetExpandedWidth(1))));
    }

    #[test]
    fn test_text_inline() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "inline": true}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        // Inline text should NOT have a Newline after the Text op
        let text_idx = ir
            .ops
            .iter()
            .position(|op| matches!(op, Op::Text(s) if s == "x"))
            .unwrap();
        // Next op after text should NOT be Newline
        if text_idx + 1 < ir.ops.len() {
            assert!(!matches!(ir.ops[text_idx + 1], Op::Newline));
        }
    }

    #[test]
    fn test_header_normal() {
        let json = r#"{"document": [{"type": "header", "content": "STORE"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetAlign(Alignment::Center))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetSize { height: 1, width: 1 })));
    }

    #[test]
    fn test_header_small() {
        let json =
            r#"{"document": [{"type": "header", "content": "small", "variant": "small"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        // Small header should NOT have SetSize
        assert!(!ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_line_item() {
        let json =
            r#"{"document": [{"type": "line_item", "name": "Coffee", "price": 4.50}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        let has_item = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("Coffee") && s.contains("4.50")
            } else {
                false
            }
        });
        assert!(has_item);
    }

    #[test]
    fn test_total_default() {
        let json = r#"{"document": [{"type": "total", "amount": 19.99}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetAlign(Alignment::Right))));
    }

    #[test]
    fn test_total_labeled() {
        let json = r#"{"document": [{"type": "total", "label": "TAX:", "amount": 0.99, "bold": false}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        // Should NOT be bold
        assert!(!ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
        let has_tax = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("TAX:")
            } else {
                false
            }
        });
        assert!(has_tax);
    }

    #[test]
    fn test_divider_styles() {
        for style in &["dashed", "solid", "double", "equals"] {
            let json = format!(
                r#"{{"document": [{{"type": "divider", "style": "{}"}}]}}"#,
                style
            );
            let doc: JsonDocument = serde_json::from_str(&json).unwrap();
            let receipt = doc.to_receipt().unwrap();
            let ir = receipt.compile();
            assert!(ir.ops.iter().any(|op| matches!(op, Op::Text(_))));
        }
    }

    #[test]
    fn test_divider_default() {
        let json = r#"{"document": [{"type": "divider"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        // Default dashed: should contain dashes
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
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Feed { units: 20 })));
    }

    #[test]
    fn test_spacer_lines() {
        let json = r#"{"document": [{"type": "spacer", "lines": 2}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Feed { .. })));
    }

    #[test]
    fn test_spacer_missing_field() {
        let json = r#"{"document": [{"type": "spacer"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let result = doc.to_receipt();
        assert!(result.is_err());
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(err.contains("spacer"));
    }

    #[test]
    fn test_blank_line() {
        let json = r#"{"document": [{"type": "blank_line"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::Newline)));
    }

    #[test]
    fn test_columns() {
        let json = r#"{"document": [{"type": "columns", "left": "Name", "right": "Price", "bold": true}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::SetBold(true))));
    }

    #[test]
    fn test_markdown() {
        let json = r#"{"document": [{"type": "markdown", "content": "**bold** text"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let bytes = receipt.build();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_qr_code() {
        let json = r#"{"document": [{"type": "qr_code", "data": "https://example.com"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(op, Op::QrCode { .. })));
    }

    #[test]
    fn test_qr_code_options() {
        let json = r#"{"document": [{"type": "qr_code", "data": "test", "cell_size": 6, "error_level": "H", "align": "left"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::QrCode {
                cell_size: 6,
                error_level: QrErrorLevel::H,
                ..
            }
        )));
        assert!(ir
            .ops
            .iter()
            .any(|op| matches!(op, Op::SetAlign(Alignment::Left))));
    }

    #[test]
    fn test_pdf417() {
        let json = r#"{"document": [{"type": "pdf417", "data": "TICKET-123", "module_width": 4, "ecc_level": 3}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Pdf417 {
                module_width: 4,
                ecc_level: 3,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_code128() {
        let json = r#"{"document": [{"type": "barcode", "format": "code128", "data": "ABC-123", "height": 100}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Barcode1D {
                kind: BarcodeKind::Code128,
                height: 100,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_invalid_format() {
        let json =
            r#"{"document": [{"type": "barcode", "format": "invalid", "data": "123"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let result = doc.to_receipt();
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern() {
        let json = r#"{"document": [{"type": "pattern", "name": "ripple", "height": 100}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
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
        let json = r#"{"document": [{"type": "pattern", "name": "nonexistent"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let result = doc.to_receipt();
        assert!(result.is_err());
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(err.contains("unknown pattern"));
    }

    #[test]
    fn test_nv_logo() {
        let json = r#"{"document": [{"type": "nv_logo", "key": "A1", "center": true, "scale": 2}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::NvPrint {
                scale_x: 2,
                scale_y: 2,
                ..
            }
        )));
    }

    #[test]
    fn test_invalid_align() {
        let json =
            r#"{"document": [{"type": "text", "content": "x", "align": "middle"}]}"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let result = doc.to_receipt();
        assert!(result.is_err());
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(err.contains("align"));
    }

    #[test]
    fn test_full_briefing() {
        let json = r#"{
            "document": [
                {"type": "header", "content": "DAILY BRIEFING"},
                {"type": "text", "content": "Monday, Jan 27", "center": true, "font": "B"},
                {"type": "divider"},
                {"type": "text", "content": "WEATHER", "bold": true},
                {"type": "columns", "left": "Temperature", "right": "42°F"},
                {"type": "spacer", "mm": 2},
                {"type": "markdown", "content": "- Headline one\n- Headline two"},
                {"type": "divider"},
                {"type": "qr_code", "data": "https://news.example.com"}
            ],
            "cut": true
        }"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let bytes = receipt.build();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_all_component_types() {
        let json = r#"{
            "document": [
                {"type": "text", "content": "hello"},
                {"type": "header", "content": "TITLE"},
                {"type": "header", "content": "small", "variant": "small"},
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
                {"type": "markdown", "content": "**bold**"},
                {"type": "qr_code", "data": "test"},
                {"type": "pdf417", "data": "test"},
                {"type": "barcode", "format": "code128", "data": "TEST"},
                {"type": "nv_logo", "key": "A1"}
            ]
        }"#;
        let doc: JsonDocument = serde_json::from_str(json).unwrap();
        let receipt = doc.to_receipt().unwrap();
        let ir = receipt.compile();
        assert!(ir.len() > 10);
    }
}

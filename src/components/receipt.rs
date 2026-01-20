//! # Receipt Component
//!
//! The root container for building receipts.

use super::Component;
use crate::ir::Op;

/// Receipt is the root container component.
///
/// It holds child components and optionally adds a cut at the end.
///
/// ## Example
///
/// ```
/// use estrella::components::*;
///
/// let receipt = Receipt::new()
///     .child(Header::new("STORE NAME"))
///     .child(Divider::dashed())
///     .child(LineItem::new("Item", 9.99))
///     .child(Total::new(9.99))
///     .cut();
///
/// let bytes = receipt.build();
/// ```
pub struct Receipt {
    children: Vec<Box<dyn Component>>,
    auto_cut: bool,
    partial_cut: bool,
}

impl Default for Receipt {
    fn default() -> Self {
        Self::new()
    }
}

impl Receipt {
    /// Create a new empty receipt.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            auto_cut: false,
            partial_cut: false,
        }
    }

    /// Add a child component.
    pub fn child<C: Component + 'static>(mut self, component: C) -> Self {
        self.children.push(Box::new(component));
        self
    }

    /// Add multiple child components.
    pub fn children<I, C>(mut self, components: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Component + 'static,
    {
        for c in components {
            self.children.push(Box::new(c));
        }
        self
    }

    /// Enable auto-cut at the end (full cut).
    pub fn cut(mut self) -> Self {
        self.auto_cut = true;
        self.partial_cut = false;
        self
    }

    /// Enable auto-cut at the end (partial cut, leaves hinge).
    pub fn partial_cut(mut self) -> Self {
        self.auto_cut = true;
        self.partial_cut = true;
        self
    }
}

impl Component for Receipt {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Emit all children
        for child in &self.children {
            child.emit(ops);
        }

        // Auto-cut if enabled
        if self.auto_cut {
            ops.push(Op::Cut {
                partial: self.partial_cut,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{ComponentExt, Text};

    #[test]
    fn test_empty_receipt() {
        let receipt = Receipt::new();
        let ir = receipt.compile();
        // Just Init
        assert_eq!(ir.len(), 1);
        assert_eq!(ir.ops[0], Op::Init);
    }

    #[test]
    fn test_receipt_with_cut() {
        let receipt = Receipt::new().cut();
        let ir = receipt.compile();
        // Init + Cut
        assert_eq!(ir.len(), 2);
        assert_eq!(ir.ops[1], Op::Cut { partial: false });
    }

    #[test]
    fn test_receipt_with_partial_cut() {
        let receipt = Receipt::new().partial_cut();
        let ir = receipt.compile();
        // Init + Cut(partial)
        assert_eq!(ir.len(), 2);
        assert_eq!(ir.ops[1], Op::Cut { partial: true });
    }

    #[test]
    fn test_receipt_with_children() {
        let receipt = Receipt::new()
            .child(Text::new("Hello"))
            .child(Text::new("World"))
            .cut();

        let ir = receipt.compile();
        // Init + text ops + Cut
        assert!(ir.len() >= 3);
        // Last op should be cut
        assert_eq!(*ir.ops.last().unwrap(), Op::Cut { partial: false });
    }
}

//! # Declarative Components
//!
//! React-like components for building receipts declaratively.
//!
//! ## Design Philosophy
//!
//! Instead of imperative commands, you describe *what* you want:
//!
//! ```
//! use estrella::components::*;
//! use estrella::ir::Program;
//!
//! let receipt = Receipt::new()
//!     .child(Header::new("CHURRA MART"))
//!     .child(Divider::dashed())
//!     .child(LineItem::new("Espresso", 4.50))
//!     .child(Total::new(7.75))
//!     .child(QrCode::new("https://example.com"))
//!     .cut();
//!
//! // Compile to IR (inspectable)
//! let ir = receipt.compile();
//! println!("{:#?}", ir);
//!
//! // Generate bytes
//! let bytes = receipt.build();
//! ```
//!
//! ## Component Trait
//!
//! All components implement the `Component` trait, which emits IR ops.
//! Components can be nested (containers hold children).

mod barcode;
mod graphics;
mod layout;
mod receipt;
mod text;

pub use barcode::*;
pub use graphics::*;
pub use layout::*;
pub use receipt::*;
pub use text::*;

use crate::ir::{Op, Program};
use crate::printer::PrinterConfig;

/// Trait for declarative components.
///
/// Components emit IR ops when compiled. This is the core abstraction
/// that enables the declarative receipt building pattern.
pub trait Component {
    /// Emit IR ops for this component into the ops vector.
    fn emit(&self, ops: &mut Vec<Op>);
}

/// Extension trait for compiling components.
pub trait ComponentExt: Component {
    /// Compile this component to an IR program.
    ///
    /// The program starts with an Init op, followed by the component's ops.
    fn compile(&self) -> Program {
        let mut ops = vec![Op::Init];
        self.emit(&mut ops);
        Program { ops }
    }

    /// Compile, optimize, and generate bytes.
    fn build(&self) -> Vec<u8> {
        self.build_with_config(&PrinterConfig::TSP650II)
    }

    /// Compile, optimize, and generate bytes with a specific printer config.
    fn build_with_config(&self, config: &PrinterConfig) -> Vec<u8> {
        self.compile().optimize().to_bytes_with_config(config)
    }
}

// Blanket implementation for all components
impl<T: Component> ComponentExt for T {}

// Allow boxed components
impl Component for Box<dyn Component> {
    fn emit(&self, ops: &mut Vec<Op>) {
        self.as_ref().emit(ops);
    }
}

// Allow references to components
impl<T: Component + ?Sized> Component for &T {
    fn emit(&self, ops: &mut Vec<Op>) {
        (*self).emit(ops);
    }
}

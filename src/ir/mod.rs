//! # Intermediate Representation (IR)
//!
//! This module provides the IR layer for receipt printing. The IR is a
//! "bytecode" representation that sits between declarative components
//! and raw StarPRNT protocol bytes.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌───────────┐     ┌──────────┐
//! │ Components  │ ──► │     IR      │ ──► │ Optimizer │ ──► │ Codegen  │
//! │(declarative)│     │  (Vec<Op>)  │     │           │     │ (bytes)  │
//! └─────────────┘     └─────────────┘     └───────────┘     └──────────┘
//! ```
//!
//! ## Benefits of IR
//!
//! 1. **Inspectable**: Debug and visualize what will be printed
//! 2. **Optimizable**: Remove redundant style changes, merge text
//! 3. **Testable**: Unit test components without actual printer
//! 4. **Serializable**: Export/import print jobs (future)
//!
//! ## Example
//!
//! ```
//! use estrella::ir::{Op, Program};
//! use estrella::protocol::text::Alignment;
//!
//! let mut program = Program::with_init();
//! program.push(Op::SetAlign(Alignment::Center));
//! program.push(Op::SetBold(true));
//! program.push(Op::Text("HELLO".into()));
//! program.push(Op::Newline);
//! program.push(Op::Cut { partial: false });
//!
//! // Inspect the IR
//! println!("{:#?}", program);
//!
//! // Optimize and generate bytes
//! let optimized = program.optimize();
//! let bytes = optimized.to_bytes();
//! ```

mod codegen;
mod ops;
mod optimize;

// Re-export the ops types (codegen and optimize add methods to Program via impl)
pub use ops::*;

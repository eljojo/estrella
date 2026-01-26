//! # JSON API
//!
//! Deserialize JSON documents into the component library for printing.
//!
//! This module provides a JSON schema that maps directly to Estrella's
//! declarative component system, allowing external tools (e.g., Home Assistant)
//! to construct rich receipts via HTTP.
//!
//! ## Example
//!
//! ```
//! use estrella::json_api::JsonDocument;
//! use estrella::components::ComponentExt;
//!
//! let json = r#"{
//!     "document": [
//!         {"type": "header", "content": "HELLO"},
//!         {"type": "divider"},
//!         {"type": "text", "content": "World", "bold": true}
//!     ],
//!     "cut": true
//! }"#;
//!
//! let doc: JsonDocument = serde_json::from_str(json).unwrap();
//! let receipt = doc.to_receipt().unwrap();
//! let bytes = receipt.build();
//! assert!(!bytes.is_empty());
//! ```

mod convert;
mod schema;

pub use convert::JsonApiError;
pub use schema::{JsonComponent, JsonDocument};

//! # Error Types
//!
//! This module defines error types used throughout the estrella library.

use thiserror::Error;

/// Main error type for estrella operations
#[derive(Debug, Error)]
pub enum EstrellaError {
    /// Transport-level errors (connection, I/O)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Invalid command or parameter
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Pattern generation error
    #[error("Pattern error: {0}")]
    Pattern(String),

    /// Image processing error
    #[error("Image error: {0}")]
    Image(String),

    /// I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

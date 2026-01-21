//! Error types for Tusk application.

use thiserror::Error;

/// Main error type for Tusk application.
#[derive(Debug, Error)]
pub enum TuskError {
    /// Window creation or management error.
    #[error("Window error: {message}")]
    Window {
        /// Human-readable error message.
        message: String,
    },

    /// Theme loading or application error.
    #[error("Theme error: {message}")]
    Theme {
        /// Human-readable error message.
        message: String,
    },

    /// Font loading or rendering error.
    #[error("Font error: {message}")]
    Font {
        /// Human-readable error message.
        message: String,
        /// Optional path to the font file that caused the error.
        path: Option<String>,
    },

    /// Configuration error.
    #[error("Config error: {message}")]
    Config {
        /// Human-readable error message.
        message: String,
    },
}

impl TuskError {
    /// Create a new window error.
    pub fn window(message: impl Into<String>) -> Self {
        Self::Window { message: message.into() }
    }

    /// Create a new theme error.
    pub fn theme(message: impl Into<String>) -> Self {
        Self::Theme { message: message.into() }
    }

    /// Create a new font error.
    pub fn font(message: impl Into<String>) -> Self {
        Self::Font { message: message.into(), path: None }
    }

    /// Create a new font error with a path.
    pub fn font_with_path(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::Font { message: message.into(), path: Some(path.into()) }
    }

    /// Create a new config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
}

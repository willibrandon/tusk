//! Icon management module for Tusk application.
//!
//! This module provides the foundation for icon management.
//! Icons will be loaded from the assets/icons directory.

/// Icon identifiers for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// Application icon.
    App,
}

impl Icon {
    /// Get the icon name for loading from assets.
    pub fn name(&self) -> &'static str {
        match self {
            Icon::App => "tusk",
        }
    }
}

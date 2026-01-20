use serde::Serialize;

/// Application error type for IPC commands.
/// Serializes to JSON for frontend consumption.
#[allow(dead_code)] // Scaffolding for future command implementations
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    /// Machine-readable error code (e.g., "ERR_CONNECTION_FAILED")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Technical detail for debugging (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Actionable suggestion for the user (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

#[allow(dead_code)] // Scaffolding for future command implementations
impl AppError {
    /// Create a new application error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
            hint: None,
        }
    }

    /// Add technical detail to the error
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Add actionable hint to the error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

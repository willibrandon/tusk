use serde::{Deserialize, Serialize};

/// Error category for frontend handling.
/// This enum provides a type-safe way to categorize errors for UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorKind {
    /// PostgreSQL server error
    Database,
    /// Connection establishment/pool error
    Connection,
    /// Local SQLite storage error
    Storage,
    /// Keychain access error
    Credential,
    /// User-initiated cancellation
    QueryCancelled,
    /// Statement timeout exceeded
    QueryTimeout,
    /// Application startup error
    Initialization,
    /// Input validation error
    Validation,
    /// Unexpected internal error
    Internal,
}

/// Structured error response for frontend display.
/// This structure matches the IPC contract for error handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    /// Error category
    pub kind: ErrorKind,
    /// Human-readable error message
    pub message: String,
    /// Additional technical detail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Actionable suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Character position in SQL (for syntax errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// PostgreSQL error code (e.g., "42P01")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ErrorResponse {
    /// Create a simple error response with just a message
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
            hint: None,
            position: None,
            code: None,
        }
    }

    /// Create an error response with a hint
    pub fn with_hint(kind: ErrorKind, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
            hint: Some(hint.into()),
            position: None,
            code: None,
        }
    }
}

impl From<crate::error::TuskError> for ErrorResponse {
    fn from(err: crate::error::TuskError) -> Self {
        match err {
            crate::error::TuskError::Database {
                message,
                code,
                position,
                hint,
                detail,
            } => ErrorResponse {
                kind: ErrorKind::Database,
                message,
                detail,
                hint,
                position,
                code,
            },
            crate::error::TuskError::Connection { message, hint } => ErrorResponse {
                kind: ErrorKind::Connection,
                message,
                detail: None,
                hint,
                position: None,
                code: None,
            },
            crate::error::TuskError::Storage { message, hint } => ErrorResponse {
                kind: ErrorKind::Storage,
                message,
                detail: None,
                hint,
                position: None,
                code: None,
            },
            crate::error::TuskError::Credential { message, hint } => ErrorResponse {
                kind: ErrorKind::Credential,
                message,
                detail: None,
                hint,
                position: None,
                code: None,
            },
            crate::error::TuskError::QueryCancelled => ErrorResponse {
                kind: ErrorKind::QueryCancelled,
                message: "Query was cancelled".to_string(),
                detail: None,
                hint: None,
                position: None,
                code: None,
            },
            crate::error::TuskError::QueryTimeout { elapsed_ms } => ErrorResponse {
                kind: ErrorKind::QueryTimeout,
                message: format!("Query timed out after {}ms", elapsed_ms),
                detail: None,
                hint: Some("Consider adding a LIMIT clause or optimizing the query.".to_string()),
                position: None,
                code: None,
            },
            crate::error::TuskError::Initialization { message, hint } => ErrorResponse {
                kind: ErrorKind::Initialization,
                message,
                detail: None,
                hint,
                position: None,
                code: None,
            },
            crate::error::TuskError::Validation { message, hint } => ErrorResponse {
                kind: ErrorKind::Validation,
                message,
                detail: None,
                hint,
                position: None,
                code: None,
            },
            crate::error::TuskError::Internal(message) => ErrorResponse {
                kind: ErrorKind::Internal,
                message,
                detail: None,
                hint: Some("This is an unexpected error. Please report it.".to_string()),
                position: None,
                code: None,
            },
        }
    }
}

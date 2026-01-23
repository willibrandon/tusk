//! Error types for Tusk application.
//!
//! Provides comprehensive error handling with PostgreSQL-specific details (FR-001 through FR-004).

use thiserror::Error;
use uuid::Uuid;

/// Main error type for Tusk application (FR-001).
#[derive(Debug, Error)]
pub enum TuskError {
    /// Database connection failed.
    #[error("Connection error: {message}")]
    Connection {
        /// Human-readable error message.
        message: String,
        /// Optional underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication failed.
    #[error("Authentication error: {message}")]
    Authentication {
        /// Human-readable error message.
        message: String,
        /// Actionable hint for the user.
        hint: Option<String>,
    },

    /// SSL/TLS error.
    #[error("SSL error: {message}")]
    Ssl {
        /// Human-readable error message.
        message: String,
        /// Optional underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// SSH tunnel error.
    #[error("SSH error: {message}")]
    Ssh {
        /// Human-readable error message.
        message: String,
        /// Optional underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Query execution error with PostgreSQL-specific details (FR-002).
    #[error("{message}")]
    Query {
        /// PostgreSQL error message.
        message: String,
        /// Additional detail from PostgreSQL.
        detail: Option<String>,
        /// PostgreSQL hint.
        hint: Option<String>,
        /// Position in query (1-indexed).
        position: Option<usize>,
        /// PostgreSQL error code (e.g., "42P01").
        code: Option<String>,
    },

    /// Query was cancelled.
    #[error("Query cancelled")]
    QueryCancelled {
        /// ID of the cancelled query.
        query_id: Uuid,
    },

    /// Local SQLite storage error.
    #[error("Storage error: {message}")]
    Storage {
        /// Human-readable error message.
        message: String,
        /// Actionable hint for the user.
        hint: Option<String>,
        /// Optional underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// OS keychain error.
    #[error("Keyring error: {message}")]
    Keyring {
        /// Human-readable error message.
        message: String,
        /// Actionable hint for the user.
        hint: Option<String>,
    },

    /// Connection pool exhausted (FR-013a).
    #[error("Pool timeout: {message}")]
    PoolTimeout {
        /// Human-readable error message.
        message: String,
        /// Number of tasks waiting for connections.
        waiting: usize,
    },

    /// Unexpected internal error.
    #[error("Internal error: {message}")]
    Internal {
        /// Human-readable error message.
        message: String,
        /// Optional underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

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
    // ========== Constructors ==========

    /// Create a new connection error.
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection { message: message.into(), source: None }
    }

    /// Create a new connection error with source.
    pub fn connection_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Connection { message: message.into(), source: Some(Box::new(source)) }
    }

    /// Create a new authentication error.
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
            hint: Some("Check username and password".to_string()),
        }
    }

    /// Create a new authentication error with custom hint.
    pub fn authentication_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Authentication { message: message.into(), hint: Some(hint.into()) }
    }

    /// Create a new SSL error.
    pub fn ssl(message: impl Into<String>) -> Self {
        Self::Ssl { message: message.into(), source: None }
    }

    /// Create a new SSH error.
    pub fn ssh(message: impl Into<String>) -> Self {
        Self::Ssh { message: message.into(), source: None }
    }

    /// Create a new query error with full PostgreSQL details (FR-002).
    pub fn query(
        message: impl Into<String>,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<usize>,
        code: Option<String>,
    ) -> Self {
        Self::Query { message: message.into(), detail, hint, position, code }
    }

    /// Create a query cancelled error.
    pub fn query_cancelled(query_id: Uuid) -> Self {
        Self::QueryCancelled { query_id }
    }

    /// Create a new storage error.
    pub fn storage(message: impl Into<String>, hint: Option<&str>) -> Self {
        Self::Storage { message: message.into(), hint: hint.map(String::from), source: None }
    }

    /// Create a new storage error with source.
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Storage { message: message.into(), hint: None, source: Some(Box::new(source)) }
    }

    /// Create a new keyring error.
    pub fn keyring(message: impl Into<String>, hint: Option<&str>) -> Self {
        Self::Keyring { message: message.into(), hint: hint.map(String::from) }
    }

    /// Create a new pool timeout error (FR-013a).
    pub fn pool_timeout(message: impl Into<String>, waiting: usize) -> Self {
        Self::PoolTimeout { message: message.into(), waiting }
    }

    /// Create a new internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into(), source: None }
    }

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

    // ========== Methods ==========

    /// Check if this error represents a cancelled query.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::QueryCancelled { .. })
    }

    /// Check if this error represents a connection lost during query (T049).
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, Self::Connection { .. })
    }

    /// Get the error category name.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Connection { .. } => "Connection",
            Self::Authentication { .. } => "Authentication",
            Self::Ssl { .. } => "SSL",
            Self::Ssh { .. } => "SSH",
            Self::Query { .. } => "Query",
            Self::QueryCancelled { .. } => "Query",
            Self::Storage { .. } => "Storage",
            Self::Keyring { .. } => "Keyring",
            Self::PoolTimeout { .. } => "Pool",
            Self::Internal { .. } => "Internal",
            Self::Window { .. } => "Window",
            Self::Theme { .. } => "Theme",
            Self::Font { .. } => "Font",
            Self::Config { .. } => "Config",
        }
    }

    /// Get actionable hint for the user.
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::Connection { .. } => Some("Check that the database server is running"),
            Self::Authentication { hint, .. } => hint.as_deref(),
            Self::Ssl { .. } => Some("Verify SSL certificate configuration"),
            Self::Ssh { .. } => Some("Check SSH key permissions"),
            Self::Query { hint, .. } => hint.as_deref(),
            Self::QueryCancelled { .. } => None,
            Self::Storage { hint, .. } => hint.as_deref(),
            Self::Keyring { hint, .. } => hint.as_deref(),
            Self::PoolTimeout { .. } => Some("Try closing unused connections"),
            Self::Internal { .. } => Some("Please report this issue"),
            Self::Window { .. } => None,
            Self::Theme { .. } => None,
            Self::Font { .. } => None,
            Self::Config { .. } => None,
        }
    }

    /// Get PostgreSQL error code (if applicable).
    pub fn pg_code(&self) -> Option<&str> {
        match self {
            Self::Query { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// Get position in query (if applicable).
    pub fn position(&self) -> Option<usize> {
        match self {
            Self::Query { position, .. } => *position,
            _ => None,
        }
    }

    /// Convert to user-displayable error info (FR-003, FR-019, FR-020, FR-021).
    ///
    /// Display rules per error-handling.md contract:
    /// - recoverable=true, no position: Toast notification (auto-dismiss 10s)
    /// - recoverable=true, has position: Error panel (query errors)
    /// - recoverable=false: Error panel + log (critical errors)
    pub fn to_error_info(&self) -> ErrorInfo {
        match self {
            Self::Connection { message, .. } => ErrorInfo {
                error_type: "Connection Error".to_string(),
                message: message.clone(),
                hint: Some("Check that the database server is running and accessible".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Authentication { message, hint } => ErrorInfo {
                error_type: "Authentication Failed".to_string(),
                message: message.clone(),
                hint: hint.clone().or(Some("Verify username and password".to_string())),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Ssl { message, .. } => ErrorInfo {
                error_type: "SSL Error".to_string(),
                message: message.clone(),
                hint: Some("Verify SSL certificate configuration".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Ssh { message, .. } => ErrorInfo {
                error_type: "SSH Error".to_string(),
                message: message.clone(),
                hint: Some("Check SSH key permissions".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Query { message, detail, hint, position, code } => ErrorInfo {
                error_type: "Query Error".to_string(),
                message: message.clone(),
                hint: hint.clone().or_else(|| Self::hint_for_pg_code(code.as_deref())),
                technical_detail: detail.clone(),
                position: *position,
                code: code.clone(),
                recoverable: true,
            },

            Self::QueryCancelled { .. } => ErrorInfo {
                error_type: "Query Cancelled".to_string(),
                message: "Query was cancelled by user".to_string(),
                hint: None,
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Storage { message, hint, .. } => ErrorInfo {
                error_type: "Storage Error".to_string(),
                message: message.clone(),
                hint: hint.clone().or(Some("Check file permissions and disk space".to_string())),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false, // Storage errors are typically not recoverable
            },

            Self::Keyring { message, hint } => ErrorInfo {
                error_type: "Credential Storage Error".to_string(),
                message: message.clone(),
                hint: hint
                    .clone()
                    .or(Some("Password will be stored for this session only".to_string())),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            Self::PoolTimeout { message, waiting } => ErrorInfo {
                error_type: "Connection Pool Timeout".to_string(),
                message: message.clone(),
                hint: Some(format!("{} queries waiting. Consider closing unused tabs", waiting)),
                technical_detail: Some(format!("{} tasks waiting for connections", waiting)),
                position: None,
                code: None,
                recoverable: true,
            },

            Self::Internal { message, .. } => ErrorInfo {
                error_type: "Internal Error".to_string(),
                message: message.clone(),
                hint: Some("Please report this issue".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false, // Internal errors are not recoverable
            },

            Self::Window { message } => ErrorInfo {
                error_type: "Window Error".to_string(),
                message: message.clone(),
                hint: None,
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false,
            },

            Self::Theme { message } => ErrorInfo {
                error_type: "Theme Error".to_string(),
                message: message.clone(),
                hint: None,
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false,
            },

            Self::Font { message, path } => ErrorInfo {
                error_type: "Font Error".to_string(),
                message: message.clone(),
                hint: None,
                technical_detail: path.clone(),
                position: None,
                code: None,
                recoverable: false,
            },

            Self::Config { message } => ErrorInfo {
                error_type: "Config Error".to_string(),
                message: message.clone(),
                hint: None,
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false,
            },
        }
    }

    /// Map PostgreSQL error codes to user-friendly hints (T065).
    ///
    /// Common PostgreSQL error codes per error-handling.md contract.
    fn hint_for_pg_code(code: Option<&str>) -> Option<String> {
        let code = code?;
        match code {
            // Authentication errors (28xxx)
            "28P01" => Some("Check your password and try again".to_string()),
            "28000" => Some("Authentication failed - check username and permissions".to_string()),

            // Connection exceptions (08xxx)
            "08000" => Some("Connection error occurred".to_string()),
            "08003" => Some("Connection does not exist".to_string()),
            "08006" => Some("Connection failure".to_string()),
            "08001" => Some("Unable to establish connection".to_string()),
            "08004" => Some("Server rejected the connection".to_string()),

            // Database errors (3Dxxx)
            "3D000" => Some("Database does not exist".to_string()),

            // Syntax and semantic errors (42xxx)
            "42601" => Some("Check SQL syntax".to_string()),
            "42P01" => Some("Table does not exist".to_string()),
            "42703" => Some("Column does not exist".to_string()),
            "42501" => Some("Insufficient privileges".to_string()),
            "42P02" => Some("Parameter does not exist".to_string()),

            // Resource limit errors (53xxx)
            "53000" => Some("Insufficient resources".to_string()),
            "53100" => Some("Disk full".to_string()),
            "53200" => Some("Out of memory".to_string()),
            "53300" => Some("Server connection limit reached. Try again later".to_string()),

            // Query cancelled (57xxx)
            "57014" => Some("Query was cancelled".to_string()),
            "57P01" => Some("Database server is shutting down".to_string()),
            "57P02" => Some("Database server is starting up".to_string()),
            "57P03" => Some("Cannot connect now - server not accepting connections".to_string()),

            // Other errors
            _ => None,
        }
    }
}

/// User-displayable error information (FR-003, FR-019, FR-020, FR-021).
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Category name (e.g., "Connection Error").
    pub error_type: String,
    /// User-friendly message.
    pub message: String,
    /// Actionable suggestion.
    pub hint: Option<String>,
    /// Technical detail for "Show Details" expansion.
    pub technical_detail: Option<String>,
    /// Character position for query errors (1-indexed).
    pub position: Option<usize>,
    /// PostgreSQL error code (e.g., "42P01" for undefined table).
    pub code: Option<String>,
    /// Whether the error is recoverable (affects display type).
    /// - true: Show as toast notification (auto-dismiss 10s)
    /// - false: Show as error panel/modal
    pub recoverable: bool,
}

// ========== Error Conversions (FR-004) ==========

/// Convert from tokio_postgres::Error to TuskError (T066, T067).
///
/// Maps PostgreSQL error codes to appropriate TuskError variants with
/// user-friendly hints per SC-006 requirements.
impl From<tokio_postgres::Error> for TuskError {
    fn from(err: tokio_postgres::Error) -> Self {
        // Try to extract PostgreSQL error details
        if let Some(db_err) = err.as_db_error() {
            let message = db_err.message().to_string();
            let detail = db_err.detail().map(String::from);
            let hint = db_err.hint().map(String::from);
            let position = db_err.position().and_then(|p| match p {
                tokio_postgres::error::ErrorPosition::Original(pos) => Some(*pos as usize),
                tokio_postgres::error::ErrorPosition::Internal { .. } => None,
            });
            let code = Some(db_err.code().code().to_string());
            let code_str = db_err.code().code();

            // Map specific error codes to appropriate variants (T066)
            match code_str {
                // Authentication errors (E01, E02 per error-handling.md)
                "28P01" => {
                    return TuskError::Authentication {
                        message,
                        hint: Some("Check your password and try again".to_string()),
                    }
                }
                "28000" => {
                    return TuskError::Authentication {
                        message,
                        hint: Some(
                            "Authentication failed - check username and permissions".to_string(),
                        ),
                    }
                }

                // Connection exceptions (08xxx) - E03, E04, E15
                "08000" => {
                    return TuskError::Connection {
                        message: format!("{} - Check network connectivity", message),
                        source: Some(Box::new(err)),
                    }
                }
                "08001" => return TuskError::Connection { message, source: Some(Box::new(err)) },
                "08003" => {
                    return TuskError::Connection {
                        message: "Connection lost. Reconnect to continue".to_string(),
                        source: Some(Box::new(err)),
                    }
                }
                "08006" => {
                    return TuskError::Connection {
                        message: "Connection to server lost. Reconnect to continue".to_string(),
                        source: Some(Box::new(err)),
                    }
                }

                // Database does not exist (E05)
                "3D000" => {
                    return TuskError::Connection {
                        message: format!("{} - Database does not exist on this server", message),
                        source: Some(Box::new(err)),
                    }
                }

                // Too many connections (E19)
                "53300" => {
                    return TuskError::Connection {
                        message: format!(
                            "{} - Server connection limit reached. Try again later",
                            message
                        ),
                        source: Some(Box::new(err)),
                    }
                }

                // Admin shutdown (E18)
                "57P01" => {
                    return TuskError::Connection {
                        message: "Database server is shutting down".to_string(),
                        source: Some(Box::new(err)),
                    }
                }

                // Query cancelled (E12, E13)
                "57014" => {
                    // This is handled separately via QueryCancelled, but just in case
                    return TuskError::Query {
                        message: "Query was cancelled".to_string(),
                        detail,
                        hint: Some("Query was cancelled by database administrator".to_string()),
                        position,
                        code,
                    };
                }

                // Other connection exceptions
                _ if code_str.starts_with("08") => {
                    return TuskError::Connection { message, source: Some(Box::new(err)) }
                }

                // Syntax/semantic errors (42xxx) and others - return as Query error
                _ => return TuskError::Query { message, detail, hint, position, code },
            }
        }

        // Check for connection timeout (T067)
        let err_msg = err.to_string().to_lowercase();
        if err_msg.contains("timeout") || err_msg.contains("timed out") {
            return TuskError::Connection {
                message: "Connection timeout - Server may be slow or unreachable. Check network connectivity".to_string(),
                source: Some(Box::new(err)),
            };
        }

        // Connection errors without db_error details
        if err.is_closed() {
            return TuskError::Connection {
                message: "Connection lost. Reconnect to continue".to_string(),
                source: Some(Box::new(err)),
            };
        }

        // Generic fallback
        TuskError::Connection { message: err.to_string(), source: Some(Box::new(err)) }
    }
}

/// Convert from rusqlite::Error to TuskError.
impl From<rusqlite::Error> for TuskError {
    fn from(err: rusqlite::Error) -> Self {
        TuskError::Storage {
            message: err.to_string(),
            hint: Some("The local database may be corrupted".to_string()),
            source: Some(Box::new(err)),
        }
    }
}

/// Convert from std::io::Error to TuskError.
impl From<std::io::Error> for TuskError {
    fn from(err: std::io::Error) -> Self {
        TuskError::Storage {
            message: err.to_string(),
            hint: Some("Check file permissions and disk space".to_string()),
            source: Some(Box::new(err)),
        }
    }
}

/// Convert from serde_json::Error to TuskError.
impl From<serde_json::Error> for TuskError {
    fn from(err: serde_json::Error) -> Self {
        TuskError::Storage {
            message: format!("JSON error: {err}"),
            hint: Some("Data may be corrupted".to_string()),
            source: Some(Box::new(err)),
        }
    }
}

/// Convert from keyring::Error to TuskError.
impl From<keyring::Error> for TuskError {
    fn from(err: keyring::Error) -> Self {
        TuskError::Keyring {
            message: err.to_string(),
            hint: Some("Grant Tusk access in system preferences".to_string()),
        }
    }
}

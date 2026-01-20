use serde::Serialize;
use std::error::Error;

/// Unified error type for all Tusk backend operations.
/// This enum is serializable for IPC transport to the frontend.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum TuskError {
    /// Database error from PostgreSQL with full error metadata
    #[error("Database error: {message}")]
    Database {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },

    /// Connection establishment or pool error
    #[error("Connection failed: {message}")]
    Connection {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },

    /// Local SQLite storage error
    #[error("Storage error: {message}")]
    Storage {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },

    /// OS keychain credential error
    #[error("Credential error: {message}")]
    Credential {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },

    /// Query was cancelled by user
    #[error("Query cancelled")]
    QueryCancelled,

    /// Query exceeded timeout
    #[error("Query timeout after {elapsed_ms}ms")]
    QueryTimeout { elapsed_ms: u64 },

    /// Application startup/initialization error
    #[error("Initialization failed: {message}")]
    Initialization {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },

    /// Input validation error
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },

    /// Unexpected internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl TuskError {
    /// Create a database error from a message
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            code: None,
            position: None,
            hint: None,
            detail: None,
        }
    }

    /// Create a connection error with optional hint
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a connection error with hint
    pub fn connection_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create a storage error with optional hint
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a storage error with hint
    pub fn storage_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create a credential error with optional hint
    pub fn credential(message: impl Into<String>) -> Self {
        Self::Credential {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a credential error with hint
    pub fn credential_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Credential {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create an initialization error with optional hint
    pub fn initialization(message: impl Into<String>) -> Self {
        Self::Initialization {
            message: message.into(),
            hint: None,
        }
    }

    /// Create an initialization error with hint
    pub fn initialization_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Initialization {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create a validation error with optional hint
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a validation error with hint
    pub fn validation_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }
}

/// Convert tokio_postgres error to TuskError, preserving PostgreSQL error metadata
impl From<tokio_postgres::Error> for TuskError {
    fn from(err: tokio_postgres::Error) -> Self {
        // Try to extract DbError for rich error information
        if let Some(db_err) = err
            .source()
            .and_then(|e| e.downcast_ref::<tokio_postgres::error::DbError>())
        {
            let hint = generate_postgres_hint(db_err.code().code(), db_err.hint());

            TuskError::Database {
                message: db_err.message().to_string(),
                code: Some(db_err.code().code().to_string()),
                position: db_err.position().map(|p| match p {
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => {
                        *position
                    }
                }),
                hint,
                detail: db_err.detail().map(|s| s.to_string()),
            }
        } else {
            // Connection-level error without DbError
            let message = err.to_string();
            let hint = generate_connection_hint(&message);

            if message.contains("connection refused")
                || message.contains("Connection refused")
                || message.contains("timeout")
                || message.contains("could not connect")
            {
                TuskError::Connection {
                    message,
                    hint: Some(hint),
                }
            } else {
                TuskError::Database {
                    message,
                    code: None,
                    position: None,
                    hint: Some(hint),
                    detail: None,
                }
            }
        }
    }
}

/// Convert rusqlite error to TuskError
impl From<rusqlite::Error> for TuskError {
    fn from(err: rusqlite::Error) -> Self {
        let message = err.to_string();
        let hint = match &err {
            rusqlite::Error::SqliteFailure(ffi_err, _) => match ffi_err.code {
                rusqlite::ffi::ErrorCode::DatabaseBusy => {
                    Some("The database is busy. Try again in a moment.".to_string())
                }
                rusqlite::ffi::ErrorCode::DatabaseLocked => {
                    Some("The database is locked. Close other applications using it.".to_string())
                }
                rusqlite::ffi::ErrorCode::DiskFull => {
                    Some("Disk is full. Free up space and try again.".to_string())
                }
                rusqlite::ffi::ErrorCode::PermissionDenied => {
                    Some("Permission denied. Check file permissions.".to_string())
                }
                rusqlite::ffi::ErrorCode::ReadOnly => {
                    Some("Database is read-only. Check file permissions.".to_string())
                }
                rusqlite::ffi::ErrorCode::DatabaseCorrupt => {
                    Some("Database is corrupted. Restart the application to attempt repair.".to_string())
                }
                _ => None,
            },
            rusqlite::Error::QueryReturnedNoRows => None,
            _ => None,
        };

        TuskError::Storage {
            message,
            hint,
        }
    }
}

/// Convert keyring error to TuskError
impl From<keyring::Error> for TuskError {
    fn from(err: keyring::Error) -> Self {
        let (message, hint) = match &err {
            keyring::Error::NoEntry => (
                "No password found".to_string(),
                Some("The password may not have been saved. Enter it manually.".to_string()),
            ),
            keyring::Error::Ambiguous(_) => (
                "Ambiguous credential".to_string(),
                Some("Multiple credentials match. Delete duplicates from your keychain.".to_string()),
            ),
            keyring::Error::TooLong(_, _) => (
                "Credential too long".to_string(),
                Some("The password exceeds the maximum length supported by your keychain.".to_string()),
            ),
            keyring::Error::Invalid(_, _) => (
                "Invalid credential format".to_string(),
                Some("The credential data is invalid. Try deleting and re-saving.".to_string()),
            ),
            keyring::Error::NoStorageAccess(_) => (
                "Keychain access denied".to_string(),
                Some("Grant Tusk access to your keychain in system settings.".to_string()),
            ),
            keyring::Error::PlatformFailure(_) => (
                "Keychain unavailable".to_string(),
                Some("Your system keychain is unavailable. Check if the keychain service is running.".to_string()),
            ),
            _ => (err.to_string(), None),
        };

        TuskError::Credential { message, hint }
    }
}

/// Convert std::io::Error to TuskError
impl From<std::io::Error> for TuskError {
    fn from(err: std::io::Error) -> Self {
        let hint = match err.kind() {
            std::io::ErrorKind::NotFound => {
                Some("The file or directory was not found.".to_string())
            }
            std::io::ErrorKind::PermissionDenied => {
                Some("Permission denied. Check file permissions.".to_string())
            }
            std::io::ErrorKind::AlreadyExists => {
                Some("The file or directory already exists.".to_string())
            }
            _ => None,
        };

        TuskError::Storage {
            message: err.to_string(),
            hint,
        }
    }
}

/// Generate an actionable hint for PostgreSQL error codes
fn generate_postgres_hint(code: &str, db_hint: Option<&str>) -> Option<String> {
    // Return the database-provided hint if available
    if let Some(hint) = db_hint {
        return Some(hint.to_string());
    }

    // Generate hints for common error codes
    // See: https://www.postgresql.org/docs/current/errcodes-appendix.html
    match code {
        // Class 08 — Connection Exception
        "08000" | "08003" | "08006" => Some(
            "Check that the database server is running and accepting connections.".to_string(),
        ),
        "08001" => Some("Unable to connect. Verify host, port, and network connectivity.".to_string()),
        "08004" => Some("Connection rejected. Check authentication settings.".to_string()),
        "08P01" => Some("Protocol error. The client and server may be incompatible versions.".to_string()),

        // Class 28 — Invalid Authorization Specification
        "28000" => Some("Invalid authorization. Check username and password.".to_string()),
        "28P01" => Some("Password authentication failed. Verify your password is correct.".to_string()),

        // Class 3D — Invalid Catalog Name
        "3D000" => Some("Database does not exist. Check the database name.".to_string()),

        // Class 3F — Invalid Schema Name
        "3F000" => Some("Schema does not exist. Check the schema name.".to_string()),

        // Class 42 — Syntax Error or Access Rule Violation
        "42601" => Some("SQL syntax error. Check your query syntax.".to_string()),
        "42501" => Some("Permission denied. You may not have access to this object.".to_string()),
        "42P01" => Some("Table does not exist. Check the table name and schema.".to_string()),
        "42703" => Some("Column does not exist. Check column names in your query.".to_string()),
        "42883" => Some("Function does not exist. Check the function name and argument types.".to_string()),

        // Class 53 — Insufficient Resources
        "53000" => Some("Insufficient resources. The server may be overloaded.".to_string()),
        "53100" => Some("Disk full. Free up space on the database server.".to_string()),
        "53200" => Some("Out of memory. The query may be too complex.".to_string()),
        "53300" => Some("Too many connections. Try again later or increase max_connections.".to_string()),

        // Class 57 — Operator Intervention
        "57014" => Some("Query cancelled. The query was interrupted.".to_string()),
        "57P01" => Some("Server is shutting down. Try reconnecting later.".to_string()),
        "57P02" => Some("Crash shutdown. The server crashed and restarted.".to_string()),
        "57P03" => Some("Cannot connect now. The server is starting up.".to_string()),

        _ => None,
    }
}

/// Generate an actionable hint for connection errors based on the message
fn generate_connection_hint(message: &str) -> String {
    let lower = message.to_lowercase();

    if lower.contains("connection refused") {
        "Check that the PostgreSQL server is running and accepting connections on the specified host and port.".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "The connection timed out. Check network connectivity and firewall settings.".to_string()
    } else if lower.contains("host not found") || lower.contains("name resolution") {
        "Could not resolve hostname. Check the server address.".to_string()
    } else if lower.contains("ssl") || lower.contains("tls") {
        "SSL/TLS error. Check SSL settings and certificate configuration.".to_string()
    } else if lower.contains("authentication") || lower.contains("password") {
        "Authentication failed. Verify your username and password.".to_string()
    } else {
        "Check server address, port, and network connectivity.".to_string()
    }
}

/// Type alias for Result with TuskError
pub type TuskResult<T> = Result<T, TuskError>;

# Feature 28: Error Handling and Recovery

## Overview

This feature implements comprehensive error handling across the application using GPUI's native error propagation and UI patterns. It covers connection errors, query errors, and recovery mechanisms. Errors are displayed with user-friendly messages, technical details for debugging, actionable hints, and position highlighting for query syntax errors. The system provides auto-reconnect functionality and ensures unsaved queries are never lost.

## Goals

1. Provide user-friendly error messages with actionable hints
2. Display Postgres error details (SQLSTATE, position, detail, hint)
3. Highlight error positions in the native query editor
4. Implement auto-reconnect with retry UI
5. Persist unsaved queries to prevent data loss
6. Enable quick actions (search error, copy, retry)
7. Graceful handling of connection drops mid-query
8. Consistent error structure across all operations

## Dependencies

- Feature 01: Project Setup (GPUI application)
- Feature 05: Local Storage (query persistence)
- Feature 07: Connection Management (reconnection)
- Feature 12: Query Editor (error highlighting)

## Technical Specification

### 28.1 Error Types and Structure

**File: `src/error.rs`**

```rust
use serde::{Deserialize, Serialize};
use tokio_postgres::error::SqlState;
use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum TuskError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    #[error("Database error: {message}")]
    Database {
        message: String,
        code: Option<String>,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<u32>,
    },

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("SSH tunnel error: {0}")]
    SshTunnel(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Update error: {0}")]
    Update(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Connection-specific errors
#[derive(Error, Debug, Clone)]
pub enum ConnectionError {
    #[error("Cannot connect to server at {host}:{port}")]
    ConnectionRefused { host: String, port: u16 },

    #[error("Connection timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u32 },

    #[error("SSL connection required by server")]
    SslRequired,

    #[error("SSL certificate error: {0}")]
    SslCertificate(String),

    #[error("Host not found: {0}")]
    HostNotFound(String),

    #[error("Network unreachable")]
    NetworkUnreachable,

    #[error("Connection reset by server")]
    ConnectionReset,

    #[error("Too many connections")]
    TooManyConnections,

    #[error("Database '{0}' does not exist")]
    DatabaseNotFound(String),

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,
}

/// Query-specific errors
#[derive(Error, Debug, Clone)]
pub enum QueryError {
    #[error("Syntax error at position {position}")]
    Syntax { position: u32, message: String },

    #[error("Statement timeout exceeded")]
    StatementTimeout,

    #[error("Query cancelled by user")]
    Cancelled,

    #[error("Permission denied")]
    PermissionDenied { object: String, operation: String },

    #[error("Relation '{0}' does not exist")]
    RelationNotFound(String),

    #[error("Column '{column}' does not exist in '{table}'")]
    ColumnNotFound { column: String, table: String },

    #[error("Duplicate key violates unique constraint")]
    UniqueViolation { constraint: String, detail: String },

    #[error("Foreign key violation")]
    ForeignKeyViolation { constraint: String, detail: String },

    #[error("Check constraint violation")]
    CheckViolation { constraint: String, detail: String },

    #[error("Not null violation: column '{0}' cannot be null")]
    NotNullViolation(String),

    #[error("Deadlock detected")]
    Deadlock,
}

/// User-facing error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<u32>,
    pub severity: ErrorSeverity,
    pub recoverable: bool,
    pub actions: Vec<ErrorAction>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Notice,
    Info,
}

impl ErrorSeverity {
    pub fn color(&self) -> gpui::Hsla {
        match self {
            ErrorSeverity::Error => gpui::hsla(0.0, 0.7, 0.5, 1.0),      // Red
            ErrorSeverity::Warning => gpui::hsla(0.12, 0.9, 0.5, 1.0),   // Orange
            ErrorSeverity::Notice => gpui::hsla(0.58, 0.7, 0.5, 1.0),    // Blue
            ErrorSeverity::Info => gpui::hsla(0.0, 0.0, 0.6, 1.0),       // Gray
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAction {
    pub id: String,
    pub label: String,
    pub action_type: ErrorActionType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorActionType {
    Retry,
    Reconnect,
    SearchError,
    CopyError,
    EditConnection,
    ViewPosition,
    Dismiss,
}

impl TuskError {
    /// Convert to user-facing response with actions
    pub fn to_response(&self) -> ErrorResponse {
        match self {
            TuskError::Connection(e) => e.to_response(),
            TuskError::Query(e) => e.to_response(),
            TuskError::Database { message, code, detail, hint, position } => {
                ErrorResponse {
                    code: code.clone().unwrap_or_else(|| "DB_ERROR".into()),
                    message: message.clone(),
                    detail: detail.clone(),
                    hint: hint.clone(),
                    position: *position,
                    severity: ErrorSeverity::Error,
                    recoverable: false,
                    actions: vec![
                        ErrorAction {
                            id: "search".into(),
                            label: "Search Error".into(),
                            action_type: ErrorActionType::SearchError,
                        },
                        ErrorAction {
                            id: "copy".into(),
                            label: "Copy Error".into(),
                            action_type: ErrorActionType::CopyError,
                        },
                    ],
                }
            }
            TuskError::Authentication(msg) => ErrorResponse {
                code: "AUTH_FAILED".into(),
                message: msg.clone(),
                detail: None,
                hint: Some("Check your username and password".into()),
                position: None,
                severity: ErrorSeverity::Error,
                recoverable: true,
                actions: vec![
                    ErrorAction {
                        id: "edit".into(),
                        label: "Edit Connection".into(),
                        action_type: ErrorActionType::EditConnection,
                    },
                ],
            },
            TuskError::SshTunnel(msg) => ErrorResponse {
                code: "SSH_ERROR".into(),
                message: format!("SSH tunnel failed: {}", msg),
                detail: None,
                hint: Some("Check SSH host, port, username, and key".into()),
                position: None,
                severity: ErrorSeverity::Error,
                recoverable: true,
                actions: vec![
                    ErrorAction {
                        id: "edit".into(),
                        label: "Edit Connection".into(),
                        action_type: ErrorActionType::EditConnection,
                    },
                    ErrorAction {
                        id: "retry".into(),
                        label: "Retry".into(),
                        action_type: ErrorActionType::Retry,
                    },
                ],
            },
            TuskError::Timeout(msg) => ErrorResponse {
                code: "TIMEOUT".into(),
                message: msg.clone(),
                detail: None,
                hint: Some("Increase timeout in connection settings".into()),
                position: None,
                severity: ErrorSeverity::Error,
                recoverable: true,
                actions: vec![
                    ErrorAction {
                        id: "retry".into(),
                        label: "Retry".into(),
                        action_type: ErrorActionType::Retry,
                    },
                ],
            },
            TuskError::Cancelled => ErrorResponse {
                code: "CANCELLED".into(),
                message: "Operation cancelled".into(),
                detail: None,
                hint: None,
                position: None,
                severity: ErrorSeverity::Info,
                recoverable: false,
                actions: vec![],
            },
            _ => ErrorResponse {
                code: "INTERNAL".into(),
                message: self.to_string(),
                detail: None,
                hint: None,
                position: None,
                severity: ErrorSeverity::Error,
                recoverable: false,
                actions: vec![
                    ErrorAction {
                        id: "copy".into(),
                        label: "Copy Error".into(),
                        action_type: ErrorActionType::CopyError,
                    },
                ],
            },
        }
    }
}

impl ConnectionError {
    pub fn to_response(&self) -> ErrorResponse {
        let (code, message, hint, recoverable) = match self {
            ConnectionError::ConnectionRefused { host, port } => (
                "ECONNREFUSED",
                format!("Cannot connect to server at {}:{}", host, port),
                Some("Check that the server is running and accepting connections".into()),
                true,
            ),
            ConnectionError::Timeout { timeout_secs } => (
                "TIMEOUT",
                format!("Connection timed out after {} seconds", timeout_secs),
                Some("Increase connection timeout or check network".into()),
                true,
            ),
            ConnectionError::SslRequired => (
                "SSL_REQUIRED",
                "Server requires SSL connection".into(),
                Some("Enable SSL in connection settings".into()),
                true,
            ),
            ConnectionError::SslCertificate(msg) => (
                "SSL_CERT_ERROR",
                format!("SSL certificate error: {}", msg),
                Some("Check certificate or use 'require' SSL mode".into()),
                true,
            ),
            ConnectionError::HostNotFound(host) => (
                "HOST_NOT_FOUND",
                format!("Host not found: {}", host),
                Some("Check the hostname or IP address".into()),
                true,
            ),
            ConnectionError::NetworkUnreachable => (
                "NETWORK_UNREACHABLE",
                "Network is unreachable".into(),
                Some("Check your network connection".into()),
                true,
            ),
            ConnectionError::ConnectionReset => (
                "CONNECTION_RESET",
                "Connection reset by server".into(),
                Some("The server closed the connection unexpectedly".into()),
                true,
            ),
            ConnectionError::TooManyConnections => (
                "TOO_MANY_CONNECTIONS",
                "Too many connections".into(),
                Some("Close unused connections or increase max_connections".into()),
                true,
            ),
            ConnectionError::DatabaseNotFound(db) => (
                "DATABASE_NOT_FOUND",
                format!("Database '{}' does not exist", db),
                Some("Check the database name or create it first".into()),
                true,
            ),
            ConnectionError::ConnectionClosed => (
                "CONNECTION_CLOSED",
                "Connection closed unexpectedly".into(),
                Some("The connection may have been idle too long".into()),
                true,
            ),
        };

        ErrorResponse {
            code: code.into(),
            message,
            detail: None,
            hint,
            position: None,
            severity: ErrorSeverity::Error,
            recoverable,
            actions: vec![
                ErrorAction {
                    id: "reconnect".into(),
                    label: "Reconnect".into(),
                    action_type: ErrorActionType::Reconnect,
                },
                ErrorAction {
                    id: "edit".into(),
                    label: "Edit Connection".into(),
                    action_type: ErrorActionType::EditConnection,
                },
            ],
        }
    }
}

impl QueryError {
    pub fn to_response(&self) -> ErrorResponse {
        let (code, message, hint, position) = match self {
            QueryError::Syntax { position, message } => (
                "SYNTAX_ERROR",
                message.clone(),
                Some("Check the SQL syntax".into()),
                Some(*position),
            ),
            QueryError::StatementTimeout => (
                "STATEMENT_TIMEOUT",
                "Query exceeded statement timeout".into(),
                Some("Increase statement_timeout or optimize the query".into()),
                None,
            ),
            QueryError::Cancelled => (
                "CANCELLED",
                "Query cancelled by user".into(),
                None,
                None,
            ),
            QueryError::PermissionDenied { object, operation } => (
                "PERMISSION_DENIED",
                format!("Permission denied for {} on {}", operation, object),
                Some("Check your role privileges".into()),
                None,
            ),
            QueryError::RelationNotFound(name) => (
                "RELATION_NOT_FOUND",
                format!("Relation '{}' does not exist", name),
                Some("Check the table/view name and schema".into()),
                None,
            ),
            QueryError::ColumnNotFound { column, table } => (
                "COLUMN_NOT_FOUND",
                format!("Column '{}' does not exist in '{}'", column, table),
                Some("Check the column name".into()),
                None,
            ),
            QueryError::UniqueViolation { constraint, detail } => (
                "UNIQUE_VIOLATION",
                format!("Duplicate key violates constraint '{}'", constraint),
                Some(detail.clone()),
                None,
            ),
            QueryError::ForeignKeyViolation { constraint, detail } => (
                "FOREIGN_KEY_VIOLATION",
                format!("Foreign key violation on constraint '{}'", constraint),
                Some(detail.clone()),
                None,
            ),
            QueryError::CheckViolation { constraint, detail } => (
                "CHECK_VIOLATION",
                format!("Check constraint '{}' violated", constraint),
                Some(detail.clone()),
                None,
            ),
            QueryError::NotNullViolation(column) => (
                "NOT_NULL_VIOLATION",
                format!("Column '{}' cannot be null", column),
                Some("Provide a value for this required column".into()),
                None,
            ),
            QueryError::Deadlock => (
                "DEADLOCK",
                "Deadlock detected".into(),
                Some("Retry the transaction".into()),
                None,
            ),
        };

        let severity = match self {
            QueryError::Cancelled => ErrorSeverity::Info,
            _ => ErrorSeverity::Error,
        };

        let mut actions = vec![];

        if position.is_some() {
            actions.push(ErrorAction {
                id: "view_position".into(),
                label: "Go to Error".into(),
                action_type: ErrorActionType::ViewPosition,
            });
        }

        actions.push(ErrorAction {
            id: "search".into(),
            label: "Search Error".into(),
            action_type: ErrorActionType::SearchError,
        });

        actions.push(ErrorAction {
            id: "copy".into(),
            label: "Copy Error".into(),
            action_type: ErrorActionType::CopyError,
        });

        if matches!(self, QueryError::Deadlock | QueryError::StatementTimeout) {
            actions.insert(0, ErrorAction {
                id: "retry".into(),
                label: "Retry".into(),
                action_type: ErrorActionType::Retry,
            });
        }

        ErrorResponse {
            code: code.into(),
            message,
            detail: None,
            hint,
            position,
            severity,
            recoverable: matches!(self, QueryError::Deadlock | QueryError::Cancelled),
            actions,
        }
    }
}

/// Convert tokio-postgres error to TuskError
impl From<tokio_postgres::Error> for TuskError {
    fn from(err: tokio_postgres::Error) -> Self {
        // Check if it's a database error with SQLSTATE
        if let Some(db_err) = err.as_db_error() {
            let code = db_err.code();
            let position = db_err.position().map(|p| match p {
                tokio_postgres::error::ErrorPosition::Original(pos) => *pos,
                tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position,
            });

            // Map common SQLSTATE codes to specific errors
            match code {
                &SqlState::SYNTAX_ERROR | &SqlState::SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION => {
                    return TuskError::Query(QueryError::Syntax {
                        position: position.unwrap_or(0),
                        message: db_err.message().to_string(),
                    });
                }
                &SqlState::QUERY_CANCELED => {
                    return TuskError::Query(QueryError::Cancelled);
                }
                &SqlState::INSUFFICIENT_PRIVILEGE => {
                    return TuskError::Query(QueryError::PermissionDenied {
                        object: db_err.table().unwrap_or("unknown").to_string(),
                        operation: "access".to_string(),
                    });
                }
                &SqlState::UNDEFINED_TABLE => {
                    return TuskError::Query(QueryError::RelationNotFound(
                        db_err.message().to_string()
                    ));
                }
                &SqlState::UNDEFINED_COLUMN => {
                    return TuskError::Query(QueryError::ColumnNotFound {
                        column: db_err.column().unwrap_or("unknown").to_string(),
                        table: db_err.table().unwrap_or("unknown").to_string(),
                    });
                }
                &SqlState::UNIQUE_VIOLATION => {
                    return TuskError::Query(QueryError::UniqueViolation {
                        constraint: db_err.constraint().unwrap_or("unknown").to_string(),
                        detail: db_err.detail().unwrap_or("").to_string(),
                    });
                }
                &SqlState::FOREIGN_KEY_VIOLATION => {
                    return TuskError::Query(QueryError::ForeignKeyViolation {
                        constraint: db_err.constraint().unwrap_or("unknown").to_string(),
                        detail: db_err.detail().unwrap_or("").to_string(),
                    });
                }
                &SqlState::CHECK_VIOLATION => {
                    return TuskError::Query(QueryError::CheckViolation {
                        constraint: db_err.constraint().unwrap_or("unknown").to_string(),
                        detail: db_err.detail().unwrap_or("").to_string(),
                    });
                }
                &SqlState::NOT_NULL_VIOLATION => {
                    return TuskError::Query(QueryError::NotNullViolation(
                        db_err.column().unwrap_or("unknown").to_string()
                    ));
                }
                &SqlState::T_R_DEADLOCK_DETECTED => {
                    return TuskError::Query(QueryError::Deadlock);
                }
                &SqlState::INVALID_PASSWORD => {
                    return TuskError::Authentication(db_err.message().to_string());
                }
                _ => {}
            }

            // Generic database error
            return TuskError::Database {
                message: db_err.message().to_string(),
                code: Some(code.code().to_string()),
                detail: db_err.detail().map(String::from),
                hint: db_err.hint().map(String::from),
                position,
            };
        }

        // Check for connection errors by message
        let msg = err.to_string().to_lowercase();

        if msg.contains("connection refused") {
            return TuskError::Connection(ConnectionError::ConnectionRefused {
                host: "unknown".into(),
                port: 5432,
            });
        }

        if msg.contains("timed out") || msg.contains("timeout") {
            return TuskError::Connection(ConnectionError::Timeout { timeout_secs: 30 });
        }

        if msg.contains("ssl") {
            if msg.contains("required") {
                return TuskError::Connection(ConnectionError::SslRequired);
            }
            return TuskError::Connection(ConnectionError::SslCertificate(err.to_string()));
        }

        if msg.contains("connection reset") {
            return TuskError::Connection(ConnectionError::ConnectionReset);
        }

        TuskError::Internal(err.to_string())
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, TuskError>;
```

### 28.2 Error State Management

**File: `src/state/error_state.rs`**

```rust
use crate::error::{ErrorResponse, ErrorSeverity, ErrorActionType};
use gpui::Global;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Application error with metadata
#[derive(Debug, Clone)]
pub struct AppError {
    pub id: String,
    pub response: ErrorResponse,
    pub connection_id: Option<Uuid>,
    pub query_id: Option<Uuid>,
    pub timestamp: u64,
    pub dismissed: bool,
}

impl AppError {
    pub fn new(response: ErrorResponse) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = format!("error-{}", COUNTER.fetch_add(1, Ordering::SeqCst));

        Self {
            id,
            response,
            connection_id: None,
            query_id: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            dismissed: false,
        }
    }

    pub fn with_connection(mut self, connection_id: Uuid) -> Self {
        self.connection_id = Some(connection_id);
        self
    }

    pub fn with_query(mut self, query_id: Uuid) -> Self {
        self.query_id = Some(query_id);
        self
    }
}

/// Global error state
pub struct ErrorState {
    errors: RwLock<Vec<AppError>>,
    /// Errors by connection ID for quick lookup
    by_connection: RwLock<HashMap<Uuid, Vec<String>>>,
    /// Errors by query ID for quick lookup
    by_query: RwLock<HashMap<Uuid, Vec<String>>>,
}

impl Global for ErrorState {}

impl ErrorState {
    pub fn new() -> Self {
        Self {
            errors: RwLock::new(Vec::new()),
            by_connection: RwLock::new(HashMap::new()),
            by_query: RwLock::new(HashMap::new()),
        }
    }

    /// Add a new error
    pub fn add(&self, error: AppError) -> String {
        let error_id = error.id.clone();

        // Index by connection if present
        if let Some(conn_id) = error.connection_id {
            self.by_connection
                .write()
                .entry(conn_id)
                .or_default()
                .push(error_id.clone());
        }

        // Index by query if present
        if let Some(query_id) = error.query_id {
            self.by_query
                .write()
                .entry(query_id)
                .or_default()
                .push(error_id.clone());
        }

        self.errors.write().push(error);
        error_id
    }

    /// Add error from response with context
    pub fn add_error(
        &self,
        response: ErrorResponse,
        connection_id: Option<Uuid>,
        query_id: Option<Uuid>,
    ) -> String {
        let mut error = AppError::new(response);
        error.connection_id = connection_id;
        error.query_id = query_id;
        self.add(error)
    }

    /// Get all active (non-dismissed) errors
    pub fn active_errors(&self) -> Vec<AppError> {
        self.errors
            .read()
            .iter()
            .filter(|e| !e.dismissed)
            .cloned()
            .collect()
    }

    /// Get the latest error
    pub fn latest(&self) -> Option<AppError> {
        self.errors
            .read()
            .iter()
            .filter(|e| !e.dismissed)
            .last()
            .cloned()
    }

    /// Get error by ID
    pub fn get(&self, error_id: &str) -> Option<AppError> {
        self.errors
            .read()
            .iter()
            .find(|e| e.id == error_id)
            .cloned()
    }

    /// Dismiss an error
    pub fn dismiss(&self, error_id: &str) {
        if let Some(error) = self.errors.write().iter_mut().find(|e| e.id == error_id) {
            error.dismissed = true;
        }
    }

    /// Remove an error completely
    pub fn remove(&self, error_id: &str) {
        let mut errors = self.errors.write();
        if let Some(pos) = errors.iter().position(|e| e.id == error_id) {
            let error = errors.remove(pos);

            // Clean up indexes
            if let Some(conn_id) = error.connection_id {
                if let Some(ids) = self.by_connection.write().get_mut(&conn_id) {
                    ids.retain(|id| id != error_id);
                }
            }
            if let Some(query_id) = error.query_id {
                if let Some(ids) = self.by_query.write().get_mut(&query_id) {
                    ids.retain(|id| id != error_id);
                }
            }
        }
    }

    /// Clear all errors
    pub fn clear(&self) {
        self.errors.write().clear();
        self.by_connection.write().clear();
        self.by_query.write().clear();
    }

    /// Clear errors for a specific connection
    pub fn clear_for_connection(&self, connection_id: Uuid) {
        let error_ids: Vec<String> = self.by_connection
            .write()
            .remove(&connection_id)
            .unwrap_or_default();

        let mut errors = self.errors.write();
        errors.retain(|e| !error_ids.contains(&e.id));
    }

    /// Clear errors for a specific query
    pub fn clear_for_query(&self, query_id: Uuid) {
        let error_ids: Vec<String> = self.by_query
            .write()
            .remove(&query_id)
            .unwrap_or_default();

        let mut errors = self.errors.write();
        errors.retain(|e| !error_ids.contains(&e.id));
    }

    /// Get errors for a connection
    pub fn for_connection(&self, connection_id: Uuid) -> Vec<AppError> {
        let error_ids = self.by_connection
            .read()
            .get(&connection_id)
            .cloned()
            .unwrap_or_default();

        self.errors
            .read()
            .iter()
            .filter(|e| error_ids.contains(&e.id) && !e.dismissed)
            .cloned()
            .collect()
    }

    /// Get errors for a query
    pub fn for_query(&self, query_id: Uuid) -> Vec<AppError> {
        let error_ids = self.by_query
            .read()
            .get(&query_id)
            .cloned()
            .unwrap_or_default();

        self.errors
            .read()
            .iter()
            .filter(|e| error_ids.contains(&e.id) && !e.dismissed)
            .cloned()
            .collect()
    }

    /// Count active errors by severity
    pub fn count_by_severity(&self) -> HashMap<ErrorSeverity, usize> {
        let mut counts = HashMap::new();
        for error in self.errors.read().iter().filter(|e| !e.dismissed) {
            *counts.entry(error.response.severity).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for ErrorState {
    fn default() -> Self {
        Self::new()
    }
}
```

### 28.3 Auto-Reconnect Service

**File: `src/services/reconnect.rs`**

```rust
use crate::error::{Result, TuskError, ConnectionError};
use crate::services::connection::ConnectionService;
use gpui::{App, AsyncApp, Context, EventEmitter, Model};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Reconnection configuration
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Reconnection status for UI
#[derive(Debug, Clone)]
pub enum ReconnectStatus {
    Idle,
    Attempting {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
    },
    Success {
        attempts: u32,
    },
    Failed {
        attempts: u32,
        error: String,
    },
    Exhausted {
        max_attempts: u32,
    },
}

/// Events emitted during reconnection
#[derive(Debug, Clone)]
pub enum ReconnectEvent {
    StatusChanged(Uuid, ReconnectStatus),
    ConnectionDropped(Uuid, Option<Uuid>), // connection_id, query_id
}

/// Reconnect service state
pub struct ReconnectState {
    status_by_connection: RwLock<std::collections::HashMap<Uuid, ReconnectStatus>>,
}

impl ReconnectState {
    pub fn new() -> Self {
        Self {
            status_by_connection: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn status(&self, connection_id: Uuid) -> ReconnectStatus {
        self.status_by_connection
            .read()
            .get(&connection_id)
            .cloned()
            .unwrap_or(ReconnectStatus::Idle)
    }

    pub fn set_status(&self, connection_id: Uuid, status: ReconnectStatus) {
        self.status_by_connection.write().insert(connection_id, status);
    }

    pub fn clear(&self, connection_id: Uuid) {
        self.status_by_connection.write().remove(&connection_id);
    }
}

pub struct ReconnectService {
    connection_service: Arc<ConnectionService>,
    state: Arc<ReconnectState>,
}

impl ReconnectService {
    pub fn new(connection_service: Arc<ConnectionService>) -> Self {
        Self {
            connection_service,
            state: Arc::new(ReconnectState::new()),
        }
    }

    pub fn state(&self) -> &Arc<ReconnectState> {
        &self.state
    }

    /// Attempt to reconnect with exponential backoff
    pub async fn reconnect(
        &self,
        connection_id: Uuid,
        config: ReconnectConfig,
        on_status: impl Fn(ReconnectStatus) + Send + 'static,
    ) -> Result<()> {
        let mut attempt = 0;
        let mut delay = config.initial_delay_ms;
        let mut last_error: Option<String> = None;

        while attempt < config.max_attempts {
            attempt += 1;

            // Update status
            let status = ReconnectStatus::Attempting {
                attempt,
                max_attempts: config.max_attempts,
                delay_ms: delay,
            };
            self.state.set_status(connection_id, status.clone());
            on_status(status);

            // Wait before attempting (skip first attempt)
            if attempt > 1 {
                sleep(Duration::from_millis(delay)).await;
            }

            // Try to reconnect
            match self.connection_service.reconnect(connection_id).await {
                Ok(_) => {
                    // Success!
                    let status = ReconnectStatus::Success { attempts: attempt };
                    self.state.set_status(connection_id, status.clone());
                    on_status(status);

                    // Clear status after a delay
                    tokio::spawn({
                        let state = self.state.clone();
                        async move {
                            sleep(Duration::from_millis(2000)).await;
                            state.clear(connection_id);
                        }
                    });

                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e.to_string());

                    let status = ReconnectStatus::Failed {
                        attempts: attempt,
                        error: e.to_string(),
                    };
                    self.state.set_status(connection_id, status.clone());
                    on_status(status);

                    // Check if error is recoverable
                    if !self.is_recoverable(&e) {
                        return Err(e);
                    }

                    // Increase delay with backoff
                    delay = ((delay as f64 * config.backoff_multiplier) as u64)
                        .min(config.max_delay_ms);
                }
            }
        }

        // All attempts exhausted
        let status = ReconnectStatus::Exhausted {
            max_attempts: config.max_attempts,
        };
        self.state.set_status(connection_id, status.clone());
        on_status(status);

        Err(TuskError::Connection(ConnectionError::ConnectionClosed))
    }

    /// Check if an error is potentially recoverable via reconnect
    fn is_recoverable(&self, error: &TuskError) -> bool {
        match error {
            TuskError::Connection(e) => match e {
                ConnectionError::ConnectionRefused { .. } => true,
                ConnectionError::Timeout { .. } => true,
                ConnectionError::NetworkUnreachable => true,
                ConnectionError::ConnectionReset => true,
                ConnectionError::ConnectionClosed => true,
                // Auth errors not recoverable without user action
                _ => false,
            },
            TuskError::Timeout(_) => true,
            _ => false,
        }
    }

    /// Handle a connection drop during query execution
    pub async fn handle_connection_drop(
        &self,
        connection_id: Uuid,
        query_id: Option<Uuid>,
        on_status: impl Fn(ReconnectStatus) + Send + 'static,
    ) -> Result<()> {
        // Attempt reconnection
        self.reconnect(connection_id, ReconnectConfig::default(), on_status).await
    }
}
```

### 28.4 Query Persistence Service

**File: `src/services/query_persistence.rs`**

```rust
use crate::error::Result;
use crate::services::storage::StorageService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Persisted query tab state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedQuery {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub sql: String,
    pub cursor_position: u32,
    pub file_path: Option<String>,
    pub is_dirty: bool,
    pub saved_at: u64,
}

impl PersistedQuery {
    pub fn new(id: Uuid, sql: String) -> Self {
        Self {
            id,
            connection_id: None,
            sql,
            cursor_position: 0,
            file_path: None,
            is_dirty: false,
            saved_at: Self::now(),
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

pub struct QueryPersistenceService {
    storage: Arc<StorageService>,
}

impl QueryPersistenceService {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self { storage }
    }

    /// Save a query tab to persistent storage
    pub async fn save_query(&self, query: &PersistedQuery) -> Result<()> {
        let key = format!("query_tab:{}", query.id);
        let json = serde_json::to_string(query)?;
        self.storage.set(&key, &json).await
    }

    /// Load a persisted query
    pub async fn load_query(&self, query_id: Uuid) -> Result<Option<PersistedQuery>> {
        let key = format!("query_tab:{}", query_id);
        match self.storage.get(&key).await? {
            Some(json) => {
                let query: PersistedQuery = serde_json::from_str(&json)?;
                Ok(Some(query))
            }
            None => Ok(None),
        }
    }

    /// Load all persisted queries (for session restore)
    pub async fn load_all_queries(&self) -> Result<Vec<PersistedQuery>> {
        let entries = self.storage.list_by_prefix("query_tab:").await?;
        let mut queries: Vec<PersistedQuery> = entries
            .into_iter()
            .filter_map(|(_, json)| serde_json::from_str(&json).ok())
            .collect();

        // Sort by saved_at
        queries.sort_by(|a, b| a.saved_at.cmp(&b.saved_at));

        Ok(queries)
    }

    /// Delete a persisted query
    pub async fn delete_query(&self, query_id: Uuid) -> Result<()> {
        let key = format!("query_tab:{}", query_id);
        self.storage.delete(&key).await
    }

    /// Clear all persisted queries
    pub async fn clear_all(&self) -> Result<()> {
        let entries = self.storage.list_by_prefix("query_tab:").await?;
        for (key, _) in entries {
            self.storage.delete(&key).await?;
        }
        Ok(())
    }

    /// Update cursor position only (lightweight update)
    pub async fn update_cursor(&self, query_id: Uuid, position: u32) -> Result<()> {
        if let Some(mut query) = self.load_query(query_id).await? {
            query.cursor_position = position;
            self.save_query(&query).await?;
        }
        Ok(())
    }

    /// Mark query as dirty/clean
    pub async fn set_dirty(&self, query_id: Uuid, is_dirty: bool) -> Result<()> {
        if let Some(mut query) = self.load_query(query_id).await? {
            query.is_dirty = is_dirty;
            query.saved_at = PersistedQuery::now() as u64;
            self.save_query(&query).await?;
        }
        Ok(())
    }
}
```

### 28.5 Auto-Save Manager

**File: `src/services/auto_save.rs`**

```rust
use crate::services::query_persistence::{QueryPersistenceService, PersistedQuery};
use crate::state::tab_state::TabState;
use gpui::{App, Global};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;

/// Auto-save configuration
#[derive(Debug, Clone)]
pub struct AutoSaveConfig {
    pub interval_ms: u64,
    pub enabled: bool,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5000,
            enabled: true,
        }
    }
}

/// Message to the auto-save worker
enum AutoSaveMessage {
    SaveNow,
    Stop,
}

pub struct AutoSaveManager {
    persistence: Arc<QueryPersistenceService>,
    config: RwLock<AutoSaveConfig>,
    tx: RwLock<Option<mpsc::Sender<AutoSaveMessage>>>,
}

impl AutoSaveManager {
    pub fn new(persistence: Arc<QueryPersistenceService>) -> Self {
        Self {
            persistence,
            config: RwLock::new(AutoSaveConfig::default()),
            tx: RwLock::new(None),
        }
    }

    /// Start the auto-save background task
    pub fn start(&self, tab_state: Arc<TabState>) {
        let config = self.config.read().clone();
        if !config.enabled {
            return;
        }

        let (tx, mut rx) = mpsc::channel::<AutoSaveMessage>(16);
        *self.tx.write() = Some(tx);

        let persistence = self.persistence.clone();
        let interval_ms = config.interval_ms;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(interval_ms));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Save all dirty tabs
                        Self::save_dirty_tabs(&persistence, &tab_state).await;
                    }
                    msg = rx.recv() => {
                        match msg {
                            Some(AutoSaveMessage::SaveNow) => {
                                Self::save_dirty_tabs(&persistence, &tab_state).await;
                            }
                            Some(AutoSaveMessage::Stop) | None => {
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Stop auto-save
    pub fn stop(&self) {
        if let Some(tx) = self.tx.write().take() {
            let _ = tx.blocking_send(AutoSaveMessage::Stop);
        }
    }

    /// Trigger immediate save
    pub fn save_now(&self) {
        if let Some(tx) = self.tx.read().as_ref() {
            let _ = tx.blocking_send(AutoSaveMessage::SaveNow);
        }
    }

    /// Update configuration
    pub fn set_config(&self, config: AutoSaveConfig) {
        *self.config.write() = config;
    }

    /// Save all dirty tabs
    async fn save_dirty_tabs(
        persistence: &QueryPersistenceService,
        tab_state: &TabState,
    ) {
        let tabs = tab_state.all_tabs();

        for tab in tabs {
            if tab.is_dirty {
                let query = PersistedQuery {
                    id: tab.id,
                    connection_id: tab.connection_id,
                    sql: tab.content.clone(),
                    cursor_position: tab.cursor_position,
                    file_path: tab.file_path.clone(),
                    is_dirty: tab.is_dirty,
                    saved_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };

                if let Err(e) = persistence.save_query(&query).await {
                    tracing::error!("Failed to auto-save tab {}: {}", tab.id, e);
                }
            }
        }
    }
}

impl Drop for AutoSaveManager {
    fn drop(&mut self) {
        self.stop();
    }
}
```

### 28.6 Error Display Component

**File: `src/ui/components/error_display.rs`**

```rust
use crate::error::{ErrorSeverity, ErrorActionType};
use crate::state::error_state::{AppError, ErrorState};
use crate::ui::theme::Theme;
use gpui::*;

/// Error display component
pub struct ErrorDisplay {
    error: AppError,
    compact: bool,
    on_action: Box<dyn Fn(ErrorActionType, &AppError) + Send + Sync>,
}

impl ErrorDisplay {
    pub fn new(
        error: AppError,
        on_action: impl Fn(ErrorActionType, &AppError) + Send + Sync + 'static,
    ) -> Self {
        Self {
            error,
            compact: false,
            on_action: Box::new(on_action),
        }
    }

    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    fn severity_icon(&self) -> &'static str {
        match self.error.response.severity {
            ErrorSeverity::Error => "⊘",
            ErrorSeverity::Warning => "⚠",
            ErrorSeverity::Notice => "ℹ",
            ErrorSeverity::Info => "ℹ",
        }
    }
}

impl Render for ErrorDisplay {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let severity_color = self.error.response.severity.color();

        let padding = if self.compact { px(8.0) } else { px(12.0) };
        let gap = if self.compact { px(8.0) } else { px(12.0) };
        let icon_size = if self.compact { px(16.0) } else { px(20.0) };

        div()
            .id(SharedString::from(self.error.id.clone()))
            .flex()
            .flex_row()
            .gap(gap)
            .p(padding)
            .bg(severity_color.opacity(0.1))
            .border_1()
            .border_color(severity_color.opacity(0.3))
            .border_l_3()
            .border_l_color(severity_color)
            .rounded_lg()
            .child(
                // Severity icon
                div()
                    .text_size(icon_size)
                    .text_color(severity_color)
                    .child(self.severity_icon())
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        // Header with code and dismiss
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(severity_color)
                                    .child(self.error.response.code.clone())
                            )
                            .when(!self.compact, |el| {
                                let error = self.error.clone();
                                let on_action = self.on_action.clone();
                                el.child(
                                    div()
                                        .p_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.bg_hover))
                                        .text_color(theme.text_tertiary)
                                        .on_click(move |_, cx| {
                                            on_action(ErrorActionType::Dismiss, &error);
                                        })
                                        .child("✕")
                                )
                            })
                    )
                    .child(
                        // Message
                        div()
                            .text_sm()
                            .text_color(theme.text_primary)
                            .child(self.error.response.message.clone())
                    )
                    .when(!self.compact, |el| {
                        let mut el = el;

                        // Detail
                        if let Some(detail) = &self.error.response.detail {
                            el = el.child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_secondary)
                                    .child(format!("Detail: {}", detail))
                            );
                        }

                        // Hint
                        if let Some(hint) = &self.error.response.hint {
                            el = el.child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_secondary)
                                    .child(format!("Hint: {}", hint))
                            );
                        }

                        // Position
                        if let Some(position) = self.error.response.position {
                            el = el.child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(theme.text_tertiary)
                                    .child(format!("Error at position {}", position))
                            );
                        }

                        // Actions
                        if !self.error.response.actions.is_empty() {
                            el = el.child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .mt_3()
                                    .children(
                                        self.error.response.actions.iter().map(|action| {
                                            let action_type = action.action_type;
                                            let error = self.error.clone();
                                            let on_action = self.on_action.clone();

                                            ErrorActionButton::new(
                                                action.label.clone(),
                                                action_type,
                                                move |_, _| {
                                                    on_action(action_type, &error);
                                                }
                                            )
                                        })
                                    )
                            );
                        }

                        el
                    })
            )
    }
}

/// Error action button
struct ErrorActionButton {
    label: String,
    action_type: ErrorActionType,
    on_click: Box<dyn Fn(&ClickEvent, &mut App) + Send + Sync>,
}

impl ErrorActionButton {
    fn new(
        label: String,
        action_type: ErrorActionType,
        on_click: impl Fn(&ClickEvent, &mut App) + Send + Sync + 'static,
    ) -> Self {
        Self {
            label,
            action_type,
            on_click: Box::new(on_click),
        }
    }

    fn icon(&self) -> &'static str {
        match self.action_type {
            ErrorActionType::Retry | ErrorActionType::Reconnect => "↻",
            ErrorActionType::SearchError => "🔍",
            ErrorActionType::CopyError => "📋",
            ErrorActionType::EditConnection => "✎",
            ErrorActionType::ViewPosition => "→",
            ErrorActionType::Dismiss => "✕",
        }
    }
}

impl Render for ErrorActionButton {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let on_click = self.on_click.clone();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .bg(theme.bg_secondary)
            .hover(|s| s.bg(theme.bg_hover))
            .text_sm()
            .text_color(theme.text_secondary)
            .on_click(move |event, cx| {
                on_click(event, cx);
            })
            .child(self.icon())
            .child(self.label.clone())
    }
}
```

### 28.7 Reconnect Overlay Component

**File: `src/ui/components/reconnect_overlay.rs`**

```rust
use crate::services::reconnect::{ReconnectService, ReconnectStatus, ReconnectConfig};
use crate::ui::theme::Theme;
use gpui::*;
use std::sync::Arc;
use uuid::Uuid;

/// Reconnection overlay shown when connection is lost
pub struct ReconnectOverlay {
    connection_id: Uuid,
    status: ReconnectStatus,
    reconnect_service: Arc<ReconnectService>,
    on_dismiss: Option<Box<dyn Fn() + Send + Sync>>,
}

impl ReconnectOverlay {
    pub fn new(
        connection_id: Uuid,
        reconnect_service: Arc<ReconnectService>,
    ) -> Self {
        let status = reconnect_service.state().status(connection_id);
        Self {
            connection_id,
            status,
            reconnect_service,
            on_dismiss: None,
        }
    }

    pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn update_status(&mut self, status: ReconnectStatus) {
        self.status = status;
    }

    fn is_visible(&self) -> bool {
        !matches!(self.status, ReconnectStatus::Idle)
    }
}

impl Render for ReconnectOverlay {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if !self.is_visible() {
            return div();
        }

        div()
            .absolute()
            .inset_0()
            .bg(hsla(0.0, 0.0, 0.0, 0.5))
            .flex()
            .items_center()
            .justify_center()
            .z_index(1000)
            .child(
                div()
                    .bg(theme.bg_primary)
                    .rounded_xl()
                    .p_8()
                    .text_center()
                    .max_w(px(400.0))
                    .shadow_lg()
                    .child(self.render_content(cx))
            )
    }
}

impl ReconnectOverlay {
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        match &self.status {
            ReconnectStatus::Idle => div(),

            ReconnectStatus::Attempting { attempt, max_attempts, delay_ms } => {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        // Spinning icon
                        div()
                            .text_color(theme.primary)
                            .text_size(px(32.0))
                            .with_animation(
                                "spin",
                                Animation::new(Duration::from_secs(1))
                                    .repeat()
                                    .with_easing(Ease::Linear),
                                |div, progress| {
                                    div.rotate(progress * std::f32::consts::TAU)
                                }
                            )
                            .child("↻")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Connection Lost")
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .child(format!("Attempting to reconnect... ({}/{})", attempt, max_attempts))
                    )
            }

            ReconnectStatus::Success { attempts } => {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_color(theme.success)
                            .text_size(px(32.0))
                            .child("✓")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Reconnected!")
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .child("Connection restored successfully")
                    )
            }

            ReconnectStatus::Failed { attempts, error } => {
                let connection_id = self.connection_id;
                let service = self.reconnect_service.clone();
                let on_dismiss = self.on_dismiss.clone();

                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_color(theme.error)
                            .text_size(px(32.0))
                            .child("⊘")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Reconnection Failed")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.error)
                            .child(error.clone())
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .mt_4()
                            .child(
                                Self::button("Dismiss", theme, false, move |_, _| {
                                    if let Some(on_dismiss) = &on_dismiss {
                                        on_dismiss();
                                    }
                                })
                            )
                            .child(
                                Self::button("Try Again", theme, true, move |_, cx| {
                                    cx.spawn(|_, _| async move {
                                        let _ = service.reconnect(
                                            connection_id,
                                            ReconnectConfig { max_attempts: 1, ..Default::default() },
                                            |_| {}
                                        ).await;
                                    }).detach();
                                })
                            )
                    )
            }

            ReconnectStatus::Exhausted { max_attempts } => {
                let on_dismiss = self.on_dismiss.clone();
                let connection_id = self.connection_id;
                let service = self.reconnect_service.clone();

                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_color(theme.error)
                            .text_size(px(32.0))
                            .child("✕")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Connection Failed")
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .child(format!("Unable to reconnect after {} attempts", max_attempts))
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .mt_4()
                            .child(
                                Self::button("Dismiss", theme, false, move |_, _| {
                                    if let Some(on_dismiss) = &on_dismiss {
                                        on_dismiss();
                                    }
                                })
                            )
                            .child(
                                Self::button("Try Again", theme, true, move |_, cx| {
                                    cx.spawn(|_, _| async move {
                                        let _ = service.reconnect(
                                            connection_id,
                                            ReconnectConfig::default(),
                                            |_| {}
                                        ).await;
                                    }).detach();
                                })
                            )
                    )
            }
        }
    }

    fn button(
        label: &str,
        theme: &Theme,
        primary: bool,
        on_click: impl Fn(&ClickEvent, &mut App) + Send + Sync + 'static,
    ) -> impl IntoElement {
        let (bg, text) = if primary {
            (theme.primary, theme.text_on_primary)
        } else {
            (theme.bg_secondary, theme.text_secondary)
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_4()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .bg(bg)
            .hover(|s| s.opacity(0.9))
            .text_color(text)
            .on_click(on_click)
            .child(label)
    }
}
```

### 28.8 Editor Error Highlighting

**File: `src/ui/editor/error_highlight.rs`**

```rust
use crate::error::ErrorResponse;
use crate::ui::theme::Theme;
use gpui::*;
use std::ops::Range;

/// Error highlight information for the editor
#[derive(Debug, Clone)]
pub struct EditorError {
    pub position: u32,
    pub message: String,
    pub code: String,
    /// Calculated line and column from position
    pub line: usize,
    pub column: usize,
    /// Character range for highlighting
    pub range: Range<usize>,
}

impl EditorError {
    /// Create error highlight from error response and source text
    pub fn from_response(response: &ErrorResponse, source: &str) -> Option<Self> {
        let position = response.position?;
        let (line, column) = Self::position_to_line_col(source, position as usize);
        let range = Self::find_token_range(source, position as usize);

        Some(Self {
            position,
            message: response.message.clone(),
            code: response.code.clone(),
            line,
            column,
            range,
        })
    }

    /// Convert character offset to line/column (1-based)
    fn position_to_line_col(source: &str, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in source.chars().enumerate() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Find the word/token range at the given position
    fn find_token_range(source: &str, offset: usize) -> Range<usize> {
        let bytes = source.as_bytes();
        let len = bytes.len();

        if offset >= len {
            return offset.saturating_sub(1)..offset;
        }

        // Find start of token
        let mut start = offset;
        while start > 0 {
            let ch = bytes[start - 1] as char;
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            start -= 1;
        }

        // Find end of token
        let mut end = offset;
        while end < len {
            let ch = bytes[end] as char;
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            end += 1;
        }

        // Ensure we have at least one character
        if start == end {
            end = (start + 1).min(len);
        }

        start..end
    }
}

/// Error highlight decoration for rendering
pub struct ErrorHighlight {
    pub error: EditorError,
    pub hover_visible: bool,
}

impl ErrorHighlight {
    pub fn new(error: EditorError) -> Self {
        Self {
            error,
            hover_visible: false,
        }
    }
}

impl Render for ErrorHighlight {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let error_color = hsla(0.0, 0.7, 0.5, 1.0);

        div()
            .relative()
            // Wavy underline effect using gradient
            .border_b_2()
            .border_b_color(error_color)
            // Light background tint
            .bg(error_color.opacity(0.15))
            // Hover tooltip
            .when(self.hover_visible, |el| {
                el.child(
                    div()
                        .absolute()
                        .bottom_full()
                        .left_0()
                        .mb_1()
                        .z_index(100)
                        .bg(theme.bg_elevated)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .shadow_md()
                        .p_2()
                        .max_w(px(400.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .font_weight(FontWeight::BOLD)
                                        .text_sm()
                                        .text_color(error_color)
                                        .child(self.error.code.clone())
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.text_primary)
                                        .child(self.error.message.clone())
                                )
                        )
                )
            })
    }
}

/// Error line highlight (full line background)
pub struct ErrorLineHighlight {
    pub line: usize,
}

impl Render for ErrorLineHighlight {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let error_color = hsla(0.0, 0.7, 0.5, 1.0);

        div()
            .w_full()
            .bg(error_color.opacity(0.1))
    }
}

/// Error gutter marker
pub struct ErrorGutterMarker {
    pub line: usize,
    pub error: EditorError,
}

impl Render for ErrorGutterMarker {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let error_color = hsla(0.0, 0.7, 0.5, 1.0);

        div()
            .size_2()
            .rounded_full()
            .bg(error_color)
            .cursor_pointer()
            .hover(|s| s.bg(error_color.opacity(0.8)))
    }
}
```

### 28.9 Error Action Handlers

**File: `src/ui/handlers/error_actions.rs`**

```rust
use crate::error::ErrorActionType;
use crate::state::error_state::{AppError, ErrorState};
use crate::state::tab_state::TabState;
use crate::services::connection::ConnectionService;
use crate::services::reconnect::ReconnectService;
use crate::ui::dialogs::connection_dialog::ConnectionDialog;
use gpui::*;
use std::sync::Arc;

/// Handle error action button clicks
pub struct ErrorActionHandler {
    error_state: Arc<ErrorState>,
    tab_state: Arc<TabState>,
    connection_service: Arc<ConnectionService>,
    reconnect_service: Arc<ReconnectService>,
}

impl ErrorActionHandler {
    pub fn new(
        error_state: Arc<ErrorState>,
        tab_state: Arc<TabState>,
        connection_service: Arc<ConnectionService>,
        reconnect_service: Arc<ReconnectService>,
    ) -> Self {
        Self {
            error_state,
            tab_state,
            connection_service,
            reconnect_service,
        }
    }

    /// Handle an error action
    pub fn handle(&self, action: ErrorActionType, error: &AppError, cx: &mut App) {
        match action {
            ErrorActionType::Retry => {
                self.handle_retry(error, cx);
            }
            ErrorActionType::Reconnect => {
                self.handle_reconnect(error, cx);
            }
            ErrorActionType::SearchError => {
                self.handle_search_error(error, cx);
            }
            ErrorActionType::CopyError => {
                self.handle_copy_error(error, cx);
            }
            ErrorActionType::EditConnection => {
                self.handle_edit_connection(error, cx);
            }
            ErrorActionType::ViewPosition => {
                self.handle_view_position(error, cx);
            }
            ErrorActionType::Dismiss => {
                self.error_state.dismiss(&error.id);
            }
        }
    }

    fn handle_retry(&self, error: &AppError, cx: &mut App) {
        if let Some(query_id) = error.query_id {
            // Get the tab and re-execute
            if let Some(tab) = self.tab_state.get_tab(query_id) {
                // Emit retry event
                // The query execution system will handle re-running
                cx.emit_global(QueryRetryEvent { query_id });
            }
        }
        self.error_state.dismiss(&error.id);
    }

    fn handle_reconnect(&self, error: &AppError, cx: &mut App) {
        if let Some(connection_id) = error.connection_id {
            let service = self.reconnect_service.clone();
            cx.spawn(|_, _| async move {
                let _ = service.reconnect(
                    connection_id,
                    crate::services::reconnect::ReconnectConfig::default(),
                    |_| {}
                ).await;
            }).detach();
        }
        self.error_state.dismiss(&error.id);
    }

    fn handle_search_error(&self, error: &AppError, cx: &mut App) {
        let query = format!(
            "postgres {} {}",
            error.response.code,
            error.response.message
        );
        let encoded = urlencoding::encode(&query);
        let url = format!("https://www.google.com/search?q={}", encoded);

        // Open in default browser
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&url).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        }
    }

    fn handle_copy_error(&self, error: &AppError, cx: &mut App) {
        let mut text = format!("Error: {}\n{}", error.response.code, error.response.message);

        if let Some(detail) = &error.response.detail {
            text.push_str(&format!("\nDetail: {}", detail));
        }
        if let Some(hint) = &error.response.hint {
            text.push_str(&format!("\nHint: {}", hint));
        }
        if let Some(position) = error.response.position {
            text.push_str(&format!("\nPosition: {}", position));
        }

        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn handle_edit_connection(&self, error: &AppError, cx: &mut App) {
        if let Some(connection_id) = error.connection_id {
            // Open connection dialog in edit mode
            cx.emit_global(OpenConnectionDialogEvent {
                connection_id: Some(connection_id),
                mode: ConnectionDialogMode::Edit,
            });
        }
        self.error_state.dismiss(&error.id);
    }

    fn handle_view_position(&self, error: &AppError, cx: &mut App) {
        if let Some(position) = error.response.position {
            if let Some(query_id) = error.query_id {
                // Navigate editor to error position
                cx.emit_global(NavigateToPositionEvent {
                    query_id,
                    position,
                });
            }
        }
    }
}

/// Events for error actions
#[derive(Debug, Clone)]
pub struct QueryRetryEvent {
    pub query_id: uuid::Uuid,
}

impl EventEmitter<QueryRetryEvent> for GlobalEvents {}

#[derive(Debug, Clone)]
pub struct OpenConnectionDialogEvent {
    pub connection_id: Option<uuid::Uuid>,
    pub mode: ConnectionDialogMode,
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionDialogMode {
    Create,
    Edit,
}

impl EventEmitter<OpenConnectionDialogEvent> for GlobalEvents {}

#[derive(Debug, Clone)]
pub struct NavigateToPositionEvent {
    pub query_id: uuid::Uuid,
    pub position: u32,
}

impl EventEmitter<NavigateToPositionEvent> for GlobalEvents {}

/// Global event emitter marker
pub struct GlobalEvents;
```

### 28.10 Error Toast Component

**File: `src/ui/components/error_toast.rs`**

```rust
use crate::error::ErrorSeverity;
use crate::state::error_state::{AppError, ErrorState};
use crate::ui::theme::Theme;
use gpui::*;
use std::sync::Arc;
use std::time::Duration;

/// Toast notification for errors
pub struct ErrorToast {
    error: AppError,
    auto_dismiss_delay: Option<Duration>,
    on_dismiss: Option<Box<dyn Fn() + Send + Sync>>,
    on_click: Option<Box<dyn Fn() + Send + Sync>>,
}

impl ErrorToast {
    pub fn new(error: AppError) -> Self {
        Self {
            error,
            auto_dismiss_delay: Some(Duration::from_secs(5)),
            on_dismiss: None,
            on_click: None,
        }
    }

    pub fn auto_dismiss(mut self, delay: Option<Duration>) -> Self {
        self.auto_dismiss_delay = delay;
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn on_click(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Render for ErrorToast {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let severity_color = self.error.response.severity.color();

        // Set up auto-dismiss timer
        if let Some(delay) = self.auto_dismiss_delay {
            let on_dismiss = self.on_dismiss.clone();
            cx.spawn(|_, _| async move {
                smol::Timer::after(delay).await;
                if let Some(dismiss) = on_dismiss {
                    dismiss();
                }
            }).detach();
        }

        let on_click = self.on_click.clone();
        let on_dismiss = self.on_dismiss.clone();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .bg(theme.bg_elevated)
            .border_1()
            .border_color(theme.border)
            .border_l_3()
            .border_l_color(severity_color)
            .rounded_lg()
            .shadow_lg()
            .cursor_pointer()
            .on_click(move |_, _| {
                if let Some(on_click) = &on_click {
                    on_click();
                }
            })
            .child(
                // Icon
                div()
                    .text_color(severity_color)
                    .child(match self.error.response.severity {
                        ErrorSeverity::Error => "⊘",
                        ErrorSeverity::Warning => "⚠",
                        ErrorSeverity::Notice | ErrorSeverity::Info => "ℹ",
                    })
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_sm()
                            .text_color(theme.text_primary)
                            .truncate()
                            .child(self.error.response.message.clone())
                    )
            )
            .child(
                // Dismiss button
                div()
                    .p_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_hover))
                    .text_color(theme.text_tertiary)
                    .on_click(move |_, _| {
                        if let Some(dismiss) = &on_dismiss {
                            dismiss();
                        }
                    })
                    .child("✕")
            )
    }
}

/// Container for displaying toast notifications
pub struct ToastContainer {
    error_state: Arc<ErrorState>,
    max_visible: usize,
}

impl ToastContainer {
    pub fn new(error_state: Arc<ErrorState>) -> Self {
        Self {
            error_state,
            max_visible: 3,
        }
    }
}

impl Render for ToastContainer {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = self.error_state.active_errors();
        let visible_errors: Vec<_> = errors
            .into_iter()
            .rev()
            .take(self.max_visible)
            .collect();

        div()
            .absolute()
            .bottom_4()
            .right_4()
            .flex()
            .flex_col()
            .gap_2()
            .z_index(1000)
            .children(visible_errors.into_iter().map(|error| {
                let error_id = error.id.clone();
                let state = self.error_state.clone();

                ErrorToast::new(error)
                    .on_dismiss(move || {
                        state.dismiss(&error_id);
                    })
            }))
    }
}
```

## Acceptance Criteria

1. **Error Display**
   - [ ] User-friendly error messages
   - [ ] Postgres error code displayed
   - [ ] Detail and hint shown when available
   - [ ] Error position highlighted in editor

2. **Error Actions**
   - [ ] Retry query action
   - [ ] Reconnect action
   - [ ] Web search for error
   - [ ] Copy error to clipboard
   - [ ] Edit connection settings
   - [ ] Jump to error position

3. **Connection Recovery**
   - [ ] Auto-reconnect with exponential backoff
   - [ ] Visual reconnect status overlay
   - [ ] Manual reconnect option
   - [ ] Progress indicator during reconnection

4. **Query Persistence**
   - [ ] Auto-save query tabs to SQLite
   - [ ] Restore queries on startup
   - [ ] Never lose unsaved work
   - [ ] Track dirty state

5. **Error Severity**
   - [ ] Error (red) for failures
   - [ ] Warning (orange) for alerts
   - [ ] Notice (blue) for info
   - [ ] Info (gray) for informational

6. **Editor Integration**
   - [ ] Error position highlighting
   - [ ] Underline at error location
   - [ ] Hover tooltip with error details
   - [ ] Line highlight for error line
   - [ ] Gutter marker for error

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_position_to_line_col() {
        let source = "SELECT *\nFROM users\nWHERE id = 1";

        // Position 0 (S) -> line 1, col 1
        assert_eq!(EditorError::position_to_line_col(source, 0), (1, 1));

        // Position 9 (F) -> line 2, col 1
        assert_eq!(EditorError::position_to_line_col(source, 9), (2, 1));

        // Position 20 (W) -> line 3, col 1
        assert_eq!(EditorError::position_to_line_col(source, 20), (3, 1));
    }

    #[test]
    fn test_error_token_range() {
        let source = "SELECT * FROM users WHERE id = 1";

        // Position at "FROM" (14)
        let range = EditorError::find_token_range(source, 14);
        assert_eq!(&source[range.clone()], "FROM");

        // Position at "users" (19)
        let range = EditorError::find_token_range(source, 19);
        assert_eq!(&source[range], "users");
    }

    #[test]
    fn test_connection_error_response() {
        let error = ConnectionError::ConnectionRefused {
            host: "localhost".into(),
            port: 5432,
        };
        let response = error.to_response();

        assert_eq!(response.code, "ECONNREFUSED");
        assert!(response.recoverable);
        assert!(response.actions.iter().any(|a| a.action_type == ErrorActionType::Reconnect));
    }

    #[test]
    fn test_query_error_with_position() {
        let error = QueryError::Syntax {
            position: 42,
            message: "syntax error at or near \"FORM\"".into(),
        };
        let response = error.to_response();

        assert_eq!(response.code, "SYNTAX_ERROR");
        assert_eq!(response.position, Some(42));
        assert!(response.actions.iter().any(|a| a.action_type == ErrorActionType::ViewPosition));
    }

    #[test]
    fn test_error_state_management() {
        let state = ErrorState::new();
        let conn_id = uuid::Uuid::new_v4();

        let error = AppError::new(ErrorResponse {
            code: "TEST".into(),
            message: "Test error".into(),
            detail: None,
            hint: None,
            position: None,
            severity: ErrorSeverity::Error,
            recoverable: false,
            actions: vec![],
        }).with_connection(conn_id);

        let error_id = state.add(error);

        assert_eq!(state.active_errors().len(), 1);
        assert_eq!(state.for_connection(conn_id).len(), 1);

        state.dismiss(&error_id);
        assert_eq!(state.active_errors().len(), 0);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_error_display_renders(cx: &mut TestAppContext) {
        let error = AppError::new(ErrorResponse {
            code: "42P01".into(),
            message: "relation \"users\" does not exist".into(),
            detail: None,
            hint: Some("Check the table name".into()),
            position: Some(15),
            severity: ErrorSeverity::Error,
            recoverable: false,
            actions: vec![
                ErrorAction {
                    id: "search".into(),
                    label: "Search".into(),
                    action_type: ErrorActionType::SearchError,
                },
            ],
        });

        cx.update(|cx| {
            let view = cx.new_view(|_| {
                ErrorDisplay::new(error, |_, _| {})
            });

            // Render and verify
            let element = view.render(cx);
            // Assert error code, message, hint are present
        });
    }

    #[gpui::test]
    async fn test_reconnect_overlay_states(cx: &mut TestAppContext) {
        let connection_id = uuid::Uuid::new_v4();

        // Test each state
        for status in [
            ReconnectStatus::Attempting { attempt: 1, max_attempts: 5, delay_ms: 1000 },
            ReconnectStatus::Success { attempts: 2 },
            ReconnectStatus::Failed { attempts: 3, error: "Connection refused".into() },
            ReconnectStatus::Exhausted { max_attempts: 5 },
        ] {
            cx.update(|cx| {
                // Verify overlay renders correctly for each state
            });
        }
    }
}
```

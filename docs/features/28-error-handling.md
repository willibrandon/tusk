# Feature 28: Error Handling and Recovery

## Overview

This feature implements comprehensive error handling across the application. It covers connection errors, query errors, and recovery mechanisms. Errors are displayed with user-friendly messages, technical details for debugging, actionable hints, and position highlighting for query syntax errors. The system provides auto-reconnect functionality and ensures unsaved queries are never lost.

## Goals

1. Provide user-friendly error messages with actionable hints
2. Display Postgres error details (SQLSTATE, position, detail, hint)
3. Highlight error positions in the query editor
4. Implement auto-reconnect with retry UI
5. Persist unsaved queries to prevent data loss
6. Enable quick actions (Google error, copy, retry)
7. Graceful handling of connection drops mid-query
8. Consistent error structure across all operations

## Dependencies

- Feature 01: Project Setup (Tauri + Svelte)
- Feature 02: Local Storage (query persistence)
- Feature 07: Connection Management (reconnection)
- Feature 11: Query Editor (error highlighting)

## Technical Specification

### 28.1 Error Types and Structure

**File: `src-tauri/src/error.rs`**

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
#[derive(Error, Debug)]
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
#[derive(Error, Debug)]
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

/// Serializable error for frontend
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Error,
    Warning,
    Notice,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAction {
    pub id: String,
    pub label: String,
    pub action_type: ErrorActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorActionType {
    Retry,
    Reconnect,
    GoogleError,
    CopyError,
    EditConnection,
    ViewPosition,
    Dismiss,
}

impl TuskError {
    /// Convert to frontend-friendly response
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
                            id: "google".into(),
                            label: "Search Error".into(),
                            action_type: ErrorActionType::GoogleError,
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
            id: "google".into(),
            label: "Search Error".into(),
            action_type: ErrorActionType::GoogleError,
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

        // Check for connection errors
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

/// Serialize error for Tauri commands
impl Serialize for TuskError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_response().serialize(serializer)
    }
}
```

### 28.2 Auto-Reconnect Service

**File: `src-tauri/src/services/reconnect.rs`**

```rust
use crate::services::connection::ConnectionService;
use crate::error::{Result, TuskError, ConnectionError};
use std::time::Duration;
use tokio::time::sleep;
use tauri::AppHandle;

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

pub struct ReconnectService {
    connection_service: ConnectionService,
    app: AppHandle,
}

impl ReconnectService {
    pub fn new(connection_service: ConnectionService, app: AppHandle) -> Self {
        Self { connection_service, app }
    }

    /// Attempt to reconnect with exponential backoff
    pub async fn reconnect(
        &self,
        connection_id: &str,
        config: ReconnectConfig,
    ) -> Result<()> {
        let mut attempt = 0;
        let mut delay = config.initial_delay_ms;

        while attempt < config.max_attempts {
            attempt += 1;

            // Emit attempt event
            self.app.emit("reconnect:attempt", serde_json::json!({
                "connection_id": connection_id,
                "attempt": attempt,
                "max_attempts": config.max_attempts,
                "delay_ms": delay,
            })).ok();

            // Wait before attempting
            if attempt > 1 {
                sleep(Duration::from_millis(delay)).await;
            }

            // Try to reconnect
            match self.connection_service.connect(connection_id).await {
                Ok(_) => {
                    // Success!
                    self.app.emit("reconnect:success", serde_json::json!({
                        "connection_id": connection_id,
                        "attempts": attempt,
                    })).ok();

                    return Ok(());
                }
                Err(e) => {
                    // Emit failure event
                    self.app.emit("reconnect:failed", serde_json::json!({
                        "connection_id": connection_id,
                        "attempt": attempt,
                        "error": e.to_string(),
                    })).ok();

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
        self.app.emit("reconnect:exhausted", serde_json::json!({
            "connection_id": connection_id,
            "attempts": config.max_attempts,
        })).ok();

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
        connection_id: &str,
        query_id: &str,
    ) -> Result<()> {
        // Emit connection drop event
        self.app.emit("connection:dropped", serde_json::json!({
            "connection_id": connection_id,
            "query_id": query_id,
        })).ok();

        // Attempt reconnection
        self.reconnect(connection_id, ReconnectConfig::default()).await
    }
}
```

### 28.3 Query Persistence (Auto-Save)

**File: `src-tauri/src/services/query_persistence.rs`**

```rust
use crate::services::storage::StorageService;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedQuery {
    pub id: String,
    pub connection_id: Option<String>,
    pub sql: String,
    pub cursor_position: u32,
    pub file_path: Option<String>,
    pub is_dirty: bool,
    pub saved_at: u64,
}

pub struct QueryPersistenceService {
    storage: StorageService,
}

impl QueryPersistenceService {
    pub fn new(storage: StorageService) -> Self {
        Self { storage }
    }

    /// Save a query tab to persistent storage
    pub async fn save_query(&self, query: &PersistedQuery) -> Result<()> {
        let key = format!("query_tab:{}", query.id);
        let json = serde_json::to_string(query)?;
        self.storage.set(&key, &json).await
    }

    /// Load a persisted query
    pub async fn load_query(&self, query_id: &str) -> Result<Option<PersistedQuery>> {
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
    pub async fn delete_query(&self, query_id: &str) -> Result<()> {
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

    /// Get current timestamp
    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
```

### 28.4 Tauri Commands

**File: `src-tauri/src/commands/error.rs`**

```rust
use crate::error::ErrorResponse;
use crate::services::reconnect::{ReconnectService, ReconnectConfig};
use crate::state::AppState;
use tauri::State;

/// Attempt to reconnect to a connection
#[tauri::command]
pub async fn reconnect(
    state: State<'_, AppState>,
    connection_id: String,
    max_attempts: Option<u32>,
) -> Result<(), ErrorResponse> {
    let config = ReconnectConfig {
        max_attempts: max_attempts.unwrap_or(5),
        ..Default::default()
    };

    let service = state.reconnect_service.lock().await;
    service.reconnect(&connection_id, config).await
        .map_err(|e| e.to_response())
}
```

**File: `src-tauri/src/commands/query_persistence.rs`**

```rust
use crate::services::query_persistence::{QueryPersistenceService, PersistedQuery};
use crate::state::AppState;
use crate::error::Result;
use tauri::State;

/// Save query tab
#[tauri::command]
pub async fn persist_query(
    state: State<'_, AppState>,
    query: PersistedQuery,
) -> Result<()> {
    let service = state.query_persistence.lock().await;
    service.save_query(&query).await
}

/// Load all persisted queries
#[tauri::command]
pub async fn load_persisted_queries(
    state: State<'_, AppState>,
) -> Result<Vec<PersistedQuery>> {
    let service = state.query_persistence.lock().await;
    service.load_all_queries().await
}

/// Delete persisted query
#[tauri::command]
pub async fn delete_persisted_query(
    state: State<'_, AppState>,
    query_id: String,
) -> Result<()> {
    let service = state.query_persistence.lock().await;
    service.delete_query(&query_id).await
}

/// Clear all persisted queries
#[tauri::command]
pub async fn clear_persisted_queries(
    state: State<'_, AppState>,
) -> Result<()> {
    let service = state.query_persistence.lock().await;
    service.clear_all().await
}
```

### 28.5 Svelte Frontend

#### Error Store

**File: `src/lib/stores/error.ts`**

```typescript
import { writable, derived } from 'svelte/store';

export type ErrorSeverity = 'error' | 'warning' | 'notice' | 'info';

export type ErrorActionType =
	| 'retry'
	| 'reconnect'
	| 'google_error'
	| 'copy_error'
	| 'edit_connection'
	| 'view_position'
	| 'dismiss';

export interface ErrorAction {
	id: string;
	label: string;
	action_type: ErrorActionType;
}

export interface ErrorResponse {
	code: string;
	message: string;
	detail: string | null;
	hint: string | null;
	position: number | null;
	severity: ErrorSeverity;
	recoverable: boolean;
	actions: ErrorAction[];
}

export interface AppError extends ErrorResponse {
	id: string;
	timestamp: number;
	connectionId?: string;
	queryId?: string;
	dismissed: boolean;
}

function createErrorStore() {
	const { subscribe, update, set } = writable<AppError[]>([]);

	let errorId = 0;

	return {
		subscribe,

		add(error: ErrorResponse, context?: { connectionId?: string; queryId?: string }) {
			const appError: AppError = {
				...error,
				id: `error-${++errorId}`,
				timestamp: Date.now(),
				connectionId: context?.connectionId,
				queryId: context?.queryId,
				dismissed: false
			};

			update((errors) => [...errors, appError]);
			return appError.id;
		},

		dismiss(errorId: string) {
			update((errors) => errors.map((e) => (e.id === errorId ? { ...e, dismissed: true } : e)));
		},

		remove(errorId: string) {
			update((errors) => errors.filter((e) => e.id !== errorId));
		},

		clear() {
			set([]);
		},

		clearForConnection(connectionId: string) {
			update((errors) => errors.filter((e) => e.connectionId !== connectionId));
		},

		clearForQuery(queryId: string) {
			update((errors) => errors.filter((e) => e.queryId !== queryId));
		}
	};
}

export const errorStore = createErrorStore();

// Active (non-dismissed) errors
export const activeErrors = derived(errorStore, ($errors) => $errors.filter((e) => !e.dismissed));

// Latest error
export const latestError = derived(activeErrors, ($errors) =>
	$errors.length > 0 ? $errors[$errors.length - 1] : null
);
```

#### Error Display Component

**File: `src/lib/components/error/ErrorDisplay.svelte`**

```svelte
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { open } from '@tauri-apps/plugin-shell';
	import { errorStore, type AppError, type ErrorActionType } from '$lib/stores/error';
	import { connectionStore } from '$lib/stores/connection';
	import { queryStore } from '$lib/stores/query';
	import { dialogStore } from '$lib/stores/dialog';
	import {
		AlertCircle,
		AlertTriangle,
		Info,
		X,
		ExternalLink,
		Copy,
		RefreshCw,
		Edit
	} from 'lucide-svelte';
	import Button from '$lib/components/common/Button.svelte';

	export let error: AppError;
	export let compact = false;

	const severityIcons = {
		error: AlertCircle,
		warning: AlertTriangle,
		notice: Info,
		info: Info
	};

	const severityColors = {
		error: 'var(--error-color)',
		warning: 'var(--warning-color)',
		notice: 'var(--info-color)',
		info: 'var(--text-secondary)'
	};

	$: Icon = severityIcons[error.severity];
	$: color = severityColors[error.severity];

	async function handleAction(actionType: ErrorActionType) {
		switch (actionType) {
			case 'retry':
				if (error.queryId) {
					queryStore.retry(error.queryId);
				}
				errorStore.dismiss(error.id);
				break;

			case 'reconnect':
				if (error.connectionId) {
					try {
						await invoke('reconnect', { connectionId: error.connectionId });
						errorStore.dismiss(error.id);
					} catch (e) {
						// Reconnect failed, error will be shown
					}
				}
				break;

			case 'google_error':
				const query = encodeURIComponent(`postgres ${error.code} ${error.message}`);
				await open(`https://www.google.com/search?q=${query}`);
				break;

			case 'copy_error':
				const errorText = formatErrorForCopy(error);
				await writeText(errorText);
				break;

			case 'edit_connection':
				if (error.connectionId) {
					dialogStore.open('connection', { connectionId: error.connectionId, mode: 'edit' });
				}
				errorStore.dismiss(error.id);
				break;

			case 'view_position':
				if (error.position !== null && error.queryId) {
					queryStore.setCursorPosition(error.queryId, error.position);
				}
				break;

			case 'dismiss':
				errorStore.dismiss(error.id);
				break;
		}
	}

	function formatErrorForCopy(error: AppError): string {
		let text = `Error: ${error.code}\n${error.message}`;
		if (error.detail) text += `\nDetail: ${error.detail}`;
		if (error.hint) text += `\nHint: ${error.hint}`;
		if (error.position !== null) text += `\nPosition: ${error.position}`;
		return text;
	}

	function dismiss() {
		errorStore.dismiss(error.id);
	}
</script>

<div
	class="error-display"
	class:compact
	class:error={error.severity === 'error'}
	class:warning={error.severity === 'warning'}
	style="--severity-color: {color}"
>
	<div class="error-icon">
		<svelte:component this={Icon} size={compact ? 16 : 20} />
	</div>

	<div class="error-content">
		<div class="error-header">
			<span class="error-code">{error.code}</span>
			{#if !compact}
				<button class="dismiss-btn" on:click={dismiss} title="Dismiss">
					<X size={14} />
				</button>
			{/if}
		</div>

		<p class="error-message">{error.message}</p>

		{#if !compact}
			{#if error.detail}
				<p class="error-detail">
					<strong>Detail:</strong>
					{error.detail}
				</p>
			{/if}

			{#if error.hint}
				<p class="error-hint">
					<strong>Hint:</strong>
					{error.hint}
				</p>
			{/if}

			{#if error.position !== null}
				<p class="error-position">
					Error at position {error.position}
				</p>
			{/if}

			{#if error.actions.length > 0}
				<div class="error-actions">
					{#each error.actions as action}
						<Button variant="ghost" size="sm" on:click={() => handleAction(action.action_type)}>
							{#if action.action_type === 'retry' || action.action_type === 'reconnect'}
								<RefreshCw size={14} />
							{:else if action.action_type === 'google_error'}
								<ExternalLink size={14} />
							{:else if action.action_type === 'copy_error'}
								<Copy size={14} />
							{:else if action.action_type === 'edit_connection'}
								<Edit size={14} />
							{/if}
							{action.label}
						</Button>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.error-display {
		display: flex;
		gap: 12px;
		padding: 12px 16px;
		background: color-mix(in srgb, var(--severity-color) 10%, var(--bg-primary));
		border: 1px solid color-mix(in srgb, var(--severity-color) 30%, transparent);
		border-radius: 8px;
		border-left: 3px solid var(--severity-color);
	}

	.error-display.compact {
		padding: 8px 12px;
		gap: 8px;
	}

	.error-icon {
		color: var(--severity-color);
		flex-shrink: 0;
	}

	.error-content {
		flex: 1;
		min-width: 0;
	}

	.error-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 4px;
	}

	.error-code {
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: 600;
		color: var(--severity-color);
	}

	.dismiss-btn {
		padding: 4px;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text-tertiary);
		border-radius: 4px;
	}

	.dismiss-btn:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.error-message {
		font-size: 14px;
		color: var(--text-primary);
		margin-bottom: 8px;
	}

	.compact .error-message {
		font-size: 13px;
		margin-bottom: 0;
	}

	.error-detail,
	.error-hint {
		font-size: 13px;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}

	.error-detail strong,
	.error-hint strong {
		color: var(--text-primary);
	}

	.error-position {
		font-size: 12px;
		color: var(--text-tertiary);
		font-family: var(--font-mono);
		margin-bottom: 8px;
	}

	.error-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 12px;
	}
</style>
```

#### Reconnect Overlay

**File: `src/lib/components/error/ReconnectOverlay.svelte`**

```svelte
<script lang="ts">
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import { Loader, WifiOff, RefreshCw, X } from 'lucide-svelte';
	import Button from '$lib/components/common/Button.svelte';

	export let connectionId: string;

	let visible = false;
	let attempt = 0;
	let maxAttempts = 5;
	let status: 'attempting' | 'failed' | 'success' = 'attempting';
	let errorMessage: string | null = null;

	const unlisten: (() => void)[] = [];

	onMount(async () => {
		unlisten.push(
			await listen<{ connection_id: string }>('connection:dropped', (event) => {
				if (event.payload.connection_id === connectionId) {
					visible = true;
					status = 'attempting';
					attempt = 0;
				}
			})
		);

		unlisten.push(
			await listen<{ connection_id: string; attempt: number; max_attempts: number }>(
				'reconnect:attempt',
				(event) => {
					if (event.payload.connection_id === connectionId) {
						attempt = event.payload.attempt;
						maxAttempts = event.payload.max_attempts;
						status = 'attempting';
					}
				}
			)
		);

		unlisten.push(
			await listen<{ connection_id: string; error: string }>('reconnect:failed', (event) => {
				if (event.payload.connection_id === connectionId) {
					errorMessage = event.payload.error;
				}
			})
		);

		unlisten.push(
			await listen<{ connection_id: string }>('reconnect:success', (event) => {
				if (event.payload.connection_id === connectionId) {
					status = 'success';
					setTimeout(() => {
						visible = false;
					}, 1500);
				}
			})
		);

		unlisten.push(
			await listen<{ connection_id: string }>('reconnect:exhausted', (event) => {
				if (event.payload.connection_id === connectionId) {
					status = 'failed';
				}
			})
		);
	});

	onDestroy(() => {
		unlisten.forEach((fn) => fn());
	});

	async function manualReconnect() {
		status = 'attempting';
		attempt = 0;
		errorMessage = null;

		try {
			await invoke('reconnect', { connectionId, maxAttempts: 1 });
		} catch (e) {
			status = 'failed';
			errorMessage = e instanceof Error ? e.message : String(e);
		}
	}

	function dismiss() {
		visible = false;
	}
</script>

{#if visible}
	<div class="reconnect-overlay">
		<div class="reconnect-card">
			{#if status === 'attempting'}
				<div class="reconnect-icon attempting">
					<Loader size={32} class="spin" />
				</div>
				<h3>Connection Lost</h3>
				<p>Attempting to reconnect... ({attempt}/{maxAttempts})</p>
				{#if errorMessage}
					<p class="error-text">{errorMessage}</p>
				{/if}
			{:else if status === 'success'}
				<div class="reconnect-icon success">
					<RefreshCw size={32} />
				</div>
				<h3>Reconnected!</h3>
				<p>Connection restored successfully</p>
			{:else}
				<div class="reconnect-icon failed">
					<WifiOff size={32} />
				</div>
				<h3>Connection Failed</h3>
				<p>Unable to reconnect after {maxAttempts} attempts</p>
				{#if errorMessage}
					<p class="error-text">{errorMessage}</p>
				{/if}
				<div class="reconnect-actions">
					<Button variant="ghost" on:click={dismiss}>
						<X size={16} />
						Dismiss
					</Button>
					<Button variant="primary" on:click={manualReconnect}>
						<RefreshCw size={16} />
						Try Again
					</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.reconnect-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		backdrop-filter: blur(4px);
	}

	.reconnect-card {
		background: var(--bg-primary);
		border-radius: 12px;
		padding: 32px;
		text-align: center;
		max-width: 400px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
	}

	.reconnect-icon {
		margin-bottom: 16px;
	}

	.reconnect-icon.attempting {
		color: var(--primary-color);
	}

	.reconnect-icon.success {
		color: var(--success-color);
	}

	.reconnect-icon.failed {
		color: var(--error-color);
	}

	h3 {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: 8px;
	}

	p {
		color: var(--text-secondary);
		margin-bottom: 8px;
	}

	.error-text {
		font-size: 13px;
		color: var(--error-color);
	}

	.reconnect-actions {
		display: flex;
		gap: 12px;
		justify-content: center;
		margin-top: 24px;
	}

	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
```

#### Editor Error Highlighting

**File: `src/lib/components/editor/EditorErrorHighlight.ts`**

```typescript
import type * as monaco from 'monaco-editor';

export interface EditorError {
	position: number;
	message: string;
	code: string;
}

export function highlightError(
	editor: monaco.editor.IStandaloneCodeEditor,
	model: monaco.editor.ITextModel,
	error: EditorError
): monaco.IDisposable[] {
	const disposables: monaco.IDisposable[] = [];

	// Convert character position to line/column
	const pos = model.getPositionAt(error.position);

	// Find the word or token at the error position
	const wordAtPos = model.getWordAtPosition(pos);
	const startColumn = wordAtPos ? wordAtPos.startColumn : pos.column;
	const endColumn = wordAtPos ? wordAtPos.endColumn : pos.column + 1;

	// Create decoration for error highlight
	const decorations = editor.createDecorationsCollection([
		{
			range: {
				startLineNumber: pos.lineNumber,
				startColumn,
				endLineNumber: pos.lineNumber,
				endColumn
			},
			options: {
				isWholeLine: false,
				className: 'error-highlight',
				glyphMarginClassName: 'error-glyph',
				hoverMessage: {
					value: `**${error.code}**\n\n${error.message}`
				},
				overviewRuler: {
					color: '#ef4444',
					position: 4 // Right
				}
			}
		}
	]);

	disposables.push({
		dispose: () => decorations.clear()
	});

	// Add line highlight
	const lineDecorations = editor.createDecorationsCollection([
		{
			range: {
				startLineNumber: pos.lineNumber,
				startColumn: 1,
				endLineNumber: pos.lineNumber,
				endColumn: 1
			},
			options: {
				isWholeLine: true,
				className: 'error-line',
				marginClassName: 'error-margin'
			}
		}
	]);

	disposables.push({
		dispose: () => lineDecorations.clear()
	});

	// Scroll to error position
	editor.revealLineInCenter(pos.lineNumber);

	// Set cursor to error position
	editor.setPosition(pos);
	editor.focus();

	return disposables;
}

export function clearErrorHighlights(disposables: monaco.IDisposable[]): void {
	disposables.forEach((d) => d.dispose());
}

// CSS styles to inject
export const errorHighlightStyles = `
  .error-highlight {
    background-color: rgba(239, 68, 68, 0.3);
    border-bottom: 2px wavy #ef4444;
  }

  .error-line {
    background-color: rgba(239, 68, 68, 0.1);
  }

  .error-glyph {
    background-color: #ef4444;
    border-radius: 50%;
    width: 8px !important;
    height: 8px !important;
    margin-left: 4px;
    margin-top: 6px;
  }

  .error-margin {
    background-color: rgba(239, 68, 68, 0.2);
  }
`;
```

### 28.6 Query Auto-Save Integration

**File: `src/lib/stores/query.ts`** (add auto-save functionality)

```typescript
import { invoke } from '@tauri-apps/api/core';

// Add to query store
let autoSaveInterval: number | null = null;

export function startAutoSave(intervalMs = 5000) {
	if (autoSaveInterval) return;

	autoSaveInterval = window.setInterval(async () => {
		const state = get(queryStore);

		for (const tab of state.tabs) {
			if (tab.isDirty) {
				await invoke('persist_query', {
					query: {
						id: tab.id,
						connection_id: tab.connectionId,
						sql: tab.sql,
						cursor_position: tab.cursorPosition ?? 0,
						file_path: tab.filePath,
						is_dirty: tab.isDirty,
						saved_at: Date.now()
					}
				});
			}
		}
	}, intervalMs);
}

export function stopAutoSave() {
	if (autoSaveInterval) {
		window.clearInterval(autoSaveInterval);
		autoSaveInterval = null;
	}
}

export async function restoreSession() {
	const persisted = await invoke<PersistedQuery[]>('load_persisted_queries');

	for (const query of persisted) {
		queryStore.addTab({
			id: query.id,
			connectionId: query.connection_id,
			sql: query.sql,
			filePath: query.file_path,
			isDirty: query.is_dirty
		});
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
   - [ ] Google search for error
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
   - [ ] Warning (yellow) for alerts
   - [ ] Notice (blue) for info
   - [ ] Info (gray) for informational

6. **Editor Integration**
   - [ ] Error position highlighting
   - [ ] Wavy underline at error location
   - [ ] Hover tooltip with error details
   - [ ] Line highlight for error line

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Test error handling
await driver_session({ action: 'start', port: 9223 });

// Execute invalid SQL
await webview_keyboard({
	action: 'type',
	selector: '.monaco-editor textarea',
	text: 'SELECT * FROM nonexistent_table'
});

await webview_keyboard({ action: 'press', key: 'Meta+Enter' });

// Wait for error
await webview_wait_for({ type: 'selector', value: '.error-display' });

// Screenshot error state
await webview_screenshot({ filePath: 'query-error.png' });

// Check error content
const snapshot = await webview_dom_snapshot({ type: 'accessibility' });
console.log('Error displayed:', snapshot);

// Test reconnection
await ipc_execute_command({
	command: 'reconnect',
	args: { connectionId: 'test-conn', maxAttempts: 3 }
});

await driver_session({ action: 'stop' });
```

### Using Playwright MCP

```typescript
// Test error display
await browser_navigate({ url: 'http://localhost:1420' });

// Type invalid query
await browser_type({
	element: 'Query editor',
	ref: '.monaco-editor textarea',
	text: 'SELEC * FORM users' // Intentional typos
});

// Execute
await browser_press_key({ key: 'Meta+Enter' });

// Wait for error
await browser_wait_for({ text: 'SYNTAX_ERROR' });

// Verify error actions
const snapshot = await browser_snapshot();
console.log('Error actions available');

// Take screenshot
await browser_take_screenshot({ filename: 'error-display.png' });

// Test copy error action
await browser_click({
	element: 'Copy Error button',
	ref: '[data-action="copy_error"]'
});
```

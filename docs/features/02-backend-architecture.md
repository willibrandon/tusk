# Feature 02: Backend Architecture

## Overview

Establish the Rust backend structure for a pure GPUI application with proper module organization, error handling, async patterns, and service layer abstractions. Since GPUI is a pure Rust framework, there is no frontend/backend split—the entire application is Rust code. This document focuses on the service layer, data access patterns, and async execution that power the UI.

## Goals

- Define clear module boundaries and responsibilities within the Rust codebase
- Implement robust error handling with thiserror
- Set up state management using GPUI's Entity and Global patterns
- Create service layer abstractions for database, storage, and credentials
- Establish async patterns with GPUI's BackgroundExecutor and Tasks

## Technical Specification

### 1. Module Structure

```
crates/
├── tusk_core/                      # Shared types and utilities
│   └── src/
│       ├── lib.rs
│       ├── error.rs                # Error types
│       ├── util.rs                 # Utilities
│       └── models/
│           ├── mod.rs
│           ├── connection.rs       # Connection config
│           ├── schema.rs           # Schema objects
│           ├── query.rs            # Query results
│           └── settings.rs         # App settings
├── tusk_db/                        # Database connectivity
│   └── src/
│       ├── lib.rs
│       ├── connection.rs           # Connection management
│       ├── pool.rs                 # Connection pooling
│       ├── query.rs                # Query execution engine
│       ├── schema.rs               # Schema introspection
│       ├── types.rs                # PostgreSQL type mapping
│       ├── ssh.rs                  # SSH tunnel management
│       └── ssl.rs                  # SSL/TLS handling
├── tusk_storage/                   # Local SQLite storage
│   └── src/
│       ├── lib.rs
│       ├── database.rs             # SQLite database
│       ├── migrations.rs           # Schema migrations
│       ├── connections.rs          # Saved connections
│       ├── history.rs              # Query history
│       └── settings.rs             # User settings
└── tusk_app/                       # Application logic
    └── src/
        ├── lib.rs
        ├── app.rs                  # TuskApp main struct
        ├── workspace.rs            # Workspace management
        ├── actions.rs              # Global actions
        └── state.rs                # Global state management
```

### 2. Error Handling (tusk_core/src/error.rs)

```rust
use thiserror::Error;
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Clone)]
pub enum TuskError {
    // Connection errors
    #[error("Connection failed: {message}")]
    ConnectionFailed {
        message: String,
        source_msg: Option<String>,
    },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("SSL/TLS error: {0}")]
    SslError(String),

    #[error("SSH tunnel error: {0}")]
    SshError(String),

    #[error("Connection timeout after {seconds}s")]
    ConnectionTimeout { seconds: u64 },

    #[error("Connection not found: {id}")]
    ConnectionNotFound { id: String },

    #[error("No active connection")]
    NoActiveConnection,

    // Query errors
    #[error("Query execution failed")]
    QueryFailed {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<u32>,
        code: Option<String>,
    },

    #[error("Query cancelled")]
    QueryCancelled,

    #[error("Query timeout after {ms}ms")]
    QueryTimeout { ms: u64 },

    // Storage errors
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Migration failed: {0}")]
    MigrationError(String),

    // Keyring errors
    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("Credential not found for: {0}")]
    CredentialNotFound(String),

    // Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    // General errors
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// Error details for display in UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: ErrorType,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<u32>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorType {
    Connection,
    Authentication,
    Ssl,
    Ssh,
    Query,
    Storage,
    Keyring,
    Internal,
}

impl From<&TuskError> for ErrorInfo {
    fn from(err: &TuskError) -> Self {
        match err {
            TuskError::QueryFailed { message, detail, hint, position, code } => {
                ErrorInfo {
                    error_type: ErrorType::Query,
                    message: message.clone(),
                    detail: detail.clone(),
                    hint: hint.clone(),
                    position: *position,
                    code: code.clone(),
                }
            }
            TuskError::ConnectionFailed { message, source_msg } => {
                ErrorInfo {
                    error_type: ErrorType::Connection,
                    message: message.clone(),
                    detail: source_msg.clone(),
                    hint: None,
                    position: None,
                    code: None,
                }
            }
            TuskError::AuthenticationFailed(msg) => {
                ErrorInfo {
                    error_type: ErrorType::Authentication,
                    message: msg.clone(),
                    detail: None,
                    hint: Some("Check your username and password".into()),
                    position: None,
                    code: None,
                }
            }
            TuskError::SslError(msg) => {
                ErrorInfo {
                    error_type: ErrorType::Ssl,
                    message: msg.clone(),
                    detail: None,
                    hint: Some("Check SSL certificate configuration".into()),
                    position: None,
                    code: None,
                }
            }
            TuskError::SshError(msg) => {
                ErrorInfo {
                    error_type: ErrorType::Ssh,
                    message: msg.clone(),
                    detail: None,
                    hint: Some("Check SSH tunnel configuration".into()),
                    position: None,
                    code: None,
                }
            }
            _ => ErrorInfo {
                error_type: ErrorType::Internal,
                message: err.to_string(),
                detail: None,
                hint: None,
                position: None,
                code: None,
            }
        }
    }
}

impl From<tokio_postgres::Error> for TuskError {
    fn from(err: tokio_postgres::Error) -> Self {
        if let Some(db_err) = err.as_db_error() {
            TuskError::QueryFailed {
                message: db_err.message().to_string(),
                detail: db_err.detail().map(|s| s.to_string()),
                hint: db_err.hint().map(|s| s.to_string()),
                position: db_err.position().map(|p| match p {
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position,
                }),
                code: Some(db_err.code().code().to_string()),
            }
        } else {
            TuskError::ConnectionFailed {
                message: err.to_string(),
                source_msg: None,
            }
        }
    }
}

impl From<rusqlite::Error> for TuskError {
    fn from(err: rusqlite::Error) -> Self {
        TuskError::StorageError(err.to_string())
    }
}

impl From<serde_json::Error> for TuskError {
    fn from(err: serde_json::Error) -> Self {
        TuskError::SerializationError(err.to_string())
    }
}

impl From<keyring::Error> for TuskError {
    fn from(err: keyring::Error) -> Self {
        TuskError::KeyringError(err.to_string())
    }
}

impl From<std::io::Error> for TuskError {
    fn from(err: std::io::Error) -> Self {
        TuskError::IoError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, TuskError>;
```

### 3. Global Application State (tusk_app/src/state.rs)

```rust
use gpui::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use tusk_core::error::Result;
use tusk_db::{ConnectionPool, SchemaCache};
use tusk_storage::Database;

/// Global application state accessible via cx.global::<TuskState>()
pub struct TuskState {
    /// Active database connections
    connections: RwLock<HashMap<Uuid, Arc<ConnectionPool>>>,

    /// Schema cache per connection
    schema_caches: RwLock<HashMap<Uuid, Arc<SchemaCache>>>,

    /// Active query handles for cancellation
    active_queries: RwLock<HashMap<Uuid, Arc<QueryHandle>>>,

    /// Local SQLite database for settings, history, etc.
    pub database: Database,

    /// Application data directory
    pub data_dir: std::path::PathBuf,
}

impl Global for TuskState {}

/// Handle for cancelling a running query
pub struct QueryHandle {
    cancel_token: tokio_util::sync::CancellationToken,
}

impl QueryHandle {
    pub fn new() -> Self {
        Self {
            cancel_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    pub fn token(&self) -> tokio_util::sync::CancellationToken {
        self.cancel_token.clone()
    }
}

impl TuskState {
    pub fn new(data_dir: std::path::PathBuf) -> Result<Self> {
        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir)?;

        // Initialize local database
        let db_path = data_dir.join("tusk.db");
        let database = Database::new(&db_path)?;

        Ok(Self {
            connections: RwLock::new(HashMap::new()),
            schema_caches: RwLock::new(HashMap::new()),
            active_queries: RwLock::new(HashMap::new()),
            database,
            data_dir,
        })
    }

    /// Get a connection pool by ID
    pub fn get_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        self.connections.read().get(id).cloned()
    }

    /// Add a new connection pool
    pub fn add_connection(&self, id: Uuid, pool: ConnectionPool) {
        self.connections.write().insert(id, Arc::new(pool));
    }

    /// Remove a connection pool
    pub fn remove_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        let pool = self.connections.write().remove(id);
        // Also remove schema cache
        self.schema_caches.write().remove(id);
        pool
    }

    /// Get all connection IDs
    pub fn connection_ids(&self) -> Vec<Uuid> {
        self.connections.read().keys().copied().collect()
    }

    /// Get schema cache for a connection
    pub fn get_schema_cache(&self, conn_id: &Uuid) -> Option<Arc<SchemaCache>> {
        self.schema_caches.read().get(conn_id).cloned()
    }

    /// Set schema cache for a connection
    pub fn set_schema_cache(&self, conn_id: Uuid, cache: SchemaCache) {
        self.schema_caches.write().insert(conn_id, Arc::new(cache));
    }

    /// Register a new query for potential cancellation
    pub fn register_query(&self, query_id: Uuid) -> Arc<QueryHandle> {
        let handle = Arc::new(QueryHandle::new());
        self.active_queries.write().insert(query_id, handle.clone());
        handle
    }

    /// Cancel a query by ID
    pub fn cancel_query(&self, query_id: &Uuid) -> bool {
        if let Some(handle) = self.active_queries.read().get(query_id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    /// Unregister a completed query
    pub fn unregister_query(&self, query_id: &Uuid) {
        self.active_queries.write().remove(query_id);
    }
}

/// Initialize global state in GPUI
pub fn init_state(cx: &mut AppContext) -> Result<()> {
    let data_dir = get_data_directory();
    let state = TuskState::new(data_dir)?;
    cx.set_global(state);
    Ok(())
}

/// Get the appropriate data directory for the platform
fn get_data_directory() -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        // Use local directory in dev
        std::env::current_dir()
            .unwrap_or_default()
            .join(".tusk-dev")
    } else {
        // Use OS-appropriate directory in production
        directories::ProjectDirs::from("com", "tusk", "Tusk")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(".tusk")
            })
    }
}

/// Extension trait for easy state access
pub trait TuskStateExt {
    fn tusk_state(&self) -> &TuskState;
}

impl TuskStateExt for AppContext {
    fn tusk_state(&self) -> &TuskState {
        self.global::<TuskState>()
    }
}

impl<V> TuskStateExt for ViewContext<'_, V> {
    fn tusk_state(&self) -> &TuskState {
        self.global::<TuskState>()
    }
}

impl<V> TuskStateExt for ModelContext<'_, V> {
    fn tusk_state(&self) -> &TuskState {
        self.global::<TuskState>()
    }
}
```

### 4. Service Layer Pattern

Services in GPUI are typically implemented as structs that hold state and are accessed via Entity handles or global state. Async operations use GPUI's background executor.

```rust
// tusk_db/src/lib.rs
mod connection;
mod pool;
mod query;
mod schema;
mod ssh;
mod ssl;
mod types;

pub use connection::{Connection, ConnectionManager};
pub use pool::ConnectionPool;
pub use query::{QueryExecutor, QueryResult, RowBatch};
pub use schema::{SchemaCache, SchemaIntrospector};
pub use ssh::SshTunnel;
pub use ssl::SslConfig;
pub use types::{PostgresType, TypeRegistry};
```

```rust
// tusk_db/src/pool.rs
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;
use std::time::Duration;

use tusk_core::error::{Result, TuskError};
use tusk_core::models::ConnectionConfig;

pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
}

impl ConnectionPool {
    pub async fn new(config: ConnectionConfig, password: Option<String>) -> Result<Self> {
        let mut pg_config = tokio_postgres::Config::new();
        pg_config
            .host(&config.host)
            .port(config.port)
            .dbname(&config.database)
            .user(&config.username)
            .connect_timeout(Duration::from_secs(config.options.connect_timeout_sec))
            .application_name(&config.options.application_name);

        if let Some(pwd) = password {
            pg_config.password(&pwd);
        }

        if let Some(timeout_ms) = config.options.statement_timeout_ms {
            pg_config.options(&format!("-c statement_timeout={}", timeout_ms));
        }

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);

        let pool = Pool::builder(mgr)
            .max_size(5)
            .build()
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to create connection pool: {}", e),
                source_msg: None,
            })?;

        // Test connection
        let _client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to connect: {}", e),
            source_msg: None,
        })?;

        Ok(Self { pool, config })
    }

    pub async fn get_client(&self) -> Result<deadpool_postgres::Object> {
        self.pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to get connection from pool: {}", e),
            source_msg: None,
        })
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub fn status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
}
```

### 5. Async Execution with GPUI

GPUI provides BackgroundExecutor for running async work off the main thread:

```rust
// tusk_app/src/services.rs
use gpui::*;
use std::sync::Arc;
use uuid::Uuid;

use tusk_core::error::Result;
use tusk_core::models::ConnectionConfig;
use tusk_db::{ConnectionPool, QueryResult};
use crate::state::{TuskState, TuskStateExt};

/// Service for managing database connections
pub struct ConnectionService;

impl ConnectionService {
    /// Connect to a database (runs in background)
    pub fn connect(
        config: ConnectionConfig,
        password: Option<String>,
        cx: &mut AppContext,
    ) -> Task<Result<Uuid>> {
        let state = cx.global::<TuskState>();
        let conn_id = config.id;

        cx.background_executor().spawn(async move {
            let pool = ConnectionPool::new(config, password).await?;

            // Store in state (need to coordinate with main thread)
            // This is returned to caller who updates state
            Ok(conn_id)
        })
    }

    /// Disconnect from a database
    pub fn disconnect(conn_id: Uuid, cx: &mut AppContext) -> Task<Result<()>> {
        cx.background_executor().spawn(async move {
            // Connection cleanup happens when Arc is dropped
            Ok(())
        })
    }
}

/// Service for executing queries
pub struct QueryService;

impl QueryService {
    /// Execute a query (runs in background, streams results)
    pub fn execute(
        conn_id: Uuid,
        query: String,
        cx: &mut AppContext,
    ) -> (Uuid, Task<Result<QueryResult>>) {
        let query_id = Uuid::new_v4();
        let state = cx.global::<TuskState>();
        let pool = state.get_connection(&conn_id);
        let handle = state.register_query(query_id);

        let task = cx.background_executor().spawn(async move {
            let pool = pool.ok_or(TuskError::ConnectionNotFound {
                id: conn_id.to_string(),
            })?;

            let client = pool.get_client().await?;

            // Execute with cancellation support
            tokio::select! {
                result = client.query(&query, &[]) => {
                    let rows = result?;
                    Ok(QueryResult::from_rows(rows))
                }
                _ = handle.token().cancelled() => {
                    Err(TuskError::QueryCancelled)
                }
            }
        });

        (query_id, task)
    }

    /// Cancel a running query
    pub fn cancel(query_id: Uuid, cx: &AppContext) -> bool {
        cx.global::<TuskState>().cancel_query(&query_id)
    }
}
```

### 6. Models (tusk_core/src/models/)

```rust
// tusk_core/src/models/mod.rs
pub mod connection;
pub mod query;
pub mod schema;
pub mod settings;

pub use connection::*;
pub use query::*;
pub use schema::*;
pub use settings::*;
```

```rust
// tusk_core/src/models/connection.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub group_id: Option<Uuid>,

    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password_in_keyring: bool,

    pub ssl_mode: SslMode,
    pub ssl_ca_cert: Option<String>,
    pub ssl_client_cert: Option<String>,
    pub ssl_client_key: Option<String>,

    pub ssh_tunnel: Option<SshTunnelConfig>,

    pub options: ConnectionOptions,
}

impl ConnectionConfig {
    pub fn new(name: impl Into<String>, host: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            color: None,
            group_id: None,
            host: host.into(),
            port: 5432,
            database: database.into(),
            username: String::new(),
            password_in_keyring: false,
            ssl_mode: SslMode::default(),
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            ssh_tunnel: None,
            options: ConnectionOptions::default(),
        }
    }

    /// Get the keyring key for this connection's password
    pub fn keyring_key(&self) -> String {
        format!("tusk:connection:{}", self.id)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnelConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuthMethod,
    pub key_path: Option<String>,
    pub passphrase_in_keyring: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SshAuthMethod {
    Password,
    Key,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_sec: u64,

    pub statement_timeout_ms: Option<u64>,

    #[serde(default = "default_app_name")]
    pub application_name: String,

    #[serde(default)]
    pub readonly: bool,
}

fn default_connect_timeout() -> u64 { 10 }
fn default_app_name() -> String { "Tusk".to_string() }

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            connect_timeout_sec: 10,
            statement_timeout_ms: None,
            application_name: "Tusk".to_string(),
            readonly: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionGroup {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}
```

```rust
// tusk_core/src/models/query.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
    pub rows_affected: u64,
    pub execution_time_ms: u64,
    pub query_type: QueryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub type_oid: u32,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub values: Vec<CellValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Timestamp(DateTime<Utc>),
    Json(serde_json::Value),
    Array(Vec<CellValue>),
}

impl CellValue {
    pub fn is_null(&self) -> bool {
        matches!(self, CellValue::Null)
    }

    pub fn display(&self) -> String {
        match self {
            CellValue::Null => "NULL".to_string(),
            CellValue::Bool(b) => b.to_string(),
            CellValue::Int(i) => i.to_string(),
            CellValue::Float(f) => f.to_string(),
            CellValue::String(s) => s.clone(),
            CellValue::Bytes(b) => format!("\\x{}", hex::encode(b)),
            CellValue::Timestamp(ts) => ts.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            CellValue::Json(j) => j.to_string(),
            CellValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.display()).collect();
                format!("{{{}}}", items.join(","))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Alter,
    Drop,
    Other,
}

impl Default for QueryType {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub query: String,
    pub executed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
    pub rows_affected: Option<u64>,
    pub error: Option<String>,
    pub database: String,
}
```

### 7. Event-Driven Updates

GPUI components communicate through subscriptions and events:

```rust
// tusk_app/src/events.rs
use gpui::*;
use uuid::Uuid;

use tusk_core::models::{ConnectionStatus, QueryResult};
use tusk_core::error::ErrorInfo;

/// Events for connection state changes
#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    StatusChanged {
        connection_id: Uuid,
        status: ConnectionStatus,
    },
    Connected {
        connection_id: Uuid,
    },
    Disconnected {
        connection_id: Uuid,
    },
    Error {
        connection_id: Uuid,
        error: ErrorInfo,
    },
}

/// Events for query execution
#[derive(Clone, Debug)]
pub enum QueryEvent {
    Started {
        query_id: Uuid,
        connection_id: Uuid,
    },
    Progress {
        query_id: Uuid,
        rows_fetched: usize,
    },
    Completed {
        query_id: Uuid,
        result: QueryResult,
    },
    Failed {
        query_id: Uuid,
        error: ErrorInfo,
    },
    Cancelled {
        query_id: Uuid,
    },
}

/// Events for schema changes
#[derive(Clone, Debug)]
pub enum SchemaEvent {
    RefreshStarted {
        connection_id: Uuid,
    },
    RefreshCompleted {
        connection_id: Uuid,
    },
    ObjectCreated {
        connection_id: Uuid,
        object_type: String,
        object_name: String,
    },
    ObjectDropped {
        connection_id: Uuid,
        object_type: String,
        object_name: String,
    },
}

/// Global event bus for application-wide events
pub struct EventBus {
    connection_subscribers: Vec<Box<dyn Fn(&ConnectionEvent, &mut AppContext) + Send + Sync>>,
    query_subscribers: Vec<Box<dyn Fn(&QueryEvent, &mut AppContext) + Send + Sync>>,
    schema_subscribers: Vec<Box<dyn Fn(&SchemaEvent, &mut AppContext) + Send + Sync>>,
}

impl Global for EventBus {}

impl EventBus {
    pub fn new() -> Self {
        Self {
            connection_subscribers: Vec::new(),
            query_subscribers: Vec::new(),
            schema_subscribers: Vec::new(),
        }
    }

    pub fn emit_connection(&self, event: ConnectionEvent, cx: &mut AppContext) {
        for subscriber in &self.connection_subscribers {
            subscriber(&event, cx);
        }
    }

    pub fn emit_query(&self, event: QueryEvent, cx: &mut AppContext) {
        for subscriber in &self.query_subscribers {
            subscriber(&event, cx);
        }
    }

    pub fn emit_schema(&self, event: SchemaEvent, cx: &mut AppContext) {
        for subscriber in &self.schema_subscribers {
            subscriber(&event, cx);
        }
    }
}
```

### 8. Credential Management

```rust
// tusk_storage/src/keyring.rs
use keyring::Entry;
use tusk_core::error::{Result, TuskError};

const SERVICE_NAME: &str = "com.tusk.app";

pub struct KeyringService;

impl KeyringService {
    /// Store a password in the OS keyring
    pub fn store_password(key: &str, password: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        entry
            .set_password(password)
            .map_err(|e| TuskError::KeyringError(e.to_string()))
    }

    /// Retrieve a password from the OS keyring
    pub fn get_password(key: &str) -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => TuskError::CredentialNotFound(key.to_string()),
                _ => TuskError::KeyringError(e.to_string()),
            })
    }

    /// Delete a password from the OS keyring
    pub fn delete_password(key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| TuskError::KeyringError(e.to_string()))
    }

    /// Check if a password exists
    pub fn has_password(key: &str) -> bool {
        Entry::new(SERVICE_NAME, key)
            .and_then(|e| e.get_password())
            .is_ok()
    }
}
```

### 9. Logging Configuration

```rust
// crates/tusk/src/logging.rs
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use std::path::Path;

pub fn init_logging(data_dir: &Path, debug: bool) {
    let log_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "tusk.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = if debug {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "tusk=debug,gpui=info".into())
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "tusk=info,gpui=warn".into())
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();
}
```

### 10. Crate Dependencies

```toml
# crates/tusk_core/Cargo.toml
[package]
name = "tusk_core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
chrono.workspace = true
tokio-postgres.workspace = true
rusqlite.workspace = true
keyring.workspace = true
hex = "0.4"
```

```toml
# crates/tusk_db/Cargo.toml
[package]
name = "tusk_db"
version.workspace = true
edition.workspace = true

[dependencies]
tusk_core.path = "../tusk_core"
tokio.workspace = true
tokio-postgres.workspace = true
deadpool-postgres.workspace = true
postgres-native-tls.workspace = true
native-tls.workspace = true
tokio-util = { version = "0.7", features = ["rt"] }
russh.workspace = true
russh-keys.workspace = true
thiserror.workspace = true
tracing.workspace = true
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
parking_lot.workspace = true
```

```toml
# crates/tusk_storage/Cargo.toml
[package]
name = "tusk_storage"
version.workspace = true
edition.workspace = true

[dependencies]
tusk_core.path = "../tusk_core"
rusqlite.workspace = true
keyring.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
chrono.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

```toml
# crates/tusk_app/Cargo.toml
[package]
name = "tusk_app"
version.workspace = true
edition.workspace = true

[dependencies]
tusk_core.path = "../tusk_core"
tusk_db.path = "../tusk_db"
tusk_storage.path = "../tusk_storage"
tusk_ui.path = "../tusk_ui"
gpui.workspace = true
tokio.workspace = true
parking_lot.workspace = true
uuid.workspace = true
directories.workspace = true
tracing.workspace = true
anyhow.workspace = true
```

## Acceptance Criteria

1. [ ] All crates compile without errors
2. [ ] Error types cover all expected error cases
3. [ ] TuskState initializes correctly and is accessible via Global
4. [ ] Connection pool operations work correctly
5. [ ] Schema cache operations work correctly
6. [ ] Query cancellation mechanism works
7. [ ] Keyring service stores and retrieves passwords
8. [ ] Logging outputs to console and file
9. [ ] Background tasks execute correctly via BackgroundExecutor

## Testing

```rust
// tests/backend_test.rs
#[cfg(test)]
mod tests {
    use tusk_core::error::TuskError;
    use tusk_core::models::ConnectionConfig;

    #[test]
    fn test_error_display() {
        let err = TuskError::QueryFailed {
            message: "syntax error".into(),
            detail: Some("near SELECT".into()),
            hint: None,
            position: Some(10),
            code: Some("42601".into()),
        };
        assert!(err.to_string().contains("syntax error"));
    }

    #[test]
    fn test_connection_config() {
        let config = ConnectionConfig::new("Local", "localhost", "postgres");
        assert_eq!(config.port, 5432);
        assert!(!config.id.is_nil());
    }
}
```

## Dependencies on Other Features

- 01-project-initialization.md

## Dependent Features

- 04-ipc-layer.md (now internal Rust APIs, not IPC)
- 05-local-storage.md
- 07-connection-management.md
- All other backend features

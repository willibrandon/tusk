# Feature 02: Backend Architecture

## Overview

Establish the Rust backend structure with proper module organization, error handling, state management, and foundational services.

## Goals

- Define clear module boundaries and responsibilities
- Implement robust error handling with thiserror
- Set up application state management
- Create service layer abstractions
- Establish async patterns with Tokio

## Technical Specification

### 1. Module Structure

```
src-tauri/src/
├── main.rs                 # Entry point
├── lib.rs                  # Library root, Tauri setup
├── error.rs                # Error types
├── state.rs                # Application state
├── commands/
│   ├── mod.rs              # Command registration
│   ├── connection.rs       # Connection commands
│   ├── query.rs            # Query execution commands
│   ├── schema.rs           # Schema introspection commands
│   ├── admin.rs            # Admin/monitoring commands
│   ├── storage.rs          # Local storage commands
│   └── settings.rs         # Settings commands
├── services/
│   ├── mod.rs
│   ├── connection.rs       # Connection pool management
│   ├── query.rs            # Query execution engine
│   ├── schema.rs           # Schema introspection
│   ├── admin.rs            # Admin statistics
│   ├── storage.rs          # SQLite local storage
│   ├── keyring.rs          # Credential management
│   └── ssh.rs              # SSH tunnel management
└── models/
    ├── mod.rs
    ├── connection.rs       # Connection config
    ├── schema.rs           # Schema objects
    ├── query.rs            # Query results
    └── settings.rs         # App settings
```

### 2. Error Handling (error.rs)

```rust
use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug)]
pub enum TuskError {
    // Connection errors
    #[error("Connection failed: {message}")]
    ConnectionFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
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
        position: Option<i32>,
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

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

// Serializable error for IPC
#[derive(Debug, Serialize, Clone)]
pub struct ErrorResponse {
    pub error_type: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<i32>,
    pub code: Option<String>,
}

impl From<TuskError> for ErrorResponse {
    fn from(err: TuskError) -> Self {
        match &err {
            TuskError::QueryFailed { message, detail, hint, position, code } => {
                ErrorResponse {
                    error_type: "QueryFailed".to_string(),
                    message: message.clone(),
                    detail: detail.clone(),
                    hint: hint.clone(),
                    position: *position,
                    code: code.clone(),
                }
            }
            _ => ErrorResponse {
                error_type: format!("{:?}", err).split('{').next().unwrap_or("Unknown").trim().to_string(),
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
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos as i32,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position as i32,
                }),
                code: Some(db_err.code().code().to_string()),
            }
        } else {
            TuskError::ConnectionFailed {
                message: err.to_string(),
                source: Some(Box::new(err)),
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

// Make TuskError serializable for Tauri
impl serde::Serialize for TuskError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ErrorResponse::from(self.clone()).serialize(serializer)
    }
}

impl Clone for TuskError {
    fn clone(&self) -> Self {
        match self {
            TuskError::ConnectionFailed { message, .. } => {
                TuskError::ConnectionFailed { message: message.clone(), source: None }
            }
            TuskError::AuthenticationFailed(s) => TuskError::AuthenticationFailed(s.clone()),
            TuskError::SslError(s) => TuskError::SslError(s.clone()),
            TuskError::SshError(s) => TuskError::SshError(s.clone()),
            TuskError::ConnectionTimeout { seconds } => TuskError::ConnectionTimeout { seconds: *seconds },
            TuskError::ConnectionNotFound { id } => TuskError::ConnectionNotFound { id: id.clone() },
            TuskError::NoActiveConnection => TuskError::NoActiveConnection,
            TuskError::QueryFailed { message, detail, hint, position, code } => {
                TuskError::QueryFailed {
                    message: message.clone(),
                    detail: detail.clone(),
                    hint: hint.clone(),
                    position: *position,
                    code: code.clone(),
                }
            }
            TuskError::QueryCancelled => TuskError::QueryCancelled,
            TuskError::QueryTimeout { ms } => TuskError::QueryTimeout { ms: *ms },
            TuskError::StorageError(s) => TuskError::StorageError(s.clone()),
            TuskError::MigrationError(s) => TuskError::MigrationError(s.clone()),
            TuskError::KeyringError(s) => TuskError::KeyringError(s.clone()),
            TuskError::CredentialNotFound(s) => TuskError::CredentialNotFound(s.clone()),
            TuskError::SerializationError(s) => TuskError::SerializationError(s.clone()),
            TuskError::Internal(s) => TuskError::Internal(s.clone()),
            TuskError::InvalidInput(s) => TuskError::InvalidInput(s.clone()),
            TuskError::NotImplemented(s) => TuskError::NotImplemented(s.clone()),
        }
    }
}

pub type Result<T> = std::result::Result<T, TuskError>;
```

### 3. Application State (state.rs)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::services::{
    connection::ConnectionPool,
    storage::StorageService,
    schema::SchemaCache,
};

/// Global application state managed by Tauri
pub struct AppState {
    /// Active connection pools keyed by connection ID
    pub connections: RwLock<HashMap<Uuid, Arc<ConnectionPool>>>,

    /// Schema cache per connection
    pub schema_caches: RwLock<HashMap<Uuid, Arc<SchemaCache>>>,

    /// Active queries that can be cancelled
    pub active_queries: RwLock<HashMap<Uuid, tokio::sync::watch::Sender<bool>>>,

    /// Local SQLite storage
    pub storage: Arc<StorageService>,

    /// Application data directory
    pub data_dir: std::path::PathBuf,
}

impl AppState {
    pub async fn new(data_dir: std::path::PathBuf) -> crate::error::Result<Self> {
        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            crate::error::TuskError::StorageError(format!(
                "Failed to create data directory: {}",
                e
            ))
        })?;

        let storage = StorageService::new(&data_dir).await?;

        Ok(Self {
            connections: RwLock::new(HashMap::new()),
            schema_caches: RwLock::new(HashMap::new()),
            active_queries: RwLock::new(HashMap::new()),
            storage: Arc::new(storage),
            data_dir,
        })
    }

    /// Get a connection pool by ID
    pub async fn get_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        self.connections.read().await.get(id).cloned()
    }

    /// Add a new connection pool
    pub async fn add_connection(&self, id: Uuid, pool: ConnectionPool) {
        self.connections.write().await.insert(id, Arc::new(pool));
    }

    /// Remove a connection pool
    pub async fn remove_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        let mut connections = self.connections.write().await;
        let pool = connections.remove(id);

        // Also remove schema cache
        self.schema_caches.write().await.remove(id);

        pool
    }

    /// Get schema cache for a connection
    pub async fn get_schema_cache(&self, conn_id: &Uuid) -> Option<Arc<SchemaCache>> {
        self.schema_caches.read().await.get(conn_id).cloned()
    }

    /// Set schema cache for a connection
    pub async fn set_schema_cache(&self, conn_id: Uuid, cache: SchemaCache) {
        self.schema_caches.write().await.insert(conn_id, Arc::new(cache));
    }

    /// Register a cancellable query
    pub async fn register_query(&self, query_id: Uuid) -> tokio::sync::watch::Receiver<bool> {
        let (tx, rx) = tokio::sync::watch::channel(false);
        self.active_queries.write().await.insert(query_id, tx);
        rx
    }

    /// Cancel a query
    pub async fn cancel_query(&self, query_id: &Uuid) -> bool {
        if let Some(tx) = self.active_queries.write().await.remove(query_id) {
            let _ = tx.send(true);
            true
        } else {
            false
        }
    }

    /// Unregister a completed query
    pub async fn unregister_query(&self, query_id: &Uuid) {
        self.active_queries.write().await.remove(query_id);
    }
}
```

### 4. Service Layer Pattern

```rust
// services/mod.rs
pub mod connection;
pub mod query;
pub mod schema;
pub mod admin;
pub mod storage;
pub mod keyring;
pub mod ssh;

// Re-export main types
pub use connection::ConnectionPool;
pub use query::QueryService;
pub use schema::{SchemaService, SchemaCache};
pub use admin::AdminService;
pub use storage::StorageService;
pub use keyring::KeyringService;
pub use ssh::SshTunnelService;
```

```rust
// services/connection.rs (skeleton)
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;
use crate::error::{Result, TuskError};
use crate::models::connection::ConnectionConfig;

pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
}

impl ConnectionPool {
    pub async fn new(config: ConnectionConfig) -> Result<Self> {
        // Implementation in Feature 07
        todo!()
    }

    pub async fn get_client(&self) -> Result<deadpool_postgres::Client> {
        self.pool
            .get()
            .await
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to get connection from pool: {}", e),
                source: None,
            })
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}
```

### 5. Models Layer

```rust
// models/mod.rs
pub mod connection;
pub mod schema;
pub mod query;
pub mod settings;

pub use connection::*;
pub use schema::*;
pub use query::*;
pub use settings::*;
```

```rust
// models/connection.rs
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}
```

### 6. Updated lib.rs

```rust
// src-tauri/src/lib.rs
pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use directories::ProjectDirs;

pub fn run() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tusk=debug,tauri=info".into()),
        )
        .init();

    tracing::info!("Starting Tusk");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Determine data directory
            let data_dir = if cfg!(debug_assertions) {
                // Use local directory in dev
                std::env::current_dir()?.join(".tusk-dev")
            } else {
                // Use OS-appropriate directory in production
                ProjectDirs::from("com", "tusk", "Tusk")
                    .map(|dirs| dirs.data_dir().to_path_buf())
                    .unwrap_or_else(|| {
                        std::env::current_dir()
                            .unwrap_or_default()
                            .join(".tusk")
                    })
            };

            tracing::info!("Data directory: {:?}", data_dir);

            // Initialize state asynchronously
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match AppState::new(data_dir).await {
                    Ok(state) => {
                        handle.manage(state);
                        tracing::info!("Application state initialized");
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize state: {}", e);
                        // Could emit an error event to frontend here
                    }
                }
            });

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Commands registered here by feature
            commands::health_check,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 7. Command Registration Pattern

```rust
// commands/mod.rs
pub mod connection;
pub mod query;
pub mod schema;
pub mod admin;
pub mod storage;
pub mod settings;

use tauri::command;

/// Health check command for testing
#[command]
pub async fn health_check() -> Result<String, String> {
    Ok("Tusk is running".to_string())
}

// Re-export all commands for registration
// Each module will export its commands
```

### 8. Logging Configuration

```rust
// Optional: Add to lib.rs for file logging in production
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn setup_logging(data_dir: &std::path::Path) {
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        data_dir.join("logs"),
        "tusk.log",
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tusk=info".into()),
        )
        .init();
}
```

## Acceptance Criteria

1. [ ] Module structure compiles without errors
2. [ ] Error types cover all expected error cases
3. [ ] Errors serialize correctly for IPC
4. [ ] AppState initializes with data directory
5. [ ] Connection pool storage works correctly
6. [ ] Schema cache storage works correctly
7. [ ] Query cancellation registration works
8. [ ] Logging outputs to console in dev mode
9. [ ] health_check command returns success

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Execute IPC: ipc_execute_command command="health_check"
4. Verify response: "Tusk is running"
5. Check logs: read_logs source=console
```

## Dependencies on Other Features

- 01-project-initialization.md

## Dependent Features

- 04-ipc-layer.md
- 05-local-storage.md
- 07-connection-management.md
- All other backend features

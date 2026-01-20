# Research: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-19

## Overview

Research findings for implementing the Rust backend architecture with proper module organization, error handling, state management, and foundational services.

---

## 1. Error Handling with thiserror

**Decision**: Use `thiserror` with a unified `TuskError` enum that is serializable for IPC transport.

**Rationale**:

- Tauri commands require `Result<T, E>` where `E: Serialize` for frontend consumption
- PostgreSQL errors contain rich metadata (code, position, hint, detail) that must be preserved
- Single error type simplifies error propagation and frontend handling

**Alternatives considered**:

- `anyhow` alone: Rejected because it doesn't serialize well for IPC
- Multiple error types per module: Rejected due to complexity in command handlers

**Implementation Pattern**:

```rust
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum TuskError {
    #[error("Database error: {message}")]
    Database {
        message: String,
        code: Option<String>,
        position: Option<u32>,
        hint: Option<String>,
        detail: Option<String>,
    },

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Credential error: {0}")]
    Credential(String),

    #[error("Query cancelled")]
    QueryCancelled,

    #[error("Query timeout after {0}ms")]
    QueryTimeout(u64),

    #[error("Initialization failed: {0}")]
    Initialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
```

**PostgreSQL Error Preservation**:

```rust
impl From<tokio_postgres::Error> for TuskError {
    fn from(err: tokio_postgres::Error) -> Self {
        if let Some(db_err) = err.source()
            .and_then(|e| e.downcast_ref::<tokio_postgres::error::DbError>())
        {
            TuskError::Database {
                message: db_err.message().to_string(),
                code: Some(db_err.code().code().to_string()),
                position: db_err.position().map(|p| p as u32),
                hint: db_err.hint().map(|s| s.to_string()),
                detail: db_err.detail().map(|s| s.to_string()),
            }
        } else {
            TuskError::Database {
                message: err.to_string(),
                code: None,
                position: None,
                hint: None,
                detail: None,
            }
        }
    }
}
```

---

## 2. Application State Management

**Decision**: Use `tauri::State` with `Arc<RwLock<T>>` for read-heavy state and `Arc<Mutex<T>>` for write-heavy state.

**Rationale**:

- RwLock allows concurrent reads for schema cache and settings
- Connection pools are managed in a HashMap keyed by UUID
- State is initialized in `Builder::setup()` before any commands run

**Alternatives considered**:

- Global static state: Rejected due to testing difficulties
- Per-command state initialization: Rejected due to performance overhead

**Implementation Pattern**:

```rust
pub struct AppState {
    pub connections: Arc<RwLock<HashMap<Uuid, ConnectionPool>>>,
    pub active_queries: Arc<RwLock<HashMap<Uuid, QueryHandle>>>,
    pub storage: Arc<StorageService>,
    pub credentials: Arc<CredentialService>,
    pub log_dir: PathBuf,
}

impl AppState {
    pub fn new(app_dirs: &AppDirs) -> Result<Self, TuskError> {
        Ok(Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            storage: Arc::new(StorageService::new(&app_dirs.data_dir)?),
            credentials: Arc::new(CredentialService::new()),
            log_dir: app_dirs.log_dir.clone(),
        })
    }
}
```

**Tauri Setup**:

```rust
.setup(|app| {
    let app_dirs = AppDirs::new(app)?;
    let state = AppState::new(&app_dirs)?;
    app.manage(state);
    Ok(())
})
```

---

## 3. File-Based Logging with Rotation

**Decision**: Use `tracing-appender` with daily rotation, writing to both file and stdout in development, file-only in release.

**Rationale**:

- Users need access to logs for troubleshooting connection issues
- Daily rotation prevents unbounded log growth
- Non-blocking writes ensure logging doesn't impact performance

**Alternatives considered**:

- stdout-only: Rejected because users can't capture terminal output easily
- Custom log rotation: Rejected due to maintenance burden

**Implementation Pattern**:

```rust
pub struct LogGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

pub fn init_logging(log_dir: &Path, is_debug: bool) -> LogGuard {
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("tusk")
        .filename_suffix("log")
        .build(log_dir)
        .expect("failed to create log appender");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(if is_debug { "debug" } else { "info" }.parse().unwrap())
        .add_directive("tokio_postgres=info".parse().unwrap());

    if is_debug {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
            .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
            .with(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_env_filter(filter)
            .init();
    }

    LogGuard { _guard: guard }
}
```

**Log Location**: `{app_data_dir}/logs/tusk.YYYY-MM-DD.log`

---

## 4. OS Keychain Integration

**Decision**: Use `keyring` crate with service name pattern `tusk/conn:{connection_id}` for connection passwords.

**Rationale**:

- Cross-platform support (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Clear credential organization by connection ID
- Graceful fallback to session-only cache when keychain unavailable

**Alternatives considered**:

- Encrypted local file: Rejected per constitution (Principle III)
- Environment variables: Rejected due to persistence requirements

**Implementation Pattern**:

```rust
const SERVICE_NAME: &str = "tusk";

pub struct CredentialService {
    keychain_available: Arc<RwLock<bool>>,
}

impl CredentialService {
    pub fn new() -> Self {
        let available = keyring::Entry::new(SERVICE_NAME, "test")
            .map(|e| e.get_password().err() != Some(keyring::Error::NoBackendFound))
            .unwrap_or(false);

        Self {
            keychain_available: Arc::new(RwLock::new(available)),
        }
    }

    pub fn store_password(&self, conn_id: &Uuid, password: &str) -> Result<(), TuskError> {
        let entry = keyring::Entry::new(SERVICE_NAME, &format!("conn:{}", conn_id))
            .map_err(|e| TuskError::Credential(e.to_string()))?;

        entry.set_password(password)
            .map_err(|e| match e {
                keyring::Error::NoBackendFound =>
                    TuskError::Credential("Keychain unavailable".to_string()),
                _ => TuskError::Credential(e.to_string()),
            })
    }

    pub fn get_password(&self, conn_id: &Uuid) -> Result<Option<String>, TuskError> {
        let entry = keyring::Entry::new(SERVICE_NAME, &format!("conn:{}", conn_id))
            .map_err(|e| TuskError::Credential(e.to_string()))?;

        match entry.get_password() {
            Ok(pwd) => Ok(Some(pwd)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(keyring::Error::NoBackendFound) => {
                *self.keychain_available.write() = false;
                Err(TuskError::Credential("Keychain unavailable".to_string()))
            }
            Err(e) => Err(TuskError::Credential(e.to_string())),
        }
    }

    pub fn delete_password(&self, conn_id: &Uuid) -> Result<(), TuskError> {
        let entry = keyring::Entry::new(SERVICE_NAME, &format!("conn:{}", conn_id))
            .map_err(|e| TuskError::Credential(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already gone
            Err(e) => Err(TuskError::Credential(e.to_string())),
        }
    }
}
```

---

## 5. SQLite Corruption Detection and Repair

**Decision**: Use `PRAGMA integrity_check` for detection, attempt VACUUM/REINDEX for repair, backup corrupted file before reset.

**Rationale**:

- Users lose saved connections if database is reset without backup
- Automatic repair handles minor corruption without user intervention
- Backup naming with timestamp allows manual recovery

**Alternatives considered**:

- Silent reset: Rejected because user loses data without knowing why
- Require manual repair: Rejected due to poor UX

**Implementation Pattern**:

```rust
pub async fn open_or_repair(db_path: &Path) -> Result<rusqlite::Connection, TuskError> {
    match rusqlite::Connection::open(db_path) {
        Ok(conn) => {
            let integrity: String = conn.query_row(
                "PRAGMA integrity_check", [], |row| row.get(0)
            ).unwrap_or_else(|_| "error".to_string());

            if integrity == "ok" {
                return Ok(conn);
            }

            // Attempt repair
            if conn.execute_batch("VACUUM; REINDEX;").is_ok() {
                let recheck: String = conn.query_row(
                    "PRAGMA integrity_check", [], |row| row.get(0)
                ).unwrap_or_else(|_| "error".to_string());

                if recheck == "ok" {
                    tracing::warn!("Database repaired via VACUUM/REINDEX");
                    return Ok(conn);
                }
            }
            drop(conn);
        }
        Err(e) => tracing::error!("Failed to open database: {}", e),
    }

    // Backup and reset
    backup_corrupted(db_path)?;
    create_fresh(db_path)
}

fn backup_corrupted(db_path: &Path) -> Result<(), TuskError> {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup = db_path.with_file_name(format!(
        "{}.{}.corrupt",
        db_path.file_stem().unwrap().to_string_lossy(),
        timestamp
    ));

    std::fs::copy(db_path, &backup)
        .map_err(|e| TuskError::Storage(format!("Backup failed: {}", e)))?;

    tracing::warn!("Corrupted database backed up to: {:?}", backup);
    Ok(())
}

fn create_fresh(db_path: &Path) -> Result<rusqlite::Connection, TuskError> {
    std::fs::remove_file(db_path).ok();
    rusqlite::Connection::open(db_path)
        .map_err(|e| TuskError::Storage(format!("Create failed: {}", e)))
}
```

---

## 6. Query Cancellation

**Decision**: Use `tokio::select!` with timeout for automatic cancellation, store `CancelToken` per query for explicit cancellation.

**Rationale**:

- `select!` provides clean cancellation by dropping the query future
- Connection remains usable after cancellation
- Query tracking map enables frontend cancel button

**Alternatives considered**:

- CancelToken only: Rejected because cancellation is racy
- Statement timeout only: Rejected because user can't cancel early

**Implementation Pattern**:

```rust
pub struct QueryHandle {
    cancel_token: tokio_postgres::CancelToken,
    started_at: std::time::Instant,
}

impl QueryService {
    pub async fn execute(
        &self,
        state: &AppState,
        conn_id: Uuid,
        query_id: Uuid,
        sql: String,
        timeout_ms: u64,
    ) -> Result<QueryResult, TuskError> {
        let pool = state.connections.read()
            .get(&conn_id)
            .cloned()
            .ok_or_else(|| TuskError::Connection("Connection not found".to_string()))?;

        let client = pool.get().await
            .map_err(|e| TuskError::Connection(e.to_string()))?;

        // Register query for cancellation
        let handle = QueryHandle {
            cancel_token: client.cancel_token(),
            started_at: std::time::Instant::now(),
        };
        state.active_queries.write().insert(query_id, handle);

        // Execute with timeout
        let result = tokio::select! {
            rows = client.query(&sql, &[]) => {
                rows.map_err(TuskError::from)
            }
            _ = tokio::time::sleep(Duration::from_millis(timeout_ms)) => {
                Err(TuskError::QueryTimeout(timeout_ms))
            }
        };

        // Unregister query
        state.active_queries.write().remove(&query_id);

        result.map(|rows| QueryResult { rows })
    }

    pub async fn cancel(&self, state: &AppState, query_id: Uuid) -> Result<(), TuskError> {
        if let Some(handle) = state.active_queries.write().remove(&query_id) {
            // Send cancellation request (best-effort)
            let _ = handle.cancel_token.cancel_query(tokio_postgres::NoTls).await;
        }
        Ok(())
    }
}
```

---

## 7. Directory Initialization

**Decision**: Use `directories` crate to get platform-appropriate paths, create all required directories on first launch.

**Rationale**:

- Platform-appropriate paths (e.g., `~/Library/Application Support/com.tusk` on macOS)
- User doesn't need to manually create directories
- Clear error messages if creation fails

**Implementation Pattern**:

```rust
pub struct AppDirs {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl AppDirs {
    pub fn new(app: &tauri::App) -> Result<Self, TuskError> {
        let resolver = app.path();

        let data_dir = resolver.app_data_dir()
            .map_err(|e| TuskError::Initialization(e.to_string()))?;
        let log_dir = resolver.app_log_dir()
            .map_err(|e| TuskError::Initialization(e.to_string()))?;
        let config_dir = resolver.app_config_dir()
            .map_err(|e| TuskError::Initialization(e.to_string()))?;

        // Create all directories
        for dir in [&data_dir, &log_dir, &config_dir] {
            std::fs::create_dir_all(dir)
                .map_err(|e| TuskError::Initialization(
                    format!("Failed to create {:?}: {}", dir, e)
                ))?;
        }

        Ok(Self { data_dir, log_dir, config_dir })
    }
}
```

---

## Dependencies

All dependencies are already in `Cargo.toml` from 001-project-init:

| Crate              | Version | Purpose                      |
| ------------------ | ------- | ---------------------------- |
| thiserror          | 2.0     | Error type derivation        |
| tokio-postgres     | 0.7     | PostgreSQL async driver      |
| deadpool-postgres  | 0.14    | Connection pooling           |
| rusqlite           | 0.32    | Local SQLite storage         |
| keyring            | 3.6     | OS keychain integration      |
| tracing            | 0.1     | Structured logging           |
| tracing-subscriber | 0.3     | Log formatting and filtering |
| tracing-appender   | (add)   | File rotation                |
| directories        | 5       | Platform paths               |
| uuid               | 1.12    | Connection/query IDs         |
| chrono             | 0.4     | Timestamps                   |

**New dependency needed**: `tracing-appender = "0.2"` for file rotation.

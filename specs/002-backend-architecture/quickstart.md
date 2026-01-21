# Quickstart: Backend Architecture

**Feature**: 002-backend-architecture
**Purpose**: Get developers up to speed on implementing the backend service layer

---

## Prerequisites

### Rust Toolchain

```bash
# Verify Rust 1.80+
rustc --version  # Should be 1.80.0 or higher

# Update if needed
rustup update stable
```

### System Dependencies

**macOS**:
```bash
# No additional dependencies for keychain
# SQLite bundled via rusqlite
```

**Linux**:
```bash
# For keyring (Secret Service / D-Bus)
sudo apt-get install libdbus-1-dev  # Debian/Ubuntu
sudo dnf install dbus-devel          # Fedora
```

**Windows**:
```bash
# No additional dependencies
# Uses Windows Credential Manager
```

### Local PostgreSQL (for integration tests)

```bash
# Connection details (from CLAUDE.md)
Host: localhost
Port: 5432
User: brandon
Database: postgres

# Retrieve password
skate get tusk/postgres/password
```

---

## Project Structure After Implementation

```
crates/tusk_core/src/
├── lib.rs                 # Module exports
├── error.rs               # TuskError types (expand existing)
├── state.rs               # TuskState global state (new)
├── logging.rs             # Logging setup (new)
├── services/
│   ├── mod.rs
│   ├── connection.rs      # ConnectionPool management
│   ├── query.rs           # Query execution + cancellation
│   ├── storage.rs         # Local SQLite storage
│   └── credentials.rs     # OS keychain integration
└── models/
    ├── mod.rs
    ├── connection.rs      # ConnectionConfig, PoolStatus
    ├── query.rs           # QueryHandle, QueryResult
    └── history.rs         # QueryHistoryEntry
```

---

## Step 1: Add Dependencies

Update `crates/tusk_core/Cargo.toml`:

```toml
[dependencies]
thiserror.workspace = true
parking_lot.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true

# New dependencies for this feature
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "macros"] }
tokio-postgres = "0.7"
deadpool-postgres = "0.14"
tokio-util = "0.7"  # For CancellationToken
rusqlite = { version = "0.37", features = ["bundled"] }
keyring = { version = "3.6", features = ["apple-native", "windows-native", "sync-secret-service"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

---

## Step 2: Error Types (Expand Existing)

The existing `error.rs` has basic error types. Expand with PostgreSQL-specific variants:

```rust
// Add these variants to existing TuskError enum:
Query {
    message: String,
    detail: Option<String>,
    hint: Option<String>,
    position: Option<usize>,
    code: Option<String>,
},
Connection { message: String, source: Option<Box<dyn Error>> },
Authentication { message: String, hint: Option<String> },
Storage { message: String, source: Option<Box<dyn Error>> },
Keyring { message: String, hint: Option<String> },
PoolTimeout { message: String, waiting: usize },
```

See [contracts/error.md](./contracts/error.md) for full specification.

---

## Step 3: Connection Pooling

Create `services/connection.rs`:

```rust
use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use std::time::Duration;

pub struct ConnectionPool {
    id: Uuid,
    config: ConnectionConfig,
    pool: Pool,
    created_at: DateTime<Utc>,
}

impl ConnectionPool {
    pub async fn new(config: ConnectionConfig, password: &str) -> Result<Self, TuskError> {
        let mut pg_config = tokio_postgres::Config::new();
        pg_config.host(&config.host);
        pg_config.port(config.port);
        pg_config.dbname(&config.database);
        pg_config.user(&config.username);
        pg_config.password(password);
        pg_config.application_name("Tusk");
        pg_config.connect_timeout(Duration::from_secs(config.options.connect_timeout_secs as u64));
        pg_config.keepalives(true);
        pg_config.keepalives_idle(Duration::from_secs(60));

        let manager = deadpool_postgres::Manager::from_config(
            pg_config,
            tokio_postgres::NoTls,
            deadpool_postgres::ManagerConfig {
                recycling_method: deadpool_postgres::RecyclingMethod::Fast,
            },
        );

        let pool = Pool::builder(manager)
            .max_size(4)
            .wait_timeout(Some(Duration::from_secs(30)))  // FR-013a
            .create_timeout(Some(Duration::from_secs(10)))
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| TuskError::connection(e.to_string()))?;

        // Validate connection (FR-011)
        let client = pool.get().await
            .map_err(|e| TuskError::connection(format!("Failed to connect: {}", e)))?;
        client.execute("SELECT 1", &[]).await
            .map_err(|e| TuskError::connection(format!("Validation failed: {}", e)))?;

        Ok(Self {
            id: config.id,
            config,
            pool,
            created_at: Utc::now(),
        })
    }
}
```

See [contracts/connection.md](./contracts/connection.md) for full API.

---

## Step 4: Credential Service

Create `services/credentials.rs`:

```rust
use keyring::Entry;
use parking_lot::RwLock;
use std::collections::HashMap;

const KEYRING_SERVICE: &str = "dev.tusk.Tusk";

pub struct CredentialService {
    available: bool,
    fallback_reason: Option<String>,
    session_store: Option<RwLock<HashMap<String, String>>>,
}

impl CredentialService {
    pub fn new() -> Self {
        let (available, fallback_reason) = Self::check_availability();

        let session_store = if !available {
            tracing::warn!(reason = fallback_reason.as_deref(), "Keyring unavailable, using session storage");
            Some(RwLock::new(HashMap::new()))
        } else {
            None
        };

        Self { available, fallback_reason, session_store }
    }

    fn check_availability() -> (bool, Option<String>) {
        match Entry::new(KEYRING_SERVICE, "__availability_check__") {
            Ok(entry) => match entry.set_password("test") {
                Ok(()) => {
                    let _ = entry.delete_credential();
                    (true, None)
                }
                Err(e) => (false, Some(e.to_string())),
            },
            Err(e) => (false, Some(e.to_string())),
        }
    }

    pub fn store_password(&self, connection_id: Uuid, password: &str) -> Result<(), TuskError> {
        let key = format!("db:{}", connection_id);

        if let Some(ref store) = self.session_store {
            store.write().insert(key, password.to_string());
            return Ok(());
        }

        Entry::new(KEYRING_SERVICE, &key)
            .map_err(|e| TuskError::keyring(e.to_string(), None))?
            .set_password(password)
            .map_err(|e| TuskError::keyring(e.to_string(), None))
    }
}
```

See [contracts/credentials.md](./contracts/credentials.md) for full API.

---

## Step 5: Application State

Create `state.rs`:

```rust
use gpui::Global;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TuskState {
    connections: RwLock<HashMap<Uuid, Arc<ConnectionPool>>>,
    schema_caches: RwLock<HashMap<Uuid, SchemaCache>>,
    active_queries: RwLock<HashMap<Uuid, QueryHandle>>,
    storage: LocalStorage,
    data_dir: PathBuf,
    credential_service: CredentialService,
    tokio_runtime: tokio::runtime::Runtime,
}

impl Global for TuskState {}

impl TuskState {
    pub fn new() -> Result<Self, TuskError> {
        let data_dir = default_data_dir();
        init_data_dir(&data_dir)?;

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| TuskError::internal(format!("Failed to create runtime: {}", e)))?;

        let storage = LocalStorage::open(data_dir.clone())?;
        let credential_service = CredentialService::new();

        Ok(Self {
            connections: RwLock::new(HashMap::new()),
            schema_caches: RwLock::new(HashMap::new()),
            active_queries: RwLock::new(HashMap::new()),
            storage,
            data_dir,
            credential_service,
            tokio_runtime,
        })
    }

    pub fn spawn<F, T>(&self, future: F) -> tokio::task::JoinHandle<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.tokio_runtime.spawn(future)
    }
}
```

See [contracts/state.md](./contracts/state.md) for full API.

---

## Step 6: Logging Setup

Create `logging.rs`:

```rust
use tracing_appender::{non_blocking::WorkerGuard, rolling::{RollingFileAppender, Rotation}};
use tracing_subscriber::{fmt::writer::MakeWriterExt, EnvFilter};

pub struct LoggingGuard {
    _worker_guard: Option<WorkerGuard>,
}

pub fn init_logging(log_dir: &PathBuf, is_pty: bool) -> LoggingGuard {
    if is_pty {
        return init_stdout_logging();
    }

    match init_file_logging(log_dir) {
        Ok(guard) => LoggingGuard { _worker_guard: Some(guard) },
        Err(e) => {
            eprintln!("Failed to initialize file logging: {}. Using stdout.", e);
            init_stdout_logging()
        }
    }
}

fn init_file_logging(log_dir: &PathBuf) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("tusk")
        .filename_suffix("log")
        .build(log_dir)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    let combined = stdout.and(non_blocking);

    tracing_subscriber::fmt()
        .with_writer(combined)
        .with_env_filter(build_env_filter())
        .with_ansi(true)
        .init();

    Ok(guard)
}

fn build_env_filter() -> EnvFilter {
    EnvFilter::try_from_env("TUSK_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new(default_filter()))
}

fn default_filter() -> &'static str {
    #[cfg(debug_assertions)]
    { "debug,tusk=trace,tokio_postgres=warn" }
    #[cfg(not(debug_assertions))]
    { "info,tusk=info,tokio_postgres=warn" }
}
```

See [contracts/logging.md](./contracts/logging.md) for full API.

---

## Running Tests

```bash
# Unit tests
cargo test -p tusk_core

# Integration tests (requires local PostgreSQL)
cargo test -p tusk_core --test integration

# With logging
RUST_LOG=debug cargo test -p tusk_core -- --nocapture
```

---

## Common Patterns

### Accessing State from GPUI Component

```rust
impl Render for MyComponent {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<TuskState>();
        let connections = state.connection_ids();
        // ...
    }
}
```

### Running Database Operations

```rust
impl MyComponent {
    fn execute_query(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();
        let pool = state.get_connection(&self.connection_id).unwrap();
        let sql = self.sql.clone();

        cx.spawn(|this, mut cx| async move {
            let handle = QueryHandle::new(self.connection_id, &sql);
            let result = QueryService::execute(&pool, &sql, &handle).await;

            this.update(&mut cx, |this, cx| {
                this.handle_result(result, cx);
            });
        }).detach();
    }
}
```

### Error Display in UI

```rust
fn show_error(&self, error: &TuskError, cx: &mut Context<Self>) {
    let info = error.to_error_info();

    // Display error_type and message
    // Show hint if available
    // Optionally show technical_detail
}
```

---

## Reference Documentation

- [spec.md](./spec.md) - Feature specification
- [research.md](./research.md) - Technology decisions and rationale
- [data-model.md](./data-model.md) - Entity definitions and relationships
- [contracts/](./contracts/) - Full API contracts

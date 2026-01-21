# Application State Contract

**Module**: `tusk_core::state`
**Requirements**: FR-005 through FR-009

---

## TuskState

Central application state (FR-005). Implements `gpui::Global` for access from any component.

### Fields

| Field | Type | Thread Safety | Description |
|-------|------|---------------|-------------|
| `connections` | `RwLock<HashMap<Uuid, Arc<ConnectionPool>>>` | parking_lot::RwLock | Active connection pools (FR-006) |
| `schema_caches` | `RwLock<HashMap<Uuid, SchemaCache>>` | parking_lot::RwLock | Per-connection schema cache (FR-007) |
| `active_queries` | `RwLock<HashMap<Uuid, QueryHandle>>` | parking_lot::RwLock | Running queries (FR-008) |
| `storage` | `LocalStorage` | Internal sync | SQLite storage access |
| `data_dir` | `PathBuf` | Immutable | Application data directory |
| `credential_service` | `CredentialService` | Internal sync | OS keychain access |
| `tokio_runtime` | `tokio::runtime::Runtime` | Send + Sync | Async runtime for DB ops |

### Initialization

```rust
// Initialize application state (SC-002: < 100ms)
// Creates data directory if needed (FR-025)
// Uses OS-appropriate paths in production (FR-026)
// Uses local dev directory in debug builds (FR-027)
// Shows error with alternate location option if permissions fail (FR-027a)
fn new() -> Result<Self, TuskError>

// Initialize with custom data directory (for testing)
fn with_data_dir(data_dir: PathBuf) -> Result<Self, TuskError>
```

### Connection Management (FR-006)

```rust
// Add an active connection to state
fn add_connection(&self, pool: ConnectionPool) -> Uuid

// Get a connection by ID
fn get_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>>

// Remove a connection and its schema cache (FR-006 + FR-007 invariant)
fn remove_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>>

// List all connection IDs
fn connection_ids(&self) -> Vec<Uuid>

// Get status of all connection pools
fn all_pool_statuses(&self) -> HashMap<Uuid, PoolStatus>
```

### Schema Cache Management (FR-007)

```rust
fn get_schema_cache(&self, connection_id: &Uuid) -> Option<SchemaCache>
fn set_schema_cache(&self, connection_id: Uuid, cache: SchemaCache)
fn remove_schema_cache(&self, connection_id: &Uuid)
```

### Query Tracking (FR-008)

```rust
// Register a running query
fn register_query(&self, handle: QueryHandle) -> Uuid

// Get a query handle by ID
fn get_query(&self, id: &Uuid) -> Option<QueryHandle>

// Unregister a query (on completion or cancellation, FR-016)
fn unregister_query(&self, id: &Uuid) -> Option<QueryHandle>

// Cancel a specific query - only affects specified query (User Story 4, scenario 3)
fn cancel_query(&self, id: &Uuid) -> bool

// List all active query IDs
fn active_query_ids(&self) -> Vec<Uuid>
```

### Storage & Credentials Access

```rust
fn storage(&self) -> &LocalStorage
fn data_dir(&self) -> &PathBuf
fn credentials(&self) -> &CredentialService
```

### Async Runtime (FR-020, FR-021)

```rust
// Get handle to Tokio runtime
fn runtime(&self) -> &tokio::runtime::Runtime

// Spawn a future on the Tokio runtime
// UI remains responsive during database operations
fn spawn<F, T>(&self, future: F) -> tokio::task::JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static
```

---

## SchemaCache (Placeholder)

Schema cache per connection (defined in schema browser feature).

| Field | Type | Description |
|-------|------|-------------|
| `connection_id` | `Uuid` | Associated connection |
| `cached_at` | `DateTime<Utc>` | Cache timestamp |

---

## Invariants

1. Removing a connection removes its schema cache
2. Active queries trackable by ID
3. Thread-safe access from any component (FR-009)

## Thread Safety

TuskState is `Send + Sync` because:
- All mutable fields use `parking_lot::RwLock`
- Connection pools wrapped in `Arc`
- Internal services have their own synchronization

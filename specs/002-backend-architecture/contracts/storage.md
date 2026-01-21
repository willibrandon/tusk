# Local Storage Contract

**Module**: `tusk_core::services::storage`
**Requirements**: FR-025 through FR-027a

---

## LocalStorage

SQLite-based local storage for application data.

Stores saved connections, query history, saved queries, and UI state.
Credentials are NOT stored here—they go in OS keychain.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `connection` | `ThreadSafeConnection` | Thread-safe SQLite connection |
| `data_dir` | `PathBuf` | Data directory path |

### Constructors

```rust
// Open or create local storage database
// Creates data directory if needed (FR-025)
// Uses OS-appropriate paths in production (FR-026)
// Uses local dev directory in debug builds (FR-027)
// Shows error with options on permission failure (FR-027a)
fn open(data_dir: PathBuf) -> Result<Self, TuskError>

// Open with custom database path (for testing)
fn open_with_path(db_path: PathBuf) -> Result<Self, TuskError>
```

---

## Connection Operations

```rust
// Save a connection configuration (NOT password—use CredentialService)
fn save_connection(&self, config: &ConnectionConfig) -> Result<(), TuskError>

// Load a connection configuration by ID
fn load_connection(&self, id: Uuid) -> Result<Option<ConnectionConfig>, TuskError>

// Load all saved connections
fn load_all_connections(&self) -> Result<Vec<ConnectionConfig>, TuskError>

// Delete a connection configuration
fn delete_connection(&self, id: Uuid) -> Result<(), TuskError>

// Update last connected timestamp
fn update_last_connected(&self, id: Uuid) -> Result<(), TuskError>
```

---

## SSH Tunnel Operations

```rust
fn save_ssh_tunnel(&self, tunnel: &SshTunnelConfig) -> Result<(), TuskError>
fn load_ssh_tunnel(&self, id: Uuid) -> Result<Option<SshTunnelConfig>, TuskError>
fn load_all_ssh_tunnels(&self) -> Result<Vec<SshTunnelConfig>, TuskError>
fn delete_ssh_tunnel(&self, id: Uuid) -> Result<(), TuskError>
```

---

## Query History Operations

```rust
// Add a query to history
fn add_to_history(&self, entry: &QueryHistoryEntry) -> Result<i64, TuskError>

// Load recent history for a connection
fn load_history(&self, connection_id: Uuid, limit: usize) -> Result<Vec<QueryHistoryEntry>, TuskError>

// Load all recent history across connections
fn load_all_history(&self, limit: usize) -> Result<Vec<QueryHistoryEntry>, TuskError>

// Search history by SQL content
fn search_history(
    &self,
    query: &str,
    connection_id: Option<Uuid>,
    limit: usize,
) -> Result<Vec<QueryHistoryEntry>, TuskError>

// Clear history for a connection
fn clear_history(&self, connection_id: Uuid) -> Result<(), TuskError>

// Clear all history
fn clear_all_history(&self) -> Result<(), TuskError>
```

---

## Saved Queries Operations

```rust
fn save_query(&self, query: &SavedQuery) -> Result<(), TuskError>
fn load_saved_query(&self, id: Uuid) -> Result<Option<SavedQuery>, TuskError>
fn load_all_saved_queries(&self) -> Result<Vec<SavedQuery>, TuskError>
fn load_saved_queries_in_folder(&self, folder_path: &str) -> Result<Vec<SavedQuery>, TuskError>
fn delete_saved_query(&self, id: Uuid) -> Result<(), TuskError>
```

---

## UI State Operations

```rust
fn save_ui_state(&self, key: &str, value: &serde_json::Value) -> Result<(), TuskError>
fn load_ui_state(&self, key: &str) -> Result<Option<serde_json::Value>, TuskError>
fn delete_ui_state(&self, key: &str) -> Result<(), TuskError>
```

---

## SavedQuery Entity

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `connection_id` | `Option<Uuid>` | Associated connection (None = any) |
| `name` | `String` | Display name (1-255 chars) |
| `description` | `Option<String>` | Optional description |
| `sql` | `String` | The SQL text |
| `folder_path` | `Option<String>` | Folder path (e.g., "/Reports/Monthly") |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last update timestamp |

---

## Helper Functions

```rust
// Get the default data directory for the application
// Production (FR-026):
//   - macOS: ~/Library/Application Support/dev.tusk.Tusk
//   - Windows: %APPDATA%\tusk\Tusk
//   - Linux: ~/.local/share/tusk
// Debug builds (FR-027): ./tusk_data in current directory
fn default_data_dir() -> PathBuf

// Initialize data directory, handling permission errors (FR-027a)
// If directory cannot be created, returns error with options to:
//   - Select alternate location
//   - Exit application
fn init_data_dir(path: &PathBuf) -> Result<(), TuskError>
```

---

## Thread Safety

Uses Zed's `sqlez` pattern:
- Thread-local connections for reads (no mutex contention)
- Single background thread for writes (serializes, prevents WAL contention)
- Connections marked read-only by default

---

## Database PRAGMAs

| PRAGMA | Value | Rationale |
|--------|-------|-----------|
| `journal_mode` | WAL | Concurrent reads during writes |
| `synchronous` | NORMAL | Good durability/performance balance |
| `busy_timeout` | 5000 | Wait 5s for locks |
| `cache_size` | -64000 | 64MB page cache |
| `foreign_keys` | ON | Enforce referential integrity |
| `temp_store` | MEMORY | Temp tables in RAM |

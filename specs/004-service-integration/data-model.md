# Data Model: Service Integration Layer

**Feature**: 004-service-integration
**Date**: 2026-01-21
**Status**: Complete

## Entity Overview

This feature primarily defines **integration contracts** rather than new persistent entities. The data model focuses on runtime state, event types, and service interfaces that bridge UI components with backend services.

## Core Entities

### 1. TuskState (Enhanced)

**Purpose**: Application-wide state container accessible via GPUI Global trait

**Location**: `crates/tusk_core/src/state.rs`

| Field | Type | Description |
|-------|------|-------------|
| connections | `RwLock<HashMap<Uuid, ConnectionEntry>>` | Active connection pools with status |
| schema_caches | `RwLock<HashMap<Uuid, SchemaCache>>` | Cached schema per connection |
| active_queries | `RwLock<HashMap<Uuid, Arc<QueryHandle>>>` | Running queries for cancellation |
| storage | `LocalStorage` | SQLite persistence for metadata |
| credential_service | `CredentialService` | OS keychain access |
| tokio_runtime | `Runtime` | Tokio runtime for async operations |
| data_dir | `PathBuf` | Application data directory |

**New Methods Required**:
- `connect(&self, config: &ConnectionConfig, password: &str) -> Result<Uuid>`
- `disconnect(&self, connection_id: Uuid) -> Result<()>`
- `get_connection_status(&self, connection_id: Uuid) -> ConnectionStatus`
- `execute_query(&self, connection_id: Uuid, sql: &str) -> Result<QueryHandle>`
- `cancel_query(&self, query_id: Uuid) -> Result<()>`
- `get_schema(&self, connection_id: Uuid) -> Result<DatabaseSchema>`
- `refresh_schema(&self, connection_id: Uuid) -> Result<()>`

**Relationships**:
- Contains 0..* ConnectionEntry (active connections)
- Contains 0..* SchemaCache (one per connection)
- Contains 0..* QueryHandle (active queries)
- References 1 LocalStorage (persistence)
- References 1 CredentialService (credentials)

---

### 2. ConnectionEntry (New)

**Purpose**: Wrapper for connection pool with status tracking

**Location**: `crates/tusk_core/src/state.rs`

| Field | Type | Description |
|-------|------|-------------|
| config | `ConnectionConfig` | Connection settings (no password) |
| pool | `Arc<ConnectionPool>` | Pooled database connections |
| status | `ConnectionStatus` | Current connection state |
| connected_at | `DateTime<Utc>` | When connection was established |

**Validation Rules**:
- Pool must be validated before entry creation
- Status must reflect actual pool state

---

### 3. ConnectionStatus (Enhanced)

**Purpose**: Represents the current state of a database connection

**Location**: `crates/tusk_core/src/models/connection.rs`

| Variant | Fields | Description |
|---------|--------|-------------|
| Disconnected | - | No active connection |
| Connecting | - | Connection in progress |
| Connected | - | Active, healthy connection |
| Error | `message: String, recoverable: bool` | Connection failed |

**State Transitions**:
```
Disconnected → Connecting → Connected
                         → Error → Disconnected
Connected → Disconnected (user disconnect)
Connected → Error (connection lost)
Error → Connecting (retry)
```

---

### 4. QueryEvent (New)

**Purpose**: Stream events during query execution for progressive UI updates

**Location**: `crates/tusk_core/src/models/query.rs`

| Variant | Fields | Description |
|---------|--------|-------------|
| Columns | `columns: Vec<ColumnInfo>` | Column metadata (sent first) |
| Rows | `rows: Vec<Row>, total_so_far: usize` | Batch of rows with running count |
| Progress | `rows_so_far: usize` | Progress update (large queries) |
| Complete | `total_rows: usize, execution_time_ms: u64, rows_affected: Option<u64>` | Query finished |
| Error | `error: TuskError` | Query failed |

**Ordering Constraints**:
1. Columns always sent first
2. Rows sent in batches (default 1000)
3. Complete or Error sent exactly once, at end

---

### 5. SchemaCache (New)

**Purpose**: Cached database schema with TTL for cache invalidation

**Location**: `crates/tusk_core/src/models/schema.rs`

| Field | Type | Description |
|-------|------|-------------|
| schema | `DatabaseSchema` | Cached schema data |
| loaded_at | `Instant` | When cache was populated |
| ttl | `Duration` | Time-to-live (default 5 min) |

**Validation Rules**:
- `is_expired()` returns true if `loaded_at.elapsed() > ttl`
- Cache refresh resets `loaded_at`

---

### 6. ErrorInfo (Existing - Verified)

**Purpose**: User-facing error display information

**Location**: `crates/tusk_core/src/error.rs`

| Field | Type | Description |
|-------|------|-------------|
| error_type | `String` | Category (e.g., "Query Error") |
| message | `String` | User-friendly message |
| hint | `Option<String>` | Actionable suggestion |
| technical_detail | `Option<String>` | Debug information |
| position | `Option<usize>` | Character position for query errors |
| code | `Option<String>` | PostgreSQL error code |
| recoverable | `bool` | Whether user can retry |

---

### 7. QueryHandle (Existing - Verified)

**Purpose**: Handle for tracking and cancelling a running query

**Location**: `crates/tusk_core/src/models/query.rs`

| Field | Type | Description |
|-------|------|-------------|
| id | `Uuid` | Unique query identifier |
| connection_id | `Uuid` | Associated connection |
| sql | `String` | Query text |
| cancel_token | `CancellationToken` | For cancellation |
| started_at | `DateTime<Utc>` | Execution start time |

**Methods**:
- `cancel()` - Signal cancellation
- `is_cancelled()` - Check if cancelled
- `cancelled()` - Async wait for cancellation
- `elapsed_ms()` - Time since start

---

## UI State Entities

### 8. QueryEditorState (New)

**Purpose**: State for SQL editor component with service integration

**Location**: `crates/tusk_ui/src/query_editor.rs`

| Field | Type | Description |
|-------|------|-------------|
| content | `String` | Current SQL text |
| connection_id | `Option<Uuid>` | Active connection for this editor |
| active_query | `Option<Arc<QueryHandle>>` | Currently executing query |
| status | `QueryEditorStatus` | Idle, Executing, Cancelled |
| _execution_task | `Option<Task<()>>` | GPUI task for cancellation |

**QueryEditorStatus Enum**:
- `Idle` - Ready for input
- `Executing` - Query in progress
- `Cancelled` - Cancellation in progress

---

### 9. ResultsPanelState (New)

**Purpose**: State for query results grid with streaming support

**Location**: `crates/tusk_ui/src/panels/results.rs`

| Field | Type | Description |
|-------|------|-------------|
| columns | `Vec<ColumnInfo>` | Column metadata for grid |
| rows | `Vec<Row>` | Accumulated result rows |
| total_rows | `usize` | Total rows received |
| execution_time_ms | `Option<u64>` | Query execution time |
| status | `ResultsStatus` | Loading, Streaming, Complete, Error |
| error | `Option<ErrorInfo>` | Error if failed |
| _stream_subscription | `Option<Subscription>` | For receiving QueryEvents |

**ResultsStatus Enum**:
- `Empty` - No results to display
- `Loading` - Waiting for first batch
- `Streaming` - Receiving batches
- `Complete` - All results received
- `Error` - Query failed

---

### 10. SchemaBrowserState (Enhanced)

**Purpose**: State for schema tree browser with cached data

**Location**: `crates/tusk_ui/src/panels/schema_browser.rs`

| Field | Type | Description |
|-------|------|-------------|
| connection_id | `Option<Uuid>` | Current connection |
| tree_items | `Vec<TreeItem>` | Rendered tree nodes |
| loading | `bool` | Schema loading in progress |
| expanded_nodes | `HashSet<String>` | Expanded tree paths |
| selected_node | `Option<String>` | Currently selected item |
| _load_task | `Option<Task<()>>` | For cancellation on refresh |

---

## Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         TuskState (Global)                       │
├─────────────────────────────────────────────────────────────────┤
│  connections: HashMap<Uuid, ConnectionEntry>                     │
│  schema_caches: HashMap<Uuid, SchemaCache>                       │
│  active_queries: HashMap<Uuid, QueryHandle>                      │
│  storage: LocalStorage                                           │
│  credential_service: CredentialService                           │
│  tokio_runtime: Runtime                                          │
└─────────────────────────────────────────────────────────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ConnectionEntry│   │  SchemaCache  │   │  QueryHandle  │
├───────────────┤   ├───────────────┤   ├───────────────┤
│config         │   │schema         │   │id             │
│pool           │   │loaded_at      │   │connection_id  │
│status         │   │ttl            │   │sql            │
│connected_at   │   └───────────────┘   │cancel_token   │
└───────────────┘           │           │started_at     │
        │                   ▼           └───────────────┘
        ▼           ┌───────────────┐           │
┌───────────────┐   │DatabaseSchema │           ▼
│ConnectionPool │   ├───────────────┤   ┌───────────────┐
├───────────────┤   │schemas        │   │  QueryEvent   │
│deadpool inner │   │tables         │   ├───────────────┤
│config         │   │views          │   │Columns(...)   │
└───────────────┘   │functions      │   │Rows(...)      │
                    │table_columns  │   │Progress(...)  │
                    └───────────────┘   │Complete(...)  │
                                        │Error(...)     │
                                        └───────────────┘
```

## UI Component to State Mapping

| UI Component | State Entity | TuskState Access |
|--------------|--------------|------------------|
| QueryEditor | QueryEditorState | `execute_query()`, `cancel_query()` |
| ResultsPanel | ResultsPanelState | Receives QueryEvent stream |
| SchemaBrowser | SchemaBrowserState | `get_schema()`, `refresh_schema()` |
| ConnectionDialog | (form state) | `connect()`, `disconnect()` |
| StatusBar | (direct read) | `get_connection_status()` |
| ToastLayer | (receives ErrorInfo) | Error display |
| ErrorPanel | (receives ErrorInfo) | Detailed error display |

## Persistence Mapping

| Entity | Storage | Location |
|--------|---------|----------|
| ConnectionConfig | SQLite | LocalStorage.saved_connections |
| Password | OS Keychain | CredentialService |
| QueryHistoryEntry | SQLite | LocalStorage.query_history |
| WorkspaceLayout | SQLite | LocalStorage.ui_state |
| SchemaCache | Memory only | Not persisted |
| QueryHandle | Memory only | Not persisted |
| ConnectionEntry | Memory only | Not persisted |

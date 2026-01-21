# Data Model: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-20
**Source**: [spec.md](./spec.md) Key Entities section + Functional Requirements

---

## Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION STATE                               │
│                                 (TuskState)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  connections: HashMap<Uuid, ConnectionPool>                                  │
│  schema_caches: HashMap<Uuid, SchemaCache>                                  │
│  active_queries: HashMap<Uuid, QueryHandle>                                 │
│  storage: LocalStorage                                                       │
│  data_dir: PathBuf                                                          │
│  credential_service: CredentialService                                      │
└─────────────────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ ConnectionPool  │  │   SchemaCache   │  │   QueryHandle   │
├─────────────────┤  ├─────────────────┤  ├─────────────────┤
│ id: Uuid        │  │ connection_id   │  │ id: Uuid        │
│ config          │  │ tables          │  │ cancel_token    │
│ pool            │  │ views           │  │ started_at      │
│ status          │  │ functions       │  │ sql             │
└─────────────────┘  │ cached_at       │  └─────────────────┘
         │           └─────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CONNECTION CONFIG                                  │
│                         (ConnectionConfig)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  id: Uuid                    -- Unique identifier                           │
│  name: String                -- Display name                                │
│  host: String                -- Server hostname                             │
│  port: u16                   -- Server port (default 5432)                  │
│  database: String            -- Database name                               │
│  username: String            -- Login username                              │
│  ssl_mode: SslMode           -- SSL configuration                           │
│  ssh_tunnel: Option<SshTunnelConfig>  -- Optional tunnel                    │
│  options: ConnectionOptions  -- Timeouts, read-only, etc.                   │
│  color: Option<String>       -- UI accent color                             │
└─────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              LOCAL STORAGE                                   │
│                              (SQLite)                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  connections        -- Saved connection configurations                       │
│  ssh_tunnels        -- SSH tunnel configurations                            │
│  query_history      -- Executed query records                               │
│  saved_queries      -- User's query library                                 │
│  ui_state           -- Persistent UI preferences                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Entities

### TuskError

Represents all possible error conditions in the application.

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| variant | enum | Error category | See variants below |
| message | String | Human-readable error description | Required, non-empty |
| detail | Option<String> | Technical details for debugging | Optional |
| hint | Option<String> | Actionable suggestion for user | Optional |
| position | Option<usize> | Position in SQL query (1-indexed) | Optional, for query errors |
| code | Option<String> | PostgreSQL error code (e.g., "42P01") | Optional, for PG errors |

**Variants (FR-001):**

| Variant | Source | Example Hint |
|---------|--------|--------------|
| Connection | Network/socket failures | "Check that the database server is running" |
| Authentication | Invalid credentials | "Check username and password" |
| Ssl | Certificate/TLS errors | "Verify SSL certificate configuration" |
| Ssh | Tunnel failures | "Check SSH key permissions" |
| Query | SQL execution errors | "Check SQL syntax near position X" |
| Storage | Local SQLite errors | "Data directory may be corrupted" |
| Keyring | OS keychain errors | "Grant Tusk access in system preferences" |
| PoolTimeout | Pool exhausted | "Try closing unused connections" |
| Internal | Unexpected errors | "Please report this issue" |

**State Transitions:** N/A (immutable after creation)

---

### ErrorInfo

User-displayable error information for UI presentation (FR-003).

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| error_type | String | Category name (e.g., "Connection Error") | Required |
| message | String | User-friendly message | Required |
| hint | Option<String> | Actionable suggestion | Required where applicable |
| technical_detail | Option<String> | For "Show Details" expansion | Optional |

**Derived from:** `TuskError::to_error_info()`

---

### TuskState

Central application state (FR-005 through FR-009).

| Field | Type | Description | Thread Safety |
|-------|------|-------------|---------------|
| connections | RwLock<HashMap<Uuid, ConnectionPool>> | Active connection pools | parking_lot::RwLock |
| schema_caches | RwLock<HashMap<Uuid, SchemaCache>> | Per-connection schema cache | parking_lot::RwLock |
| active_queries | RwLock<HashMap<Uuid, QueryHandle>> | Running queries | parking_lot::RwLock |
| storage | LocalStorage | SQLite storage access | Internal synchronization |
| data_dir | PathBuf | Application data directory | Immutable after init |
| credential_service | CredentialService | OS keychain access | Internal serialization |
| tokio_runtime | Runtime | Async runtime for DB ops | Send + Sync |

**Implements:** `gpui::Global`

**Invariants:**
- Removing a connection removes its schema cache (FR-006, FR-007)
- Active queries trackable by ID (FR-008)
- Thread-safe access from any component (FR-009)

---

### ConnectionConfig

Configuration for a database connection (FR-012).

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | Uuid | Unique identifier | Auto-generated |
| name | String | Display name | Required, 1-255 chars |
| host | String | Server hostname or IP | Required, valid hostname |
| port | u16 | Server port | Required, default 5432 |
| database | String | Database name | Required, 1-63 chars |
| username | String | Login username | Required |
| ssl_mode | SslMode | SSL configuration | Required, default Prefer |
| ssh_tunnel | Option<SshTunnelConfig> | SSH tunnel settings | Optional |
| options | ConnectionOptions | Additional options | Required |
| color | Option<String> | UI accent color | Optional, hex format |

**SslMode enum:**
- `Disable` - No SSL
- `Prefer` - Use SSL if available (default)
- `Require` - Require SSL, accept any certificate
- `VerifyCa` - Require SSL, verify CA
- `VerifyFull` - Require SSL, verify CA and hostname

**Password:** NOT stored in config—retrieved from OS keychain via `credential_service.get_password(id)`

---

### SshTunnelConfig

SSH tunnel configuration for secure remote access.

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | Uuid | Unique identifier | Auto-generated |
| name | String | Display name | Required |
| host | String | SSH server hostname | Required |
| port | u16 | SSH server port | Required, default 22 |
| username | String | SSH username | Required |
| auth_method | SshAuthMethod | Authentication method | Required |
| key_path | Option<PathBuf> | Path to private key | Required if auth=Key |

**SshAuthMethod enum:**
- `Key` - Private key authentication
- `Password` - Password authentication (from keychain)
- `Agent` - SSH agent authentication

---

### ConnectionOptions

Additional connection options.

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| connect_timeout_secs | u32 | Connection timeout | Default 10 |
| statement_timeout_secs | Option<u32> | Query timeout | Optional |
| read_only | bool | Read-only mode | Default false |
| application_name | String | Application identifier | Default "Tusk" |

---

### ConnectionPool

A managed pool of database connections (FR-010, FR-011, FR-013).

| Field | Type | Description |
|-------|------|-------------|
| id | Uuid | Matches ConnectionConfig.id |
| config | ConnectionConfig | Original configuration |
| pool | deadpool_postgres::Pool | The actual connection pool |
| created_at | DateTime<Utc> | Pool creation time |

**Methods:**
- `get() -> Result<Connection>` - Acquire connection from pool
- `status() -> PoolStatus` - Get current pool status
- `close()` - Close pool and all connections

---

### PoolStatus

Connection pool status reporting (FR-013).

| Field | Type | Description |
|-------|------|-------------|
| max_size | usize | Maximum pool capacity |
| size | usize | Current connections (idle + active) |
| available | isize | Idle connections (can be negative during contention) |
| waiting | usize | Tasks waiting for connections |

---

### QueryHandle

Handle for tracking a running query (FR-014, FR-015, FR-016).

| Field | Type | Description |
|-------|------|-------------|
| id | Uuid | Unique query identifier |
| connection_id | Uuid | Associated connection |
| sql | String | The SQL being executed |
| cancel_token | CancellationToken | For cancellation |
| started_at | DateTime<Utc> | Execution start time |

**Methods:**
- `cancel()` - Request query cancellation
- `is_cancelled() -> bool` - Check if cancelled

**State Transitions:**
```
Running -> Completed (normal finish)
Running -> Cancelled (user cancelled)
Running -> Failed (error occurred)
```

---

### QueryResult

Results from query execution.

| Field | Type | Description |
|-------|------|-------------|
| query_id | Uuid | The query handle ID |
| columns | Vec<ColumnInfo> | Column metadata |
| rows | Vec<Row> | Result rows |
| rows_affected | Option<u64> | For INSERT/UPDATE/DELETE |
| execution_time_ms | u64 | Time to execute |
| query_type | QueryType | SELECT/INSERT/etc. |

**ColumnInfo:**
| Field | Type | Description |
|-------|------|-------------|
| name | String | Column name |
| type_oid | u32 | PostgreSQL type OID |
| type_name | String | Human-readable type |

**QueryType enum:**
- `Select` - Returns rows
- `Insert` - Insert operation
- `Update` - Update operation
- `Delete` - Delete operation
- `Other` - DDL, COPY, etc.

---

### QueryHistoryEntry

Record of a previously executed query (for local storage).

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | i64 | Auto-increment ID | Auto-generated |
| connection_id | Uuid | Associated connection | Required |
| sql | String | The executed SQL | Required |
| execution_time_ms | Option<u64> | Time to execute | Optional |
| row_count | Option<u64> | Rows returned/affected | Optional |
| error_message | Option<String> | Error if failed | Optional |
| executed_at | DateTime<Utc> | Execution timestamp | Auto-generated |

---

### CredentialService

OS keychain integration (FR-017, FR-018, FR-019, FR-019a).

| Field | Type | Description |
|-------|------|-------------|
| available | bool | Whether keychain is accessible |
| fallback_reason | Option<String> | Why keychain unavailable |
| session_store | Option<RwLock<HashMap<String, String>>> | In-memory fallback |

**Methods:**
- `is_available() -> bool` - Check keychain accessibility
- `store_password(connection_id, password) -> Result<()>` - Store credential
- `get_password(connection_id) -> Result<Option<String>>` - Retrieve credential
- `delete_password(connection_id) -> Result<()>` - Remove credential
- `has_password(connection_id) -> Result<bool>` - Check existence

**Service Name:** `dev.tusk.Tusk`
**Username Format:** `db:{connection_id}`

---

### LocalStorage

SQLite-based local storage (FR-025, FR-026, FR-027, FR-027a).

| Field | Type | Description |
|-------|------|-------------|
| connection | ThreadSafeConnection | SQLite connection |
| data_dir | PathBuf | Data directory path |

**Tables:**
- `connections` - Saved connection configurations
- `ssh_tunnels` - SSH tunnel configurations
- `query_history` - Executed query records
- `saved_queries` - User's query library
- `ui_state` - Persistent UI state (key-value JSON)
- `migrations` - Migration tracking

---

## SQLite Schema

```sql
-- Saved Connections (credentials in OS keychain, NOT here)
CREATE TABLE connections (
    connection_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 5432,
    database TEXT NOT NULL,
    username TEXT NOT NULL,
    ssl_mode TEXT NOT NULL DEFAULT 'prefer',
    ssh_tunnel_id TEXT,
    color TEXT,
    read_only INTEGER NOT NULL DEFAULT 0,
    connect_timeout_secs INTEGER NOT NULL DEFAULT 10,
    statement_timeout_secs INTEGER,
    application_name TEXT NOT NULL DEFAULT 'Tusk',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_connected_at TEXT,
    FOREIGN KEY(ssh_tunnel_id) REFERENCES ssh_tunnels(tunnel_id) ON DELETE SET NULL
) STRICT;

-- SSH Tunnel Configurations
CREATE TABLE ssh_tunnels (
    tunnel_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_method TEXT NOT NULL,
    key_path TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- Query History
CREATE TABLE query_history (
    history_id INTEGER PRIMARY KEY,
    connection_id TEXT NOT NULL,
    sql_text TEXT NOT NULL,
    execution_time_ms INTEGER,
    row_count INTEGER,
    error_message TEXT,
    executed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(connection_id) REFERENCES connections(connection_id) ON DELETE CASCADE
) STRICT;

-- Saved Queries
CREATE TABLE saved_queries (
    query_id TEXT PRIMARY KEY,
    connection_id TEXT,
    name TEXT NOT NULL,
    description TEXT,
    sql_text TEXT NOT NULL,
    folder_path TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(connection_id) REFERENCES connections(connection_id) ON DELETE SET NULL
) STRICT;

-- UI State Persistence
CREATE TABLE ui_state (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- Migration Tracking
CREATE TABLE migrations (
    domain TEXT NOT NULL,
    step INTEGER NOT NULL,
    migration TEXT NOT NULL,
    PRIMARY KEY(domain, step)
) STRICT;

-- Indexes
CREATE INDEX idx_connections_last_connected ON connections(last_connected_at DESC);
CREATE INDEX idx_query_history_connection ON query_history(connection_id, executed_at DESC);
CREATE INDEX idx_query_history_executed ON query_history(executed_at DESC);
CREATE INDEX idx_saved_queries_folder ON saved_queries(folder_path);
```

---

## Validation Rules Summary

| Entity | Field | Rule |
|--------|-------|------|
| ConnectionConfig | name | 1-255 characters |
| ConnectionConfig | database | 1-63 characters (PostgreSQL limit) |
| ConnectionConfig | port | 1-65535 |
| ConnectionConfig | color | Optional, hex format (#RRGGBB) |
| QueryHistoryEntry | sql | Non-empty |
| SavedQuery | name | 1-255 characters |
| SshTunnelConfig | key_path | Must exist if auth_method = Key |

---

## State Transitions

### ConnectionPool Lifecycle

```
                    ┌──────────────┐
                    │   Created    │
                    └──────┬───────┘
                           │ validate()
                           ▼
         ┌─────────────────────────────────┐
         │            Active               │
         │  (connections available)        │
         └─────────────────┬───────────────┘
                           │ close()
                           ▼
                    ┌──────────────┐
                    │    Closed    │
                    └──────────────┘
```

### QueryHandle Lifecycle

```
                    ┌──────────────┐
                    │   Running    │
                    └──────┬───────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
   ┌────────────┐  ┌────────────┐  ┌────────────┐
   │ Completed  │  │ Cancelled  │  │   Failed   │
   └────────────┘  └────────────┘  └────────────┘
```

# Data Model: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-19

## Overview

This document defines the core data structures for Tusk's backend architecture. These models form the foundation for all subsequent features.

---

## Core Entities

### 1. ConnectionConfig

Represents a saved database connection configuration.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | UUID | Yes | Unique identifier |
| name | String | Yes | User-friendly display name |
| host | String | Yes | PostgreSQL server hostname |
| port | u16 | Yes | PostgreSQL server port (default: 5432) |
| database | String | Yes | Database name |
| username | String | Yes | Authentication username |
| ssl_mode | SslMode | Yes | SSL/TLS mode (disable/prefer/require/verify-ca/verify-full) |
| ssl_ca_cert | Option<String> | No | Path to CA certificate for verify-ca/verify-full |
| ssh_tunnel | Option<SshTunnel> | No | SSH tunnel configuration if connecting via SSH |
| read_only | bool | Yes | Whether connection should enforce read-only mode |
| statement_timeout_ms | Option<u64> | No | Default statement timeout in milliseconds |
| created_at | DateTime | Yes | Creation timestamp |
| updated_at | DateTime | Yes | Last modification timestamp |

**Validation Rules**:
- `name` must be non-empty and unique
- `host` must be a valid hostname or IP address
- `port` must be in range 1-65535
- `username` must be non-empty
- `database` must be non-empty
- Passwords are NOT stored in this model (stored in OS keychain)

**State Transitions**: N/A (static configuration)

---

### 2. SshTunnel

SSH tunnel configuration for secure remote connections.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| host | String | Yes | SSH server hostname |
| port | u16 | Yes | SSH server port (default: 22) |
| username | String | Yes | SSH username |
| auth_method | SshAuthMethod | Yes | Authentication method |
| local_port | Option<u16> | No | Local port for tunnel (auto-assigned if not specified) |

**SshAuthMethod Enum**:
- `Password` - Password stored in OS keychain
- `KeyFile { path: String }` - Private key file path (passphrase in keychain if encrypted)
- `Agent` - SSH agent forwarding

---

### 3. ConnectionPool

Represents an active database connection pool (runtime only, not persisted).

| Field | Type | Description |
|-------|------|-------------|
| config_id | UUID | Reference to ConnectionConfig |
| pool | deadpool_postgres::Pool | Connection pool instance |
| ssh_tunnel | Option<SshTunnelHandle> | Active SSH tunnel if used |
| created_at | Instant | When pool was created |
| last_used | AtomicInstant | Last query execution time |

**State Transitions**:
- `Connecting` → `Connected` | `Failed`
- `Connected` → `Disconnecting` → `Disconnected`
- `Failed` → `Disconnected` (cleanup)

---

### 4. Query

Represents a running database query (runtime only, not persisted).

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Unique query identifier |
| connection_id | UUID | Reference to ConnectionPool |
| sql | String | SQL query text |
| cancel_token | CancelToken | Token for query cancellation |
| started_at | Instant | Query start time |
| status | QueryStatus | Current execution status |

**QueryStatus Enum**:
- `Running` - Query is executing
- `Completed { rows: usize, elapsed_ms: u64 }` - Query finished successfully
- `Cancelled` - Query was cancelled by user
- `Timeout { elapsed_ms: u64 }` - Query exceeded timeout
- `Failed { error: TuskError }` - Query encountered an error

**State Transitions**:
- `Running` → `Completed` | `Cancelled` | `Timeout` | `Failed`

---

### 5. ErrorResponse

Structured error information for frontend display.

| Field | Type | Description |
|-------|------|-------------|
| kind | ErrorKind | Error category |
| message | String | Human-readable error message |
| detail | Option<String> | Additional technical detail |
| hint | Option<String> | Actionable suggestion for resolution |
| position | Option<u32> | Character position in SQL (for syntax errors) |
| code | Option<String> | PostgreSQL error code (e.g., "42P01") |

**ErrorKind Enum**:
- `Database` - PostgreSQL server error
- `Connection` - Connection establishment/pool error
- `Storage` - Local SQLite storage error
- `Credential` - Keychain access error
- `QueryCancelled` - User-initiated cancellation
- `QueryTimeout` - Statement timeout exceeded
- `Initialization` - Application startup error
- `Validation` - Input validation error
- `Internal` - Unexpected internal error

---

### 6. AppState

Application-level state container (runtime only).

| Field | Type | Description |
|-------|------|-------------|
| connections | RwLock<HashMap<UUID, ConnectionPool>> | Active connection pools |
| active_queries | RwLock<HashMap<UUID, QueryHandle>> | Currently executing queries |
| storage | StorageService | Local SQLite storage |
| credentials | CredentialService | OS keychain access |
| log_dir | PathBuf | Log file directory |

---

### 7. AppInfo

Application metadata for health checks.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Application name ("tusk") |
| version | String | Application version |
| tauri_version | String | Tauri runtime version |
| platform | String | OS platform (macos/windows/linux) |

---

### 8. DatabaseHealth

SQLite database health status.

| Field | Type | Description |
|-------|------|-------------|
| is_healthy | bool | Whether database passed integrity check |
| errors | Vec<String> | List of integrity errors (empty if healthy) |
| backup_path | Option<PathBuf> | Path to backup if repair failed |

---

## Local Storage Schema (SQLite)

### connections Table

```sql
CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY NOT NULL,          -- UUID as text
    name TEXT NOT NULL UNIQUE,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 5432,
    database TEXT NOT NULL,
    username TEXT NOT NULL,
    ssl_mode TEXT NOT NULL DEFAULT 'prefer',
    ssl_ca_cert TEXT,
    ssh_tunnel_json TEXT,                  -- JSON serialized SshTunnel
    read_only INTEGER NOT NULL DEFAULT 0,  -- boolean as int
    statement_timeout_ms INTEGER,
    created_at TEXT NOT NULL,              -- ISO 8601 timestamp
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_connections_name ON connections(name);
```

### preferences Table

```sql
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,                   -- JSON serialized value
    updated_at TEXT NOT NULL
);
```

### migrations Table

```sql
CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    applied_at TEXT NOT NULL
);
```

---

## Relationships

```
ConnectionConfig (persisted)
    │
    └──► ConnectionPool (runtime)
            │
            ├──► Query (runtime)
            │       │
            │       └──► ErrorResponse (if failed)
            │
            └──► SshTunnelHandle (runtime, optional)

AppState
    │
    ├──► HashMap<UUID, ConnectionPool>
    ├──► HashMap<UUID, QueryHandle>
    ├──► StorageService (SQLite)
    └──► CredentialService (OS Keychain)
```

---

## Credential Storage (OS Keychain)

Credentials are stored in the OS keychain using the `keyring` crate with the following key patterns:

| Credential Type | Service | Username |
|-----------------|---------|----------|
| Database password | `tusk` | `conn:{connection_id}` |
| SSH password | `tusk` | `ssh:{connection_id}` |
| SSH key passphrase | `tusk` | `ssh_key:{connection_id}` |

Credentials are NEVER stored in SQLite or configuration files per Constitution Principle III.

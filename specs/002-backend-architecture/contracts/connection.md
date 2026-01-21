# Connection Pooling Contract

**Module**: `tusk_core::services::connection`
**Requirements**: FR-010 through FR-013a

---

## ConnectionConfig

Configuration for a database connection (FR-012).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Uuid` | Auto-generated | Unique identifier |
| `name` | `String` | Required | Display name (1-255 chars) |
| `host` | `String` | Required | Server hostname or IP |
| `port` | `u16` | 5432 | Server port |
| `database` | `String` | Required | Database name (1-63 chars) |
| `username` | `String` | Required | Login username |
| `ssl_mode` | `SslMode` | `Prefer` | SSL configuration |
| `ssh_tunnel` | `Option<SshTunnelConfig>` | None | SSH tunnel settings |
| `options` | `ConnectionOptions` | Default | Additional options |
| `color` | `Option<String>` | None | UI accent color (hex format) |

**Note**: Password is NOT stored in config. Retrieved from `CredentialService`.

### Constructors

```rust
// Quick constructor with defaults
fn new(name, host, database, username) -> Self

// Builder for complex configurations
fn builder() -> ConnectionConfigBuilder

// Validate configuration
fn validate(&self) -> Result<(), TuskError>
```

---

## SslMode

SSL mode enum (per PostgreSQL conventions).

| Value | Description |
|-------|-------------|
| `Disable` | No SSL |
| `Prefer` | Use SSL if available (default) |
| `Require` | Require SSL, accept any certificate |
| `VerifyCa` | Require SSL, verify CA |
| `VerifyFull` | Require SSL, verify CA and hostname |

---

## SshTunnelConfig

SSH tunnel configuration for secure remote access.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Uuid` | Auto-generated | Unique identifier |
| `name` | `String` | Required | Display name |
| `host` | `String` | Required | SSH server hostname |
| `port` | `u16` | 22 | SSH server port |
| `username` | `String` | Required | SSH username |
| `auth_method` | `SshAuthMethod` | Required | Authentication method |
| `key_path` | `Option<PathBuf>` | None | Path to private key |

### SshAuthMethod

| Value | Description |
|-------|-------------|
| `Key` | Private key authentication |
| `Password` | Password authentication (from keychain) |
| `Agent` | SSH agent authentication |

---

## ConnectionOptions

Additional connection options.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `connect_timeout_secs` | `u32` | 10 | Connection timeout |
| `statement_timeout_secs` | `Option<u32>` | None | Query timeout |
| `read_only` | `bool` | false | Read-only mode |
| `application_name` | `String` | "Tusk" | Application identifier |

---

## PoolStatus

Connection pool status (FR-013).

| Field | Type | Description |
|-------|------|-------------|
| `max_size` | `usize` | Maximum pool capacity |
| `size` | `usize` | Current connections (idle + active) |
| `available` | `isize` | Idle connections (can be negative during contention) |
| `waiting` | `usize` | Tasks waiting for connections |

---

## ConnectionPool

A managed pool of database connections (FR-010, FR-011).

### Pool Configuration

| Setting | Value | Rationale |
|---------|-------|-----------|
| Max size | 4 connections | Desktop single-user workload |
| Wait timeout | 30 seconds | FR-013a (configurable) |
| Create timeout | 10 seconds | From ConnectionOptions |
| Recycle timeout | 5 seconds | Fast validation |
| Recycling method | Fast | Adequate for stable networks |

### Constructors

```rust
// Create a new connection pool (FR-010)
// Tests connection validity on creation (FR-011)
// Pool creation completes within configured timeout (SC-003)
async fn new(config: ConnectionConfig, password: &str) -> Result<Self, TuskError>

// Create with custom pool settings (for testing)
async fn with_pool_config(
    config: ConnectionConfig,
    password: &str,
    max_size: usize,
    wait_timeout: Duration,
) -> Result<Self, TuskError>
```

### Methods

```rust
fn id(&self) -> Uuid                           // Pool's unique identifier
fn config(&self) -> &ConnectionConfig          // Connection configuration

// Acquire a connection from the pool
// Waits up to 30 seconds if pool exhausted (FR-013a)
// Returns PoolTimeout error if timeout exceeded
async fn get(&self) -> Result<PooledConnection, TuskError>

// Get current pool status (FR-013, SC-010)
fn status(&self) -> PoolStatus

async fn close(&self)                          // Close pool and all connections
fn is_closed(&self) -> bool                    // Check if pool is closed
```

---

## PooledConnection

A connection acquired from the pool. Returns to pool when dropped.

### Methods

```rust
// Execute a query returning rows
async fn query(
    &self,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Row>, TuskError>

// Execute a query returning affected row count
async fn execute(
    &self,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64, TuskError>

// Prepare a statement
async fn prepare(&self, sql: &str) -> Result<Statement, TuskError>

// Begin a transaction
async fn transaction(&mut self) -> Result<Transaction<'_>, TuskError>
```

---

## Transaction

A database transaction.

```rust
async fn query(&self, sql, params) -> Result<Vec<Row>, TuskError>
async fn execute(&self, sql, params) -> Result<u64, TuskError>
async fn commit(self) -> Result<(), TuskError>
async fn rollback(self) -> Result<(), TuskError>
```

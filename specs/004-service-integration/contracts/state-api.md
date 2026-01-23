# Contract: TuskState API

**Feature**: 004-service-integration
**Type**: Internal Rust API (no REST/RPC)
**Location**: `crates/tusk_core/src/state.rs`

## Overview

TuskState is the central application state container accessed via GPUI's Global trait. This contract defines the methods that UI components use to interact with backend services.

## Access Pattern

```rust
// From any GPUI component with Context
let state = cx.global::<TuskState>();
```

## Connection Management API

### connect

Establishes a new database connection and adds it to the pool.

```rust
pub async fn connect(
    &self,
    config: &ConnectionConfig,
    password: &str,
) -> Result<Uuid, TuskError>
```

**Parameters**:
- `config`: Connection settings (host, port, database, username, SSL, etc.)
- `password`: Database password (not stored, used only for pool creation)

**Returns**:
- `Ok(Uuid)`: Connection ID for the new connection
- `Err(TuskError::Connection)`: Connection failed
- `Err(TuskError::Authentication)`: Invalid credentials
- `Err(TuskError::Ssl)`: SSL/TLS error

**Side Effects**:
- Creates ConnectionPool with validated connection
- Adds ConnectionEntry to `self.connections`
- Updates connection status to Connected
- Stores password in CredentialService if keychain available
- Logs at DEBUG level (without password)

**FR Coverage**: FR-005, FR-006, FR-007, FR-008, FR-009

---

### disconnect

Closes a database connection and removes it from the pool.

```rust
pub async fn disconnect(&self, connection_id: Uuid) -> Result<(), TuskError>
```

**Parameters**:
- `connection_id`: ID of connection to close

**Returns**:
- `Ok(())`: Connection closed successfully
- `Err(TuskError::Internal)`: Connection not found

**Side Effects**:
- Cancels all active queries on this connection
- Closes ConnectionPool
- Removes ConnectionEntry from `self.connections`
- Removes SchemaCache for this connection
- Updates status to Disconnected

**FR Coverage**: FR-008

---

### get_connection_status

Returns the current status of a connection.

```rust
pub fn get_connection_status(&self, connection_id: Uuid) -> ConnectionStatus
```

**Parameters**:
- `connection_id`: ID of connection to check

**Returns**:
- `ConnectionStatus::Connected`: Active connection
- `ConnectionStatus::Disconnected`: No connection
- `ConnectionStatus::Connecting`: Connection in progress
- `ConnectionStatus::Error { message, recoverable }`: Connection error

**Thread Safety**: Uses RwLock read lock, non-blocking

**FR Coverage**: FR-006

---

### test_connection

Tests connection without establishing a persistent session.

```rust
pub async fn test_connection(
    &self,
    config: &ConnectionConfig,
    password: &str,
) -> Result<(), TuskError>
```

**Parameters**:
- `config`: Connection settings to test
- `password`: Password for authentication

**Returns**:
- `Ok(())`: Connection successful
- `Err(TuskError)`: Connection failed with reason

**Side Effects**: None (connection closed after test)

---

## Query Execution API

### execute_query

Executes a SQL query and returns a handle for tracking/cancellation.

```rust
pub async fn execute_query(
    &self,
    connection_id: Uuid,
    sql: &str,
) -> Result<QueryHandle, TuskError>
```

**Parameters**:
- `connection_id`: Connection to execute on
- `sql`: SQL query text

**Returns**:
- `Ok(QueryHandle)`: Handle for tracking query
- `Err(TuskError::Internal)`: Connection not found
- `Err(TuskError::PoolTimeout)`: No connections available

**Side Effects**:
- Registers QueryHandle in `self.active_queries`
- Logs query execution at DEBUG level
- Does NOT log SQL parameters

**FR Coverage**: FR-010, FR-013, FR-015

---

### execute_query_streaming

Executes a query with streaming results via channel.

```rust
pub async fn execute_query_streaming(
    &self,
    connection_id: Uuid,
    sql: &str,
    tx: mpsc::Sender<QueryEvent>,
) -> Result<QueryHandle, TuskError>
```

**Parameters**:
- `connection_id`: Connection to execute on
- `sql`: SQL query text
- `tx`: Channel sender for QueryEvent stream

**Returns**:
- `Ok(QueryHandle)`: Handle for tracking/cancellation
- `Err(TuskError)`: Query initialization failed

**Stream Events** (sent to `tx`):
1. `QueryEvent::Columns(columns)` - First, for grid setup
2. `QueryEvent::Rows(batch, total)` - Batches of 1000 rows
3. `QueryEvent::Complete { ... }` or `QueryEvent::Error { ... }` - Final

**FR Coverage**: FR-010, FR-011, FR-012, FR-013, FR-014, FR-015

---

### cancel_query

Cancels a running query.

```rust
pub async fn cancel_query(&self, query_id: Uuid) -> Result<(), TuskError>
```

**Parameters**:
- `query_id`: ID of query to cancel

**Returns**:
- `Ok(())`: Cancellation requested
- `Err(TuskError::Internal)`: Query not found

**Side Effects**:
- Signals CancellationToken
- Sends PostgreSQL cancel to server
- Query will emit `QueryEvent::Error(QueryCancelled)`
- Removes from `self.active_queries`

**FR Coverage**: FR-013

---

## Schema Management API

### get_schema

Returns cached schema or loads if expired/missing.

```rust
pub async fn get_schema(
    &self,
    connection_id: Uuid,
) -> Result<DatabaseSchema, TuskError>
```

**Parameters**:
- `connection_id`: Connection to get schema for

**Returns**:
- `Ok(DatabaseSchema)`: Schema data
- `Err(TuskError::Internal)`: Connection not found
- `Err(TuskError::Query)`: Schema query failed

**Cache Behavior**:
- Returns cached if valid (within TTL)
- Loads from database if cache expired or missing
- Uses separate pool connection (doesn't block queries)

**FR Coverage**: FR-016, FR-017, FR-018

---

### refresh_schema

Forces schema reload, ignoring cache TTL.

```rust
pub async fn refresh_schema(
    &self,
    connection_id: Uuid,
) -> Result<DatabaseSchema, TuskError>
```

**Parameters**:
- `connection_id`: Connection to refresh schema for

**Returns**:
- `Ok(DatabaseSchema)`: Fresh schema data
- `Err(TuskError)`: Schema load failed

**Side Effects**:
- Invalidates existing cache
- Loads fresh schema from database
- Updates cache with new TTL

**FR Coverage**: FR-017

---

## Credential Management API

### store_password

Stores password in OS keychain.

```rust
pub fn store_password(
    &self,
    connection_id: Uuid,
    password: &str,
) -> Result<(), TuskError>
```

**Parameters**:
- `connection_id`: Connection ID (used as keychain key)
- `password`: Password to store

**Returns**:
- `Ok(())`: Password stored
- `Err(TuskError::Keyring)`: Keychain access failed

**Fallback**: If keychain unavailable, stores in session memory only (FR-029)

**FR Coverage**: FR-009, FR-028, FR-029

---

### get_password

Retrieves password from OS keychain.

```rust
pub fn get_password(&self, connection_id: Uuid) -> Result<Option<String>, TuskError>
```

**Parameters**:
- `connection_id`: Connection ID to look up

**Returns**:
- `Ok(Some(password))`: Password found
- `Ok(None)`: No password stored
- `Err(TuskError::Keyring)`: Keychain access failed

**FR Coverage**: FR-009

---

## Error Conversion

All errors returned implement conversion to ErrorInfo:

```rust
impl TuskError {
    pub fn to_error_info(&self) -> ErrorInfo {
        // Converts to user-displayable format
    }
}
```

**FR Coverage**: FR-019, FR-020, FR-021

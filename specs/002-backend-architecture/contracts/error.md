# Error Handling Contract

**Module**: `tusk_core::error`
**Requirements**: FR-001 through FR-004

---

## TuskError

Main error type for Tusk application (FR-001).

### Variants

| Variant | Fields | Description | Example Hint |
|---------|--------|-------------|--------------|
| `Connection` | `message: String`, `source: Option<Error>` | Database connection failed | "Check that the database server is running" |
| `Authentication` | `message: String`, `hint: Option<String>` | Authentication failed | "Check username and password" |
| `Ssl` | `message: String`, `source: Option<Error>` | SSL/TLS error | "Verify SSL certificate configuration" |
| `Ssh` | `message: String`, `source: Option<Error>` | SSH tunnel error | "Check SSH key permissions" |
| `Query` | `message, detail, hint, position, code` | Query execution error | "Check SQL syntax near position X" |
| `Storage` | `message: String`, `source: Option<Error>` | Local SQLite error | "Data directory may be corrupted" |
| `Keyring` | `message: String`, `hint: Option<String>` | OS keychain error | "Grant Tusk access in system preferences" |
| `PoolTimeout` | `message: String`, `waiting: usize` | Pool exhausted (FR-013a) | "Try closing unused connections" |
| `Internal` | `message: String`, `source: Option<Error>` | Unexpected error | "Please report this issue" |

### Query Variant Fields (FR-002)

| Field | Type | Description |
|-------|------|-------------|
| `message` | `String` | PostgreSQL error message |
| `detail` | `Option<String>` | Additional detail from PG |
| `hint` | `Option<String>` | PostgreSQL hint |
| `position` | `Option<usize>` | Position in query (1-indexed) |
| `code` | `Option<String>` | PostgreSQL error code (e.g., "42P01") |

### Constructors

```rust
// Simple constructors
TuskError::connection(message: impl Into<String>) -> Self
TuskError::connection_with_source(message, source: impl Error) -> Self
TuskError::authentication(message: impl Into<String>) -> Self
TuskError::query(message, detail, hint, position, code) -> Self
TuskError::storage(message: impl Into<String>) -> Self
TuskError::keyring(message, hint: Option<String>) -> Self
TuskError::pool_timeout(waiting: usize) -> Self
TuskError::internal(message: impl Into<String>) -> Self
```

### Methods

```rust
fn category(&self) -> &'static str      // "Connection", "Query", etc.
fn hint(&self) -> Option<&str>          // Get actionable hint
fn pg_code(&self) -> Option<&str>       // PostgreSQL error code
fn position(&self) -> Option<usize>     // Position in query
fn to_error_info(&self) -> ErrorInfo    // Convert for UI display (FR-003)
```

---

## ErrorInfo

User-displayable error information (FR-003).

| Field | Type | Description |
|-------|------|-------------|
| `error_type` | `String` | Category name (e.g., "Connection Error") |
| `message` | `String` | User-friendly message |
| `hint` | `Option<String>` | Actionable suggestion |
| `technical_detail` | `Option<String>` | For "Show Details" expansion |

---

## Error Conversions (FR-004)

### From tokio_postgres::Error

Maps PostgreSQL errors to appropriate TuskError variants:

| PG Code Pattern | TuskError Variant | Hint |
|-----------------|-------------------|------|
| `28P01` | `Authentication` | "Invalid password - check your credentials" |
| `28000` | `Authentication` | "Authentication failed - check username and permissions" |
| `3D000` | `Query` | "Database does not exist - verify database name" |
| `08xxx` | `Connection` | "Connection exception - check network connectivity" |
| `42xxx` | `Query` | Include position and PG hint |
| Other | `Connection` | Generic connection error |

### From rusqlite::Error

```rust
impl From<rusqlite::Error> for TuskError -> TuskError::Storage
```

### From std::io::Error

```rust
impl From<std::io::Error> for TuskError -> TuskError::Storage
```

### From serde_json::Error

```rust
impl From<serde_json::Error> for TuskError -> TuskError::Storage
```

# Query Execution Contract

**Module**: `tusk_core::services::query`
**Requirements**: FR-014 through FR-016

---

## QueryHandle

Handle for tracking and cancelling a running query (FR-014, FR-015).

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique query identifier (FR-014) |
| `connection_id` | `Uuid` | Associated connection |
| `sql` | `String` | The SQL being executed |
| `cancel_token` | `CancellationToken` | For cancellation (FR-015) |
| `started_at` | `DateTime<Utc>` | Execution start time |

### Constructor

```rust
fn new(connection_id: Uuid, sql: impl Into<String>) -> Self
```

### Methods

```rust
fn id(&self) -> Uuid                        // Unique query ID
fn connection_id(&self) -> Uuid             // Associated connection
fn sql(&self) -> &str                       // SQL being executed
fn started_at(&self) -> DateTime<Utc>       // Execution start time
fn elapsed(&self) -> chrono::Duration       // Elapsed time

// Request query cancellation (FR-015)
// Cancellation propagates within 50ms (SC-004)
fn cancel(&self)

fn is_cancelled(&self) -> bool              // Check if cancelled
```

---

## QueryType

Type of query being executed.

| Value | Description |
|-------|-------------|
| `Select` | SELECT query returning rows |
| `Insert` | INSERT operation |
| `Update` | UPDATE operation |
| `Delete` | DELETE operation |
| `Other` | DDL, COPY, or other |

---

## ColumnInfo

Column metadata from query results.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Column name |
| `type_oid` | `u32` | PostgreSQL type OID |
| `type_name` | `String` | Human-readable type |

---

## QueryResult

Results from query execution.

| Field | Type | Description |
|-------|------|-------------|
| `query_id` | `Uuid` | The query handle ID |
| `columns` | `Vec<ColumnInfo>` | Column metadata |
| `rows` | `Vec<Row>` | Result rows |
| `rows_affected` | `Option<u64>` | For INSERT/UPDATE/DELETE |
| `execution_time_ms` | `u64` | Time to execute |
| `query_type` | `QueryType` | Type of query executed |

---

## QueryService

Query execution service.

### Methods

```rust
// Execute a query with cancellation support
// Returns unique query identifier for tracking (FR-014)
// Supports cancellation via cancellation token (FR-015)
// If cancelled after completion, returns results normally (clarification)
async fn execute(
    pool: &ConnectionPool,
    sql: &str,
    handle: &QueryHandle,
) -> Result<QueryResult, TuskError>

// Execute a query with parameters
async fn execute_with_params(
    pool: &ConnectionPool,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    handle: &QueryHandle,
) -> Result<QueryResult, TuskError>

// Determine query type from SQL
fn detect_query_type(sql: &str) -> QueryType
```

### Cancellation Behavior

1. Query cancellation is best-effort
2. If query completes before cancellation reaches database, results are returned normally
3. Cancelled queries return `TuskError::Query` with cancellation message
4. Cancellation propagates within 50ms (SC-004)

---

## QueryHistoryEntry

Record of a previously executed query (for history).

| Field | Type | Description |
|-------|------|-------------|
| `id` | `i64` | Auto-increment ID |
| `connection_id` | `Uuid` | Associated connection |
| `sql` | `String` | The executed SQL |
| `execution_time_ms` | `Option<u64>` | Time to execute |
| `row_count` | `Option<u64>` | Rows returned or affected |
| `error_message` | `Option<String>` | Error message if failed |
| `executed_at` | `DateTime<Utc>` | Execution timestamp |

### Constructors

```rust
// Create from a successful query result
fn from_result(connection_id: Uuid, sql: &str, result: &QueryResult) -> Self

// Create from a failed query
fn from_error(connection_id: Uuid, sql: &str, error: &TuskError) -> Self
```

---

## Query State Transitions

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

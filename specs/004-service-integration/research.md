# Research: Service Integration Layer

**Feature**: 004-service-integration
**Date**: 2026-01-21
**Status**: Complete

## Research Tasks

### 1. GPUI Global State Access Patterns

**Question**: How should UI components access TuskState and services?

**Decision**: Use `cx.global::<TuskState>()` with direct method calls

**Rationale**:
- GPUI's Global trait provides type-safe, compile-time checked access to application-wide state
- No IPC overhead since everything runs in the same process
- Zed uses this pattern extensively (NotificationStore, SettingsStore, etc.)
- TuskState already implements Global trait

**Alternatives Considered**:
- Entity references passed through component hierarchy: Rejected - verbose prop drilling
- Context providers (React-style): Rejected - GPUI doesn't use this pattern
- Message passing: Rejected - unnecessary complexity for single-process app

**Implementation Pattern**:
```rust
// Access in render or methods
fn execute_query(&mut self, cx: &mut Context<Self>) {
    let state = cx.global::<TuskState>();
    let runtime = state.runtime().handle().clone();
    // ...
}
```

---

### 2. Async Task Spawning from UI

**Question**: How should UI components spawn async database operations?

**Decision**: Use `cx.spawn()` wrapping `runtime.spawn()` for database work

**Rationale**:
- GPUI's `cx.spawn()` provides weak entity reference for safe UI updates
- Tokio runtime (owned by TuskState) handles actual async I/O
- Pattern already established in app.rs for schema loading
- Automatic cancellation when entity is dropped

**Alternatives Considered**:
- Direct tokio::spawn: Rejected - no UI update capability
- GPUI background executor only: Rejected - deadpool-postgres requires Tokio
- Channel-based decoupling: Rejected - unnecessary indirection

**Implementation Pattern**:
```rust
cx.spawn(async move |this, cx| {
    let runtime = cx.global::<TuskState>().runtime().handle().clone();

    let result = runtime.spawn(async move {
        // Database operations here
    }).await;

    this.update(&cx, |this, cx| {
        // Update UI with result
        cx.notify();
    })?;

    Ok(())
}).detach();
```

---

### 3. Streaming Query Results

**Question**: How should large result sets be streamed to the UI?

**Decision**: Use tokio mpsc channels with QueryEvent enum

**Rationale**:
- mpsc channels provide backpressure and memory-bounded streaming
- QueryEvent enum (Columns, Rows, Complete, Error) matches PostgreSQL wire protocol flow
- 1000-row batch size balances responsiveness with overhead
- tokio_postgres::RowStream already supports async iteration

**Alternatives Considered**:
- Single QueryResult with all rows: Rejected - memory exhaustion for large datasets
- unbounded channels: Rejected - no backpressure, memory risk
- Direct row iteration: Rejected - can't update UI incrementally

**QueryEvent Design**:
```rust
pub enum QueryEvent {
    Columns(Vec<ColumnInfo>),           // Sent first for grid setup
    Rows(Vec<Row>, usize),               // Batch + running total
    Progress { rows_so_far: usize },     // Optional progress updates
    Complete {
        total_rows: usize,
        execution_time_ms: u64,
        rows_affected: Option<u64>,
    },
    Error(TuskError),
}
```

---

### 4. Query Cancellation

**Question**: How should running queries be cancelled?

**Decision**: Use tokio_util::CancellationToken with tokio::select!

**Rationale**:
- CancellationToken is already used in QueryHandle
- PostgreSQL supports query cancellation via cancel_token on connection
- tokio::select! provides non-blocking cancellation checks
- Task replacement in GPUI provides implicit cancellation

**Alternatives Considered**:
- Drop-based cancellation only: Rejected - doesn't send PostgreSQL cancel
- Atomic flags: Rejected - more complex than CancellationToken
- Message-based cancellation: Rejected - unnecessary indirection

**Implementation Pattern**:
```rust
tokio::select! {
    row = rows.try_next() => {
        match row? {
            Some(row) => batch.push(row),
            None => break,
        }
    }
    _ = handle.cancelled() => {
        // Send PostgreSQL cancel
        conn.cancel_token().cancel_query(NoTls).await?;
        return Err(TuskError::QueryCancelled { query_id: handle.id() });
    }
}
```

---

### 5. Error Display Patterns

**Question**: How should errors be displayed to users?

**Decision**: Toast notifications for recoverable errors, error panels for critical errors

**Rationale**:
- Zed uses StatusToast for transient feedback (auto-dismiss)
- Error panels provide persistent display with full detail
- ErrorInfo struct already contains title, message, detail, hint
- Recoverable flag determines display type

**Alternatives Considered**:
- Modal dialogs for all errors: Rejected - interrupts workflow
- Console-only logging: Rejected - users need feedback
- In-place error indicators only: Rejected - insufficient for connection errors

**Display Rules**:
| Error Type | Display Method | Auto-Dismiss |
|------------|----------------|--------------|
| Query syntax error | Error panel below results | No |
| Connection error | Error panel + status bar | No |
| Query cancelled | Toast notification | Yes (3s) |
| Permission denied | Error panel with hint | No |
| Network timeout | Toast + retry option | Yes (10s) |

---

### 6. Connection Status Tracking

**Question**: How should connection status be tracked and displayed?

**Decision**: ConnectionStatus enum in TuskState with UI subscriptions

**Rationale**:
- Status changes need to update multiple UI components (status bar, schema browser, editor)
- RwLock<HashMap<Uuid, ConnectionStatus>> for thread-safe access
- GPUI subscriptions allow reactive updates

**ConnectionStatus States**:
```rust
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected { pool: Arc<ConnectionPool> },
    Error { message: String, recoverable: bool },
}
```

---

### 7. Schema Caching with TTL

**Question**: How should schema data be cached and invalidated?

**Decision**: In-memory cache with TTL and manual refresh

**Rationale**:
- Schema changes are infrequent during typical usage
- TTL prevents stale data (default: 5 minutes)
- Manual refresh button for immediate update
- Separate cache per connection

**Alternatives Considered**:
- No caching: Rejected - schema queries are expensive
- Persistent cache (SQLite): Rejected - complexity, staleness risk
- LISTEN/NOTIFY for changes: Future enhancement, not MVP

**Cache Structure**:
```rust
pub struct SchemaCache {
    schema: DatabaseSchema,
    loaded_at: Instant,
    ttl: Duration,
}

impl SchemaCache {
    pub fn is_expired(&self) -> bool {
        self.loaded_at.elapsed() > self.ttl
    }
}
```

---

### 8. Credential Retrieval Flow

**Question**: How should passwords be retrieved when connecting?

**Decision**: Lazy retrieval from keychain, session fallback, never store in memory long-term

**Rationale**:
- FR-028: Passwords never in files
- FR-029: Session fallback when keychain unavailable
- Passwords retrieved just-in-time for connection
- Cleared from memory after pool creation

**Flow**:
1. User selects saved connection
2. Check CredentialService for stored password
3. If found, use it; if not, prompt user
4. Create connection pool with password
5. Password not stored in ConnectionConfig or TuskState

---

### 9. Observability and Logging

**Question**: What logging strategy should be used?

**Decision**: tracing crate with structured logging, DEBUG for service calls, WARN/ERROR for errors

**Rationale**:
- FR-024: Service calls at DEBUG with timing
- FR-025: Errors at WARN/ERROR with context
- FR-026: Never log passwords or credentials
- tracing already configured in logging.rs

**Logging Levels**:
```rust
// Service calls
tracing::debug!(
    connection_id = %id,
    query_id = %handle.id(),
    elapsed_ms = elapsed,
    "Query executed"
);

// Errors
tracing::error!(
    error_type = "connection",
    %message,
    hint = hint.as_deref(),
    "Connection failed"
);

// NEVER log
// - Passwords
// - Connection strings with credentials
// - SQL parameters that might contain sensitive data
```

---

### 10. Concurrent Operations

**Question**: How should concurrent queries/schema operations be handled?

**Decision**: Connection pool with minimum 2-3 connections per server

**Rationale**:
- FR-005: Pool supports concurrent operations
- Schema browser, query execution, multi-tab queries operate independently
- deadpool-postgres handles pool management
- Real DB clients (pgAdmin, DBeaver, DataGrip) use this pattern

**Pool Configuration**:
```rust
PoolConfig {
    max_size: 10,                    // Upper limit
    min_idle: 2,                     // Always available
    connection_timeout: Duration::from_secs(30),
    idle_timeout: Some(Duration::from_secs(600)),
}
```

---

## Summary

All research questions resolved. Key decisions:

1. **Global state access**: `cx.global::<TuskState>()`
2. **Async spawning**: `cx.spawn()` + `runtime.spawn()`
3. **Streaming**: mpsc channels with QueryEvent enum
4. **Cancellation**: CancellationToken + tokio::select!
5. **Error display**: Toasts for recoverable, panels for critical
6. **Connection status**: Enum in TuskState with UI subscriptions
7. **Schema caching**: In-memory with TTL (5 min default)
8. **Credentials**: Lazy keychain retrieval, session fallback
9. **Logging**: tracing at DEBUG/WARN/ERROR, no sensitive data
10. **Concurrency**: Connection pool with min 2-3 connections

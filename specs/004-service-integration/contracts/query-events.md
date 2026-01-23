# Contract: Query Event Stream

**Feature**: 004-service-integration
**Type**: Internal Rust API (channel-based streaming)
**Location**: `crates/tusk_core/src/models/query.rs`

## Overview

QueryEvent is an enum representing events during query execution. Events are sent through a tokio mpsc channel to enable streaming result delivery to UI components.

## Channel Setup

```rust
use tokio::sync::mpsc;

// Create bounded channel (backpressure at 100 pending events)
let (tx, rx) = mpsc::channel::<QueryEvent>(100);

// Start streaming query
let handle = state.execute_query_streaming(connection_id, sql, tx).await?;

// Receive events
while let Some(event) = rx.recv().await {
    match event {
        QueryEvent::Columns(columns) => { /* Setup grid */ },
        QueryEvent::Rows(rows, total) => { /* Add to grid */ },
        QueryEvent::Progress { rows_so_far } => { /* Update status */ },
        QueryEvent::Complete { total_rows, execution_time_ms, rows_affected } => break,
        QueryEvent::Error(err) => { /* Show error */ break; },
    }
}
```

## Event Types

### QueryEvent::Columns

First event sent, contains column metadata for result grid setup.

```rust
QueryEvent::Columns(columns: Vec<ColumnInfo>)
```

**ColumnInfo Structure**:
```rust
pub struct ColumnInfo {
    pub name: String,          // Column name
    pub type_oid: u32,         // PostgreSQL type OID
    pub type_name: String,     // Human-readable type name
}
```

**Timing**: Always sent first, before any Rows events
**Cardinality**: Exactly once per query execution

**FR Coverage**: FR-014

---

### QueryEvent::Rows

Batch of result rows with running total count.

```rust
QueryEvent::Rows(rows: Vec<Row>, total_so_far: usize)
```

**Parameters**:
- `rows`: Batch of tokio_postgres::Row (default batch size: 1000)
- `total_so_far`: Cumulative count including this batch

**Batch Size**: Configurable, default 1000 rows per batch
**Timing**: Sent as rows are retrieved from PostgreSQL
**Cardinality**: 0 to N times per query (0 for empty results)

**FR Coverage**: FR-011, FR-012

---

### QueryEvent::Progress

Optional progress update for large queries.

```rust
QueryEvent::Progress { rows_so_far: usize }
```

**Parameters**:
- `rows_so_far`: Number of rows received so far

**Timing**: Sent periodically for queries > 10,000 rows
**Cardinality**: 0 to N times (optional, for UI progress indicator)

---

### QueryEvent::Complete

Query finished successfully.

```rust
QueryEvent::Complete {
    total_rows: usize,
    execution_time_ms: u64,
    rows_affected: Option<u64>,
}
```

**Parameters**:
- `total_rows`: Final row count
- `execution_time_ms`: Query execution time
- `rows_affected`: For INSERT/UPDATE/DELETE (None for SELECT)

**Timing**: Final event for successful queries
**Cardinality**: Exactly once (mutually exclusive with Error)

**FR Coverage**: FR-015

---

### QueryEvent::Error

Query failed with error details.

```rust
QueryEvent::Error(error: TuskError)
```

**Error Types**:
- `TuskError::Query { ... }`: SQL error with position, hint
- `TuskError::QueryCancelled { query_id }`: User cancelled
- `TuskError::PoolTimeout { ... }`: Connection unavailable
- `TuskError::Connection { ... }`: Connection lost mid-query

**Timing**: Final event for failed queries
**Cardinality**: Exactly once (mutually exclusive with Complete)

**FR Coverage**: FR-019, FR-020, FR-021

---

## Event Ordering Guarantees

1. **Columns first**: `Columns` is always the first event (if query has results)
2. **Rows in order**: `Rows` batches maintain database row order
3. **Terminal event**: Either `Complete` or `Error`, never both
4. **No events after terminal**: Channel closes after terminal event

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌──────────┐
│ Columns │───▶│  Rows   │───▶│  Rows   │───▶│ Complete │
└─────────┘    └─────────┘    └─────────┘    └──────────┘
     │              │              │               │
     │              ▼              ▼               ▼
     │         (repeat)       (repeat)         (end)
     │
     └────────────────────────────────────────────────────┐
                                                          ▼
                                                    ┌─────────┐
                                                    │  Error  │
                                                    └─────────┘
```

---

## Backpressure

Channel is bounded (capacity: 100 events). If receiver is slow:
- Sender blocks until receiver catches up
- Prevents memory exhaustion for large result sets
- UI can batch updates while catching up

---

## Cancellation Handling

When query is cancelled via `cancel_query()`:

1. CancellationToken triggers
2. PostgreSQL cancel sent to server
3. `QueryEvent::Error(TuskError::QueryCancelled { query_id })` sent
4. Channel closes
5. Already-received rows remain visible in UI (per spec clarification)

---

## UI Integration Pattern

```rust
impl ResultsPanel {
    fn start_streaming(&mut self, rx: mpsc::Receiver<QueryEvent>, cx: &mut Context<Self>) {
        self.status = ResultsStatus::Loading;

        self._stream_subscription = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = rx.recv().await {
                this.update(&cx, |panel, cx| {
                    panel.handle_event(event, cx);
                })?;
            }
            Ok(())
        }));
    }

    fn handle_event(&mut self, event: QueryEvent, cx: &mut Context<Self>) {
        match event {
            QueryEvent::Columns(columns) => {
                self.columns = columns;
                self.status = ResultsStatus::Streaming;
            }
            QueryEvent::Rows(rows, total) => {
                self.rows.extend(rows);
                self.total_rows = total;
                cx.notify();
            }
            QueryEvent::Complete { total_rows, execution_time_ms, .. } => {
                self.total_rows = total_rows;
                self.execution_time_ms = Some(execution_time_ms);
                self.status = ResultsStatus::Complete;
                cx.notify();
            }
            QueryEvent::Error(err) => {
                self.error = Some(err.to_error_info());
                self.status = ResultsStatus::Error;
                cx.notify();
            }
            _ => {}
        }
    }
}
```

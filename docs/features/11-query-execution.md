# Feature 11: Query Execution Engine

## Overview

The query execution engine is the core service that executes SQL queries against connected Postgres databases. Built entirely in Rust with GPUI integration, it handles single and multiple statement execution, streaming results for large datasets, query cancellation, timeout enforcement, and detailed error handling with position information.

## Goals

- Execute queries asynchronously with streaming support for large results
- Support query cancellation at any point during execution
- Enforce statement timeouts to prevent runaway queries
- Parse and split multiple statements correctly (respecting strings and dollar-quoting)
- Provide detailed error information including position in the query
- Track query execution for history
- Integrate with GPUI state system for reactive UI updates

## Dependencies

- Feature 07: Connection Management (active connection pools)
- Feature 05: Local Storage (query history persistence)
- Feature 10: Schema Introspection (for autocomplete context)

## Technical Specification

### 11.1 Query Data Models

```rust
// src/models/query.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::Instant;

/// Status of a query execution
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    /// Query executed successfully
    Success,
    /// Query failed with error
    Error,
    /// Query was cancelled by user
    Cancelled,
    /// Query is currently running
    Running,
}

/// Complete result of a query execution
#[derive(Clone, Debug)]
pub struct QueryResult {
    /// Unique identifier for this query execution
    pub query_id: Uuid,
    /// Final status
    pub status: QueryStatus,
    /// SQL command type (SELECT, INSERT, etc.)
    pub command: String,
    /// Original SQL that was executed
    pub sql: String,

    // For SELECT queries
    /// Column metadata
    pub columns: Option<Vec<ColumnMeta>>,
    /// Result rows (may be partial if streaming)
    pub rows: Option<Vec<Vec<Value>>>,
    /// Total rows returned
    pub total_rows: Option<u64>,
    /// Whether results were truncated due to row limit
    pub truncated: Option<bool>,

    // For DML queries
    /// Number of rows affected by INSERT/UPDATE/DELETE
    pub rows_affected: Option<u64>,

    // For EXPLAIN
    /// Query execution plan
    pub plan: Option<QueryPlan>,

    // Timing
    /// Total execution time in milliseconds
    pub elapsed_ms: u64,

    // Errors
    /// Error details if query failed
    pub error: Option<QueryError>,
}

impl QueryResult {
    pub fn success_select(
        query_id: Uuid,
        sql: String,
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
        elapsed_ms: u64,
    ) -> Self {
        let total_rows = rows.len() as u64;
        Self {
            query_id,
            status: QueryStatus::Success,
            command: "SELECT".to_string(),
            sql,
            columns: Some(columns),
            rows: Some(rows),
            total_rows: Some(total_rows),
            truncated: Some(false),
            rows_affected: None,
            plan: None,
            elapsed_ms,
            error: None,
        }
    }

    pub fn success_dml(
        query_id: Uuid,
        command: String,
        sql: String,
        rows_affected: u64,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            query_id,
            status: QueryStatus::Success,
            command,
            sql,
            columns: None,
            rows: None,
            total_rows: None,
            truncated: None,
            rows_affected: Some(rows_affected),
            plan: None,
            elapsed_ms,
            error: None,
        }
    }

    pub fn error(query_id: Uuid, sql: String, error: QueryError, elapsed_ms: u64) -> Self {
        Self {
            query_id,
            status: QueryStatus::Error,
            command: String::new(),
            sql,
            columns: None,
            rows: None,
            total_rows: None,
            truncated: None,
            rows_affected: None,
            plan: None,
            elapsed_ms,
            error: Some(error),
        }
    }

    pub fn cancelled(query_id: Uuid, sql: String, elapsed_ms: u64) -> Self {
        Self {
            query_id,
            status: QueryStatus::Cancelled,
            command: String::new(),
            sql,
            columns: None,
            rows: None,
            total_rows: None,
            truncated: None,
            rows_affected: None,
            plan: None,
            elapsed_ms,
            error: None,
        }
    }
}

/// Metadata about a result column
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnMeta {
    /// Column name
    pub name: String,
    /// PostgreSQL type OID
    pub type_oid: u32,
    /// Human-readable type name
    pub type_name: String,
    /// Type modifier (for precision/scale)
    pub type_modifier: i32,
    /// Source table OID if from a table
    pub table_oid: Option<u32>,
    /// Column position in result
    pub column_ordinal: Option<i32>,
    /// Is this column nullable
    pub nullable: bool,
}

/// A value from a query result cell
#[derive(Clone, Debug)]
pub enum Value {
    /// SQL NULL
    Null,
    /// Boolean
    Bool(bool),
    /// Small integer (int2)
    SmallInt(i16),
    /// Integer (int4)
    Int(i32),
    /// Big integer (int8)
    BigInt(i64),
    /// Single precision float
    Float(f32),
    /// Double precision float
    Double(f64),
    /// Numeric/decimal as string (preserves precision)
    Numeric(String),
    /// Text string
    Text(String),
    /// Binary data
    Bytea(Vec<u8>),
    /// JSON/JSONB value
    Json(serde_json::Value),
    /// UUID
    Uuid(Uuid),
    /// Date (ISO format string)
    Date(String),
    /// Time (ISO format string)
    Time(String),
    /// Timestamp (ISO format string)
    Timestamp(String),
    /// Timestamp with timezone (ISO format string)
    TimestampTz(String),
    /// Interval (ISO 8601 duration string)
    Interval(String),
    /// Array of values
    Array(Vec<Value>),
    /// Point (x, y)
    Point { x: f64, y: f64 },
    /// Range
    Range { lower: Option<Box<Value>>, upper: Option<Box<Value>>, lower_inclusive: bool, upper_inclusive: bool },
    /// Composite/record type
    Composite(Vec<(String, Value)>),
    /// Unknown type rendered as string
    Unknown(String),
}

impl Value {
    /// Convert to display string for grid
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::SmallInt(n) => n.to_string(),
            Value::Int(n) => n.to_string(),
            Value::BigInt(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Double(f) => f.to_string(),
            Value::Numeric(s) => s.clone(),
            Value::Text(s) => s.clone(),
            Value::Bytea(bytes) => format!("\\x{}", hex::encode(bytes)),
            Value::Json(j) => j.to_string(),
            Value::Uuid(u) => u.to_string(),
            Value::Date(s) | Value::Time(s) | Value::Timestamp(s) | Value::TimestampTz(s) => s.clone(),
            Value::Interval(s) => s.clone(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_display_string()).collect();
                format!("{{{}}}", items.join(","))
            }
            Value::Point { x, y } => format!("({},{})", x, y),
            Value::Range { lower, upper, lower_inclusive, upper_inclusive } => {
                let l = if *lower_inclusive { "[" } else { "(" };
                let r = if *upper_inclusive { "]" } else { ")" };
                let low = lower.as_ref().map(|v| v.to_display_string()).unwrap_or_default();
                let high = upper.as_ref().map(|v| v.to_display_string()).unwrap_or_default();
                format!("{}{},{}{}", l, low, high, r)
            }
            Value::Composite(fields) => {
                let items: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{}:{}", k, v.to_display_string()))
                    .collect();
                format!("({})", items.join(","))
            }
            Value::Unknown(s) => s.clone(),
        }
    }

    /// Check if this value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get the logical type for display
    pub fn type_hint(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::SmallInt(_) | Value::Int(_) | Value::BigInt(_) => "integer",
            Value::Float(_) | Value::Double(_) => "float",
            Value::Numeric(_) => "numeric",
            Value::Text(_) => "text",
            Value::Bytea(_) => "bytea",
            Value::Json(_) => "json",
            Value::Uuid(_) => "uuid",
            Value::Date(_) => "date",
            Value::Time(_) => "time",
            Value::Timestamp(_) | Value::TimestampTz(_) => "timestamp",
            Value::Interval(_) => "interval",
            Value::Array(_) => "array",
            Value::Point { .. } => "point",
            Value::Range { .. } => "range",
            Value::Composite(_) => "composite",
            Value::Unknown(_) => "unknown",
        }
    }
}

/// Detailed error from query execution
#[derive(Clone, Debug)]
pub struct QueryError {
    /// Primary error message
    pub message: String,
    /// Additional detail (from Postgres DETAIL)
    pub detail: Option<String>,
    /// Hint for fixing (from Postgres HINT)
    pub hint: Option<String>,
    /// Position in SQL where error occurred (1-indexed)
    pub position: Option<i32>,
    /// Internal position for errors in functions
    pub internal_position: Option<i32>,
    /// Internal query that caused error
    pub internal_query: Option<String>,
    /// PostgreSQL error code (SQLSTATE)
    pub code: String,
    /// Schema name if relevant
    pub schema: Option<String>,
    /// Table name if relevant
    pub table: Option<String>,
    /// Column name if relevant
    pub column: Option<String>,
    /// Constraint name if relevant
    pub constraint: Option<String>,
    /// Error severity
    pub severity: ErrorSeverity,
}

impl QueryError {
    /// Create from tokio-postgres error
    pub fn from_postgres(error: &tokio_postgres::Error, sql: &str) -> Self {
        if let Some(db_error) = error.as_db_error() {
            Self {
                message: db_error.message().to_string(),
                detail: db_error.detail().map(String::from),
                hint: db_error.hint().map(String::from),
                position: db_error.position().map(|p| match p {
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos as i32,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position as i32,
                }),
                internal_position: match db_error.position() {
                    Some(tokio_postgres::error::ErrorPosition::Internal { position, .. }) => Some(*position as i32),
                    _ => None,
                },
                internal_query: match db_error.position() {
                    Some(tokio_postgres::error::ErrorPosition::Internal { query, .. }) => Some(query.to_string()),
                    _ => None,
                },
                code: db_error.code().code().to_string(),
                schema: db_error.schema().map(String::from),
                table: db_error.table().map(String::from),
                column: db_error.column().map(String::from),
                constraint: db_error.constraint().map(String::from),
                severity: ErrorSeverity::from_str(db_error.severity()),
            }
        } else {
            Self {
                message: error.to_string(),
                detail: None,
                hint: None,
                position: None,
                internal_position: None,
                internal_query: None,
                code: String::new(),
                schema: None,
                table: None,
                column: None,
                constraint: None,
                severity: ErrorSeverity::Error,
            }
        }
    }

    /// Get the line and column from position and SQL
    pub fn get_line_column(&self, sql: &str) -> Option<(usize, usize)> {
        let pos = self.position? as usize;
        if pos == 0 || pos > sql.len() {
            return None;
        }

        let prefix = &sql[..pos - 1]; // Position is 1-indexed
        let line = prefix.matches('\n').count() + 1;
        let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = pos - last_newline;

        Some((line, column))
    }
}

/// PostgreSQL error severity
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Fatal,
    Panic,
    Warning,
    Notice,
    Debug,
    Info,
    Log,
}

impl ErrorSeverity {
    fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ERROR" => Self::Error,
            "FATAL" => Self::Fatal,
            "PANIC" => Self::Panic,
            "WARNING" => Self::Warning,
            "NOTICE" => Self::Notice,
            "DEBUG" => Self::Debug,
            "INFO" => Self::Info,
            "LOG" => Self::Log,
            _ => Self::Error,
        }
    }
}

/// A batch of rows for streaming
#[derive(Clone, Debug)]
pub struct RowBatch {
    /// Query this batch belongs to
    pub query_id: Uuid,
    /// Rows in this batch
    pub rows: Vec<Vec<Value>>,
    /// Batch sequence number (0-indexed)
    pub batch_num: u32,
    /// Whether this is the final batch
    pub is_final: bool,
}

/// Notification that query streaming is complete
#[derive(Clone, Debug)]
pub struct QueryComplete {
    /// Query that completed
    pub query_id: Uuid,
    /// Total rows returned
    pub total_rows: u64,
    /// Total execution time
    pub elapsed_ms: u64,
    /// Whether results were truncated
    pub truncated: bool,
}

/// Progress update during query execution
#[derive(Clone, Debug)]
pub struct QueryProgress {
    /// Query being executed
    pub query_id: Uuid,
    /// Rows processed so far
    pub rows_processed: u64,
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Estimated total rows (if available)
    pub estimated_total: Option<u64>,
}

/// Query execution plan
#[derive(Clone, Debug)]
pub struct QueryPlan {
    /// Raw plan output
    pub raw: String,
    /// Plan format
    pub format: PlanFormat,
    /// Parsed plan tree (for JSON format)
    pub root: Option<PlanNode>,
    /// Planning time
    pub planning_time_ms: f64,
    /// Execution time (only with ANALYZE)
    pub execution_time_ms: Option<f64>,
}

/// Plan output format
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanFormat {
    Text,
    Json,
    Xml,
    Yaml,
}

/// A node in the query plan tree
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanNode {
    /// Node type (Seq Scan, Index Scan, etc.)
    pub node_type: String,
    /// Parent relationship
    pub parent_relationship: Option<String>,
    /// Table name if scanning
    pub relation_name: Option<String>,
    /// Alias for relation
    pub alias: Option<String>,
    /// Schema for relation
    pub schema: Option<String>,
    /// Index name if using index
    pub index_name: Option<String>,
    /// Join type for joins
    pub join_type: Option<String>,

    // Cost estimates
    /// Startup cost
    pub startup_cost: f64,
    /// Total cost
    pub total_cost: f64,
    /// Estimated rows
    pub plan_rows: u64,
    /// Estimated row width
    pub plan_width: u32,

    // Actual execution (with ANALYZE)
    /// Actual startup time
    pub actual_startup_time: Option<f64>,
    /// Actual total time
    pub actual_total_time: Option<f64>,
    /// Actual rows
    pub actual_rows: Option<u64>,
    /// Number of loops
    pub actual_loops: Option<u64>,

    // Buffers (with BUFFERS)
    /// Shared blocks hit
    pub shared_hit_blocks: Option<u64>,
    /// Shared blocks read
    pub shared_read_blocks: Option<u64>,
    /// Shared blocks dirtied
    pub shared_dirtied_blocks: Option<u64>,
    /// Shared blocks written
    pub shared_written_blocks: Option<u64>,
    /// Local blocks hit
    pub local_hit_blocks: Option<u64>,
    /// Local blocks read
    pub local_read_blocks: Option<u64>,
    /// Temp blocks read
    pub temp_read_blocks: Option<u64>,
    /// Temp blocks written
    pub temp_written_blocks: Option<u64>,

    // Conditions
    /// Filter condition
    pub filter: Option<String>,
    /// Rows removed by filter
    pub rows_removed_by_filter: Option<u64>,
    /// Index condition
    pub index_cond: Option<String>,
    /// Recheck condition
    pub recheck_cond: Option<String>,
    /// Join filter
    pub join_filter: Option<String>,
    /// Hash condition
    pub hash_cond: Option<String>,
    /// Merge condition
    pub merge_cond: Option<String>,
    /// Sort key
    pub sort_key: Option<Vec<String>>,
    /// Group key
    pub group_key: Option<Vec<String>>,
    /// Output columns
    pub output: Option<Vec<String>>,

    // Workers (for parallel plans)
    /// Workers planned
    pub workers_planned: Option<u32>,
    /// Workers launched
    pub workers_launched: Option<u32>,

    /// Child nodes
    pub children: Vec<PlanNode>,
}

impl PlanNode {
    /// Parse from JSON value
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;

        Some(Self {
            node_type: obj.get("Node Type")?.as_str()?.to_string(),
            parent_relationship: obj.get("Parent Relationship").and_then(|v| v.as_str()).map(String::from),
            relation_name: obj.get("Relation Name").and_then(|v| v.as_str()).map(String::from),
            alias: obj.get("Alias").and_then(|v| v.as_str()).map(String::from),
            schema: obj.get("Schema").and_then(|v| v.as_str()).map(String::from),
            index_name: obj.get("Index Name").and_then(|v| v.as_str()).map(String::from),
            join_type: obj.get("Join Type").and_then(|v| v.as_str()).map(String::from),

            startup_cost: obj.get("Startup Cost").and_then(|v| v.as_f64()).unwrap_or(0.0),
            total_cost: obj.get("Total Cost").and_then(|v| v.as_f64()).unwrap_or(0.0),
            plan_rows: obj.get("Plan Rows").and_then(|v| v.as_u64()).unwrap_or(0),
            plan_width: obj.get("Plan Width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,

            actual_startup_time: obj.get("Actual Startup Time").and_then(|v| v.as_f64()),
            actual_total_time: obj.get("Actual Total Time").and_then(|v| v.as_f64()),
            actual_rows: obj.get("Actual Rows").and_then(|v| v.as_u64()),
            actual_loops: obj.get("Actual Loops").and_then(|v| v.as_u64()),

            shared_hit_blocks: obj.get("Shared Hit Blocks").and_then(|v| v.as_u64()),
            shared_read_blocks: obj.get("Shared Read Blocks").and_then(|v| v.as_u64()),
            shared_dirtied_blocks: obj.get("Shared Dirtied Blocks").and_then(|v| v.as_u64()),
            shared_written_blocks: obj.get("Shared Written Blocks").and_then(|v| v.as_u64()),
            local_hit_blocks: obj.get("Local Hit Blocks").and_then(|v| v.as_u64()),
            local_read_blocks: obj.get("Local Read Blocks").and_then(|v| v.as_u64()),
            temp_read_blocks: obj.get("Temp Read Blocks").and_then(|v| v.as_u64()),
            temp_written_blocks: obj.get("Temp Written Blocks").and_then(|v| v.as_u64()),

            filter: obj.get("Filter").and_then(|v| v.as_str()).map(String::from),
            rows_removed_by_filter: obj.get("Rows Removed by Filter").and_then(|v| v.as_u64()),
            index_cond: obj.get("Index Cond").and_then(|v| v.as_str()).map(String::from),
            recheck_cond: obj.get("Recheck Cond").and_then(|v| v.as_str()).map(String::from),
            join_filter: obj.get("Join Filter").and_then(|v| v.as_str()).map(String::from),
            hash_cond: obj.get("Hash Cond").and_then(|v| v.as_str()).map(String::from),
            merge_cond: obj.get("Merge Cond").and_then(|v| v.as_str()).map(String::from),
            sort_key: obj.get("Sort Key").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                })
            }),
            group_key: obj.get("Group Key").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                })
            }),
            output: obj.get("Output").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                })
            }),

            workers_planned: obj.get("Workers Planned").and_then(|v| v.as_u64()).map(|n| n as u32),
            workers_launched: obj.get("Workers Launched").and_then(|v| v.as_u64()).map(|n| n as u32),

            children: obj.get("Plans").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(PlanNode::from_json).collect())
                .unwrap_or_default(),
        })
    }

    /// Calculate the percentage of total cost
    pub fn cost_percentage(&self, total_root_cost: f64) -> f64 {
        if total_root_cost == 0.0 {
            0.0
        } else {
            (self.total_cost / total_root_cost) * 100.0
        }
    }

    /// Get actual vs estimated row ratio
    pub fn row_estimate_accuracy(&self) -> Option<f64> {
        let actual = self.actual_rows? as f64;
        let estimated = self.plan_rows as f64;
        if estimated == 0.0 {
            None
        } else {
            Some(actual / estimated)
        }
    }
}

/// Options for query execution
#[derive(Clone, Debug)]
pub struct QueryOptions {
    /// Statement timeout in milliseconds (0 = no timeout)
    pub statement_timeout_ms: Option<u32>,
    /// Maximum rows to return (for safety)
    pub row_limit: Option<u64>,
    /// Batch size for streaming
    pub batch_size: usize,
    /// Stop executing on first error in multi-statement
    pub stop_on_error: bool,
    /// Read-only mode (execute in transaction with READ ONLY)
    pub read_only: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            statement_timeout_ms: Some(30000), // 30 seconds default
            row_limit: Some(10000),
            batch_size: 1000,
            stop_on_error: true,
            read_only: false,
        }
    }
}

impl QueryOptions {
    pub fn no_limit() -> Self {
        Self {
            statement_timeout_ms: None,
            row_limit: None,
            batch_size: 5000,
            stop_on_error: true,
            read_only: false,
        }
    }

    pub fn quick_query() -> Self {
        Self {
            statement_timeout_ms: Some(5000),
            row_limit: Some(100),
            batch_size: 100,
            stop_on_error: true,
            read_only: true,
        }
    }
}

/// Options for EXPLAIN
#[derive(Clone, Debug)]
pub struct ExplainOptions {
    /// Run ANALYZE to get actual execution stats
    pub analyze: bool,
    /// Show buffer usage statistics
    pub buffers: bool,
    /// Show additional information
    pub verbose: bool,
    /// Show cost estimates (default true)
    pub costs: bool,
    /// Show timing information (requires analyze)
    pub timing: bool,
    /// Output format
    pub format: PlanFormat,
    /// Show WAL usage (PG 13+)
    pub wal: bool,
}

impl Default for ExplainOptions {
    fn default() -> Self {
        Self {
            analyze: false,
            buffers: false,
            verbose: false,
            costs: true,
            timing: true,
            format: PlanFormat::Json,
            wal: false,
        }
    }
}
```

### 11.2 Query Service

```rust
// src/services/query.rs

use tokio_postgres::{Client, Row, Statement, types::Type};
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use futures::StreamExt;
use parking_lot::RwLock as SyncRwLock;
use tokio::runtime::Handle;

use crate::error::{Error, Result};
use crate::services::connection::ConnectionPool;
use crate::services::storage::StorageService;
use crate::models::query::*;

/// Active query tracking for cancellation
struct ActiveQuery {
    /// Cancel token from tokio-postgres
    cancel_token: tokio_postgres::CancelToken,
    /// When query started
    started_at: Instant,
    /// Original SQL
    sql: String,
    /// Cancel signal sender
    cancel_tx: Option<oneshot::Sender<()>>,
}

/// Callback for streaming results
pub type RowBatchCallback = Arc<dyn Fn(RowBatch) + Send + Sync>;
pub type ProgressCallback = Arc<dyn Fn(QueryProgress) + Send + Sync>;
pub type CompleteCallback = Arc<dyn Fn(QueryComplete) + Send + Sync>;

/// Query execution service
pub struct QueryService {
    /// Active query tracking
    active_queries: SyncRwLock<HashMap<Uuid, ActiveQuery>>,
    /// Storage service for history
    storage: Arc<StorageService>,
    /// Tokio runtime handle
    runtime: Handle,
}

impl QueryService {
    pub fn new(storage: Arc<StorageService>, runtime: Handle) -> Self {
        Self {
            active_queries: SyncRwLock::new(HashMap::new()),
            storage,
            runtime,
        }
    }

    /// Execute a single SQL query
    pub fn execute_query(
        &self,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
        sql: String,
        params: Vec<QueryParam>,
        options: QueryOptions,
    ) -> Result<QueryResult> {
        self.runtime.block_on(async {
            self.execute_query_async(pool, connection_id, sql, params, options, None, None).await
        })
    }

    /// Execute a query with streaming callback
    pub fn execute_query_streaming(
        &self,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
        sql: String,
        params: Vec<QueryParam>,
        options: QueryOptions,
        on_batch: RowBatchCallback,
        on_complete: CompleteCallback,
    ) -> Result<Uuid> {
        let query_id = Uuid::new_v4();

        // Clone what we need for the async task
        let storage = self.storage.clone();
        let active_queries = self.active_queries.clone();
        let runtime = self.runtime.clone();

        // Spawn the query execution
        self.runtime.spawn(async move {
            let started_at = Instant::now();

            // Get connection
            let client = match pool.get().await {
                Ok(c) => c,
                Err(e) => {
                    let error = QueryError {
                        message: format!("Failed to get connection: {}", e),
                        detail: None,
                        hint: Some("Check that the connection is still active".to_string()),
                        position: None,
                        internal_position: None,
                        internal_query: None,
                        code: "08000".to_string(), // connection_exception
                        schema: None,
                        table: None,
                        column: None,
                        constraint: None,
                        severity: ErrorSeverity::Error,
                    };
                    on_complete(QueryComplete {
                        query_id,
                        total_rows: 0,
                        elapsed_ms: started_at.elapsed().as_millis() as u64,
                        truncated: false,
                    });
                    return;
                }
            };

            // Store cancel token
            let cancel_token = client.cancel_token();
            let (cancel_tx, cancel_rx) = oneshot::channel();
            {
                let mut active = active_queries.write();
                active.insert(query_id, ActiveQuery {
                    cancel_token,
                    started_at,
                    sql: sql.clone(),
                    cancel_tx: Some(cancel_tx),
                });
            }

            // Execute with streaming
            let result = execute_streaming_inner(
                &client,
                &sql,
                &params,
                query_id,
                &options,
                on_batch,
                cancel_rx,
            ).await;

            // Remove from active
            {
                let mut active = active_queries.write();
                active.remove(&query_id);
            }

            let elapsed_ms = started_at.elapsed().as_millis() as u64;

            // Record history
            let _ = storage.record_query_history(
                connection_id,
                &sql,
                elapsed_ms,
                result.as_ref().ok().and_then(|r| r.total_rows),
                result.as_ref().err().map(|e| e.to_string()),
            ).await;

            // Send completion
            match result {
                Ok((total_rows, truncated)) => {
                    on_complete(QueryComplete {
                        query_id,
                        total_rows,
                        elapsed_ms,
                        truncated,
                    });
                }
                Err(_) => {
                    on_complete(QueryComplete {
                        query_id,
                        total_rows: 0,
                        elapsed_ms,
                        truncated: false,
                    });
                }
            }
        });

        Ok(query_id)
    }

    /// Execute query asynchronously (internal)
    async fn execute_query_async(
        &self,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
        sql: String,
        params: Vec<QueryParam>,
        options: QueryOptions,
        on_batch: Option<RowBatchCallback>,
        on_progress: Option<ProgressCallback>,
    ) -> Result<QueryResult> {
        let query_id = Uuid::new_v4();
        let started_at = Instant::now();

        // Get connection from pool
        let client = pool.get().await
            .map_err(|e| Error::Connection(e.to_string()))?;

        // Store cancel token
        let cancel_token = client.cancel_token();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut active = self.active_queries.write();
            active.insert(query_id, ActiveQuery {
                cancel_token,
                started_at,
                sql: sql.clone(),
                cancel_tx: Some(cancel_tx),
            });
        }

        // Apply statement timeout if configured
        if let Some(timeout_ms) = options.statement_timeout_ms {
            let timeout_sql = format!("SET LOCAL statement_timeout = {}", timeout_ms);
            if let Err(e) = client.execute(&timeout_sql, &[]).await {
                tracing::warn!("Failed to set statement timeout: {}", e);
            }
        }

        // Apply read-only if configured
        if options.read_only {
            if let Err(e) = client.execute("SET LOCAL transaction_read_only = on", &[]).await {
                tracing::warn!("Failed to set read-only: {}", e);
            }
        }

        // Convert params
        let pg_params = convert_params(&params)?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            pg_params.iter().map(|p| p.as_ref()).collect();

        // Execute
        let result = execute_query_inner(
            &client,
            &sql,
            &param_refs,
            query_id,
            &options,
            on_batch,
            cancel_rx,
        ).await;

        // Remove from active queries
        {
            let mut active = self.active_queries.write();
            active.remove(&query_id);
        }

        let elapsed_ms = started_at.elapsed().as_millis() as u64;

        // Record in history
        let _ = self.storage.record_query_history(
            connection_id,
            &sql,
            elapsed_ms,
            result.as_ref().ok().and_then(|r| r.total_rows),
            result.as_ref().err().map(|e| e.to_string()),
        ).await;

        match result {
            Ok(mut query_result) => {
                query_result.query_id = query_id;
                query_result.elapsed_ms = elapsed_ms;
                Ok(query_result)
            }
            Err(e) => {
                let error = QueryError::from_postgres(&e, &sql);
                Ok(QueryResult::error(query_id, sql, error, elapsed_ms))
            }
        }
    }

    /// Cancel a running query
    pub fn cancel_query(&self, query_id: Uuid) -> Result<()> {
        let cancel_token = {
            let mut active = self.active_queries.write();
            if let Some(query) = active.get_mut(&query_id) {
                // Send cancel signal through channel
                if let Some(tx) = query.cancel_tx.take() {
                    let _ = tx.send(());
                }
                Some(query.cancel_token.clone())
            } else {
                None
            }
        };

        if let Some(token) = cancel_token {
            // Send cancel request to Postgres
            self.runtime.spawn(async move {
                if let Err(e) = token.cancel_query(tokio_postgres::NoTls).await {
                    tracing::warn!("Failed to cancel query: {}", e);
                }
            });
            Ok(())
        } else {
            Err(Error::QueryNotFound(query_id.to_string()))
        }
    }

    /// Execute multiple statements sequentially
    pub fn execute_multiple(
        &self,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
        sql: String,
        options: QueryOptions,
    ) -> Result<Vec<QueryResult>> {
        self.runtime.block_on(async {
            let statements = split_statements(&sql)?;
            let mut results = Vec::with_capacity(statements.len());

            for statement in statements {
                let trimmed = statement.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let result = self.execute_query_async(
                    pool.clone(),
                    connection_id,
                    trimmed.to_string(),
                    Vec::new(),
                    options.clone(),
                    None,
                    None,
                ).await?;

                let is_error = result.status == QueryStatus::Error;
                results.push(result);

                if is_error && options.stop_on_error {
                    break;
                }
            }

            Ok(results)
        })
    }

    /// Get EXPLAIN plan for a query
    pub fn explain_query(
        &self,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
        sql: String,
        explain_options: ExplainOptions,
    ) -> Result<QueryResult> {
        self.runtime.block_on(async {
            let explain_sql = build_explain_sql(&sql, &explain_options);

            let result = self.execute_query_async(
                pool,
                connection_id,
                explain_sql,
                Vec::new(),
                QueryOptions::quick_query(),
                None,
                None,
            ).await?;

            // Parse plan if JSON format
            if explain_options.format == PlanFormat::Json {
                if let Some(rows) = &result.rows {
                    if let Some(first_row) = rows.first() {
                        if let Some(Value::Json(plan_json)) = first_row.first() {
                            if let Some(plan_array) = plan_json.as_array() {
                                if let Some(plan_obj) = plan_array.first() {
                                    let plan = parse_explain_json(plan_obj);
                                    return Ok(QueryResult {
                                        plan: Some(plan),
                                        ..result
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Ok(result)
        })
    }

    /// Check if a query is currently running
    pub fn is_query_running(&self, query_id: Uuid) -> bool {
        self.active_queries.read().contains_key(&query_id)
    }

    /// Get active query count
    pub fn active_query_count(&self) -> usize {
        self.active_queries.read().len()
    }

    /// Get info about running queries
    pub fn get_running_queries(&self) -> Vec<RunningQueryInfo> {
        self.active_queries.read()
            .iter()
            .map(|(id, q)| RunningQueryInfo {
                query_id: *id,
                sql: q.sql.clone(),
                running_for_ms: q.started_at.elapsed().as_millis() as u64,
            })
            .collect()
    }
}

/// Info about a running query
#[derive(Clone, Debug)]
pub struct RunningQueryInfo {
    pub query_id: Uuid,
    pub sql: String,
    pub running_for_ms: u64,
}

/// Query parameter for prepared statements
#[derive(Clone, Debug)]
pub enum QueryParam {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Uuid(Uuid),
}

/// Convert params to tokio-postgres format
fn convert_params(params: &[QueryParam]) -> Result<Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>> {
    let mut pg_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();

    for param in params {
        let boxed: Box<dyn tokio_postgres::types::ToSql + Sync + Send> = match param {
            QueryParam::Null => Box::new(None::<String>),
            QueryParam::Bool(b) => Box::new(*b),
            QueryParam::Int(n) => Box::new(*n),
            QueryParam::Float(f) => Box::new(*f),
            QueryParam::String(s) => Box::new(s.clone()),
            QueryParam::Bytes(b) => Box::new(b.clone()),
            QueryParam::Json(j) => Box::new(j.clone()),
            QueryParam::Uuid(u) => Box::new(*u),
        };
        pg_params.push(boxed);
    }

    Ok(pg_params)
}

/// Execute query and return result (internal)
async fn execute_query_inner(
    client: &tokio_postgres::Client,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    query_id: Uuid,
    options: &QueryOptions,
    on_batch: Option<RowBatchCallback>,
    mut cancel_rx: oneshot::Receiver<()>,
) -> std::result::Result<QueryResult, tokio_postgres::Error> {
    // Prepare statement to get column info
    let statement = client.prepare(sql).await?;
    let columns = extract_column_meta(&statement);
    let command = detect_command_type(sql);

    if command == "SELECT" || command == "TABLE" || command == "VALUES" || command == "WITH" {
        // For SELECT queries, stream results
        let row_stream = client.query_raw(&statement, params.iter().copied()).await?;
        tokio::pin!(row_stream);

        let batch_size = options.batch_size;
        let row_limit = options.row_limit;

        let mut all_rows: Vec<Vec<Value>> = Vec::new();
        let mut batch: Vec<Vec<Value>> = Vec::with_capacity(batch_size);
        let mut batch_num = 0u32;
        let mut total_rows = 0u64;
        let mut truncated = false;

        loop {
            tokio::select! {
                biased;

                // Check for cancellation
                _ = &mut cancel_rx => {
                    return Ok(QueryResult::cancelled(query_id, sql.to_string(), 0));
                }

                // Get next row
                row_result = row_stream.next() => {
                    match row_result {
                        Some(Ok(row)) => {
                            // Check row limit
                            if let Some(limit) = row_limit {
                                if total_rows >= limit {
                                    truncated = true;
                                    break;
                                }
                            }

                            let values = row_to_values(&row, &columns)?;
                            batch.push(values);
                            total_rows += 1;

                            // Emit batch when full
                            if batch.len() >= batch_size {
                                let batch_data = std::mem::replace(
                                    &mut batch,
                                    Vec::with_capacity(batch_size)
                                );

                                if let Some(ref callback) = on_batch {
                                    callback(RowBatch {
                                        query_id,
                                        rows: batch_data.clone(),
                                        batch_num,
                                        is_final: false,
                                    });
                                }

                                all_rows.extend(batch_data);
                                batch_num += 1;
                            }
                        }
                        Some(Err(e)) => return Err(e),
                        None => break, // Stream complete
                    }
                }
            }
        }

        // Emit final partial batch
        if !batch.is_empty() {
            if let Some(ref callback) = on_batch {
                callback(RowBatch {
                    query_id,
                    rows: batch.clone(),
                    batch_num,
                    is_final: true,
                });
            }
            all_rows.extend(batch);
        }

        Ok(QueryResult {
            query_id,
            status: QueryStatus::Success,
            command,
            sql: sql.to_string(),
            columns: Some(columns),
            rows: Some(all_rows),
            total_rows: Some(total_rows),
            truncated: Some(truncated),
            rows_affected: None,
            plan: None,
            elapsed_ms: 0,
            error: None,
        })
    } else {
        // For DML/DDL, execute and return affected rows
        let rows_affected = client.execute_raw(&statement, params.iter().copied()).await?;

        Ok(QueryResult::success_dml(
            query_id,
            command,
            sql.to_string(),
            rows_affected,
            0,
        ))
    }
}

/// Execute with streaming results
async fn execute_streaming_inner(
    client: &tokio_postgres::Client,
    sql: &str,
    params: &[QueryParam],
    query_id: Uuid,
    options: &QueryOptions,
    on_batch: RowBatchCallback,
    mut cancel_rx: oneshot::Receiver<()>,
) -> std::result::Result<(u64, bool), tokio_postgres::Error> {
    // Convert params
    let pg_params = convert_params(params).map_err(|_| {
        tokio_postgres::Error::__private_api_timeout()
    })?;
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
        pg_params.iter().map(|p| p.as_ref()).collect();

    // Prepare and execute
    let statement = client.prepare(sql).await?;
    let columns = extract_column_meta(&statement);

    let row_stream = client.query_raw(&statement, param_refs.iter().copied()).await?;
    tokio::pin!(row_stream);

    let batch_size = options.batch_size;
    let row_limit = options.row_limit;

    let mut batch: Vec<Vec<Value>> = Vec::with_capacity(batch_size);
    let mut batch_num = 0u32;
    let mut total_rows = 0u64;
    let mut truncated = false;

    loop {
        tokio::select! {
            biased;

            _ = &mut cancel_rx => {
                return Ok((total_rows, truncated));
            }

            row_result = row_stream.next() => {
                match row_result {
                    Some(Ok(row)) => {
                        if let Some(limit) = row_limit {
                            if total_rows >= limit {
                                truncated = true;
                                break;
                            }
                        }

                        let values = row_to_values(&row, &columns)?;
                        batch.push(values);
                        total_rows += 1;

                        if batch.len() >= batch_size {
                            let batch_data = std::mem::replace(
                                &mut batch,
                                Vec::with_capacity(batch_size)
                            );

                            on_batch(RowBatch {
                                query_id,
                                rows: batch_data,
                                batch_num,
                                is_final: false,
                            });

                            batch_num += 1;
                        }
                    }
                    Some(Err(e)) => return Err(e),
                    None => break,
                }
            }
        }
    }

    // Final batch
    if !batch.is_empty() {
        on_batch(RowBatch {
            query_id,
            rows: batch,
            batch_num,
            is_final: true,
        });
    }

    Ok((total_rows, truncated))
}

/// Extract column metadata from statement
fn extract_column_meta(statement: &Statement) -> Vec<ColumnMeta> {
    statement
        .columns()
        .iter()
        .enumerate()
        .map(|(i, col)| ColumnMeta {
            name: col.name().to_string(),
            type_oid: col.type_().oid(),
            type_name: col.type_().name().to_string(),
            type_modifier: -1,
            table_oid: None,
            column_ordinal: Some(i as i32),
            nullable: true, // Not directly available from statement
        })
        .collect()
}

/// Convert a Postgres row to values
fn row_to_values(row: &Row, columns: &[ColumnMeta]) -> std::result::Result<Vec<Value>, tokio_postgres::Error> {
    let mut values = Vec::with_capacity(columns.len());

    for (i, col) in columns.iter().enumerate() {
        let value = extract_value(row, i, &col.type_name)?;
        values.push(value);
    }

    Ok(values)
}

/// Extract typed value from row
fn extract_value(row: &Row, idx: usize, type_name: &str) -> std::result::Result<Value, tokio_postgres::Error> {
    // Try to get as Option first to handle NULL
    match type_name {
        "bool" => Ok(row.try_get::<_, Option<bool>>(idx)?
            .map(Value::Bool)
            .unwrap_or(Value::Null)),

        "int2" => Ok(row.try_get::<_, Option<i16>>(idx)?
            .map(Value::SmallInt)
            .unwrap_or(Value::Null)),

        "int4" => Ok(row.try_get::<_, Option<i32>>(idx)?
            .map(Value::Int)
            .unwrap_or(Value::Null)),

        "int8" => Ok(row.try_get::<_, Option<i64>>(idx)?
            .map(Value::BigInt)
            .unwrap_or(Value::Null)),

        "float4" => Ok(row.try_get::<_, Option<f32>>(idx)?
            .map(Value::Float)
            .unwrap_or(Value::Null)),

        "float8" => Ok(row.try_get::<_, Option<f64>>(idx)?
            .map(Value::Double)
            .unwrap_or(Value::Null)),

        "numeric" => Ok(row.try_get::<_, Option<rust_decimal::Decimal>>(idx)
            .ok()
            .flatten()
            .map(|d| Value::Numeric(d.to_string()))
            .unwrap_or(Value::Null)),

        "text" | "varchar" | "bpchar" | "name" | "char" => {
            Ok(row.try_get::<_, Option<String>>(idx)?
                .map(Value::Text)
                .unwrap_or(Value::Null))
        }

        "bytea" => {
            Ok(row.try_get::<_, Option<Vec<u8>>>(idx)?
                .map(Value::Bytea)
                .unwrap_or(Value::Null))
        }

        "json" | "jsonb" => {
            Ok(row.try_get::<_, Option<serde_json::Value>>(idx)?
                .map(Value::Json)
                .unwrap_or(Value::Null))
        }

        "uuid" => {
            Ok(row.try_get::<_, Option<uuid::Uuid>>(idx)?
                .map(Value::Uuid)
                .unwrap_or(Value::Null))
        }

        "date" => {
            Ok(row.try_get::<_, Option<chrono::NaiveDate>>(idx)
                .ok()
                .flatten()
                .map(|d| Value::Date(d.to_string()))
                .unwrap_or(Value::Null))
        }

        "time" => {
            Ok(row.try_get::<_, Option<chrono::NaiveTime>>(idx)
                .ok()
                .flatten()
                .map(|t| Value::Time(t.to_string()))
                .unwrap_or(Value::Null))
        }

        "timetz" => {
            Ok(row.try_get::<_, Option<String>>(idx)
                .ok()
                .flatten()
                .map(Value::Time)
                .unwrap_or(Value::Null))
        }

        "timestamp" => {
            Ok(row.try_get::<_, Option<chrono::NaiveDateTime>>(idx)
                .ok()
                .flatten()
                .map(|ts| Value::Timestamp(ts.to_string()))
                .unwrap_or(Value::Null))
        }

        "timestamptz" => {
            Ok(row.try_get::<_, Option<chrono::DateTime<chrono::Utc>>>(idx)
                .ok()
                .flatten()
                .map(|ts| Value::TimestampTz(ts.to_rfc3339()))
                .unwrap_or(Value::Null))
        }

        "interval" => {
            // Postgres interval is complex, get as string
            Ok(row.try_get::<_, Option<String>>(idx)
                .ok()
                .flatten()
                .map(Value::Interval)
                .unwrap_or(Value::Null))
        }

        "point" => {
            // Try to get geometric point
            Ok(Value::Unknown("(point)".to_string()))
        }

        // Array types (start with _)
        t if t.starts_with('_') => {
            let inner_type = &t[1..];
            extract_array_value(row, idx, inner_type)
        }

        // Unknown - try string representation
        _ => {
            Ok(row.try_get::<_, Option<String>>(idx)
                .ok()
                .flatten()
                .map(Value::Unknown)
                .unwrap_or(Value::Null))
        }
    }
}

/// Extract array values
fn extract_array_value(row: &Row, idx: usize, inner_type: &str) -> std::result::Result<Value, tokio_postgres::Error> {
    match inner_type {
        "int4" => Ok(row.try_get::<_, Option<Vec<i32>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::Int).collect()))
            .unwrap_or(Value::Null)),

        "int8" => Ok(row.try_get::<_, Option<Vec<i64>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::BigInt).collect()))
            .unwrap_or(Value::Null)),

        "text" | "varchar" => Ok(row.try_get::<_, Option<Vec<String>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::Text).collect()))
            .unwrap_or(Value::Null)),

        "bool" => Ok(row.try_get::<_, Option<Vec<bool>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::Bool).collect()))
            .unwrap_or(Value::Null)),

        "float8" => Ok(row.try_get::<_, Option<Vec<f64>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::Double).collect()))
            .unwrap_or(Value::Null)),

        "uuid" => Ok(row.try_get::<_, Option<Vec<uuid::Uuid>>>(idx)?
            .map(|v| Value::Array(v.into_iter().map(Value::Uuid).collect()))
            .unwrap_or(Value::Null)),

        _ => Ok(Value::Unknown(format!("({}[])", inner_type))),
    }
}

/// Detect SQL command type
fn detect_command_type(sql: &str) -> String {
    let sql_upper = sql.trim().to_uppercase();

    for cmd in &[
        "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER",
        "DROP", "TRUNCATE", "GRANT", "REVOKE", "VACUUM", "ANALYZE",
        "EXPLAIN", "TABLE", "VALUES", "WITH", "COPY", "DO",
        "CALL", "REFRESH", "REINDEX", "CLUSTER", "COMMENT",
        "LOCK", "NOTIFY", "LISTEN", "UNLISTEN", "SET", "SHOW",
        "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE",
    ] {
        if sql_upper.starts_with(cmd) {
            return cmd.to_string();
        }
    }

    "UNKNOWN".to_string()
}

/// Split SQL into individual statements
fn split_statements(sql: &str) -> Result<Vec<String>> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_string = false;
    let mut string_char = '"';
    let mut in_dollar_quote = false;
    let mut dollar_tag = String::new();
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut block_comment_depth = 0;

    while let Some(c) = chars.next() {
        // Line comment handling
        if in_line_comment {
            current.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        // Block comment handling
        if in_block_comment {
            current.push(c);
            if c == '*' {
                if let Some(&'/') = chars.peek() {
                    current.push(chars.next().unwrap());
                    block_comment_depth -= 1;
                    if block_comment_depth == 0 {
                        in_block_comment = false;
                    }
                }
            } else if c == '/' {
                if let Some(&'*') = chars.peek() {
                    current.push(chars.next().unwrap());
                    block_comment_depth += 1;
                }
            }
            continue;
        }

        current.push(c);

        match c {
            // Start of line comment
            '-' if !in_string && !in_dollar_quote => {
                if chars.peek() == Some(&'-') {
                    current.push(chars.next().unwrap());
                    in_line_comment = true;
                }
            }

            // Start of block comment
            '/' if !in_string && !in_dollar_quote => {
                if chars.peek() == Some(&'*') {
                    current.push(chars.next().unwrap());
                    in_block_comment = true;
                    block_comment_depth = 1;
                }
            }

            // String handling
            '\'' | '"' if !in_dollar_quote => {
                if !in_string {
                    in_string = true;
                    string_char = c;
                } else if c == string_char {
                    // Check for escaped quote
                    if chars.peek() == Some(&c) {
                        current.push(chars.next().unwrap());
                    } else {
                        in_string = false;
                    }
                }
            }

            // Dollar quote handling
            '$' if !in_string => {
                if in_dollar_quote {
                    // Check if this ends the dollar quote
                    let mut potential_end = String::from("$");
                    let mut temp_chars = chars.clone();

                    while let Some(&next) = temp_chars.peek() {
                        if next == '$' {
                            potential_end.push(temp_chars.next().unwrap());
                            break;
                        } else if next.is_alphanumeric() || next == '_' {
                            potential_end.push(temp_chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    if potential_end == format!("${}$", dollar_tag) {
                        for _ in 1..potential_end.len() {
                            current.push(chars.next().unwrap());
                        }
                        in_dollar_quote = false;
                        dollar_tag.clear();
                    }
                } else {
                    // Check if this starts a dollar quote
                    let mut tag = String::new();
                    let mut temp_chars = chars.clone();

                    while let Some(&next) = temp_chars.peek() {
                        if next == '$' {
                            temp_chars.next();
                            in_dollar_quote = true;
                            dollar_tag = tag.clone();

                            for _ in 0..tag.len() {
                                current.push(chars.next().unwrap());
                            }
                            current.push(chars.next().unwrap()); // closing $
                            break;
                        } else if next.is_alphanumeric() || next == '_' {
                            tag.push(temp_chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
            }

            // Statement separator
            ';' if !in_string && !in_dollar_quote => {
                statements.push(current.clone());
                current.clear();
            }

            _ => {}
        }
    }

    // Don't forget the last statement
    if !current.trim().is_empty() {
        statements.push(current);
    }

    Ok(statements)
}

/// Build EXPLAIN SQL from options
fn build_explain_sql(sql: &str, options: &ExplainOptions) -> String {
    let mut explain_options = Vec::new();

    if options.analyze {
        explain_options.push("ANALYZE");
    }
    if options.buffers {
        explain_options.push("BUFFERS");
    }
    if options.verbose {
        explain_options.push("VERBOSE");
    }
    if !options.costs {
        explain_options.push("COSTS OFF");
    }
    if !options.timing && options.analyze {
        explain_options.push("TIMING OFF");
    }
    if options.wal && options.analyze {
        explain_options.push("WAL");
    }

    let format_str = match options.format {
        PlanFormat::Text => "TEXT",
        PlanFormat::Json => "JSON",
        PlanFormat::Xml => "XML",
        PlanFormat::Yaml => "YAML",
    };
    explain_options.push(&format!("FORMAT {}", format_str));

    if explain_options.is_empty() {
        format!("EXPLAIN {}", sql)
    } else {
        format!("EXPLAIN ({}) {}", explain_options.join(", "), sql)
    }
}

/// Parse EXPLAIN JSON output
fn parse_explain_json(json: &serde_json::Value) -> QueryPlan {
    let obj = json.as_object();

    let planning_time = obj
        .and_then(|o| o.get("Planning Time"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let execution_time = obj
        .and_then(|o| o.get("Execution Time"))
        .and_then(|v| v.as_f64());

    let root = obj
        .and_then(|o| o.get("Plan"))
        .and_then(PlanNode::from_json);

    QueryPlan {
        raw: json.to_string(),
        format: PlanFormat::Json,
        root,
        planning_time_ms: planning_time,
        execution_time_ms: execution_time,
    }
}
```

### 11.3 GPUI Query Execution State

```rust
// src/ui/state/query_execution.rs

use gpui::*;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::models::query::*;
use crate::services::query::{QueryService, QueryOptions, RowBatchCallback, CompleteCallback};
use crate::services::connection::ConnectionPool;

/// State for an executing or completed query
#[derive(Clone)]
pub struct ExecutingQuery {
    /// Unique query ID
    pub query_id: Uuid,
    /// Original SQL
    pub sql: String,
    /// Connection ID
    pub connection_id: Uuid,
    /// When execution started
    pub started_at: std::time::Instant,
    /// Current status
    pub status: QueryStatus,
    /// Column metadata (once available)
    pub columns: Option<Vec<ColumnMeta>>,
    /// Accumulated rows (for non-streaming or small results)
    pub rows: Vec<Vec<Value>>,
    /// Total rows received
    pub total_rows: u64,
    /// Whether results are truncated
    pub truncated: bool,
    /// Execution time
    pub elapsed_ms: Option<u64>,
    /// Error if any
    pub error: Option<QueryError>,
    /// Execution plan if requested
    pub plan: Option<QueryPlan>,
}

impl ExecutingQuery {
    pub fn new(query_id: Uuid, sql: String, connection_id: Uuid) -> Self {
        Self {
            query_id,
            sql,
            connection_id,
            started_at: std::time::Instant::now(),
            status: QueryStatus::Running,
            columns: None,
            rows: Vec::new(),
            total_rows: 0,
            truncated: false,
            elapsed_ms: None,
            error: None,
            plan: None,
        }
    }

    pub fn from_result(result: QueryResult, connection_id: Uuid) -> Self {
        Self {
            query_id: result.query_id,
            sql: result.sql,
            connection_id,
            started_at: std::time::Instant::now(),
            status: result.status,
            columns: result.columns,
            rows: result.rows.unwrap_or_default(),
            total_rows: result.total_rows.unwrap_or(0),
            truncated: result.truncated.unwrap_or(false),
            elapsed_ms: Some(result.elapsed_ms),
            error: result.error,
            plan: result.plan,
        }
    }

    /// Get running time in ms
    pub fn running_time_ms(&self) -> u64 {
        self.elapsed_ms.unwrap_or_else(|| {
            self.started_at.elapsed().as_millis() as u64
        })
    }

    /// Check if query is still running
    pub fn is_running(&self) -> bool {
        self.status == QueryStatus::Running
    }

    /// Check if query succeeded
    pub fn is_success(&self) -> bool {
        self.status == QueryStatus::Success
    }

    /// Check if query failed
    pub fn is_error(&self) -> bool {
        self.status == QueryStatus::Error
    }
}

/// Global query execution state
pub struct QueryExecutionState {
    /// All tracked queries by ID
    queries: HashMap<Uuid, ExecutingQuery>,
    /// Queries by connection for quick lookup
    queries_by_connection: HashMap<Uuid, Vec<Uuid>>,
    /// Most recent query for each connection
    active_query: HashMap<Uuid, Uuid>,
}

impl QueryExecutionState {
    pub fn new() -> Self {
        Self {
            queries: HashMap::new(),
            queries_by_connection: HashMap::new(),
            active_query: HashMap::new(),
        }
    }

    /// Start tracking a new query
    pub fn start_query(&mut self, query_id: Uuid, sql: String, connection_id: Uuid) {
        let query = ExecutingQuery::new(query_id, sql, connection_id);

        self.queries.insert(query_id, query);
        self.queries_by_connection
            .entry(connection_id)
            .or_default()
            .push(query_id);
        self.active_query.insert(connection_id, query_id);
    }

    /// Update query with row batch
    pub fn add_row_batch(&mut self, query_id: Uuid, batch: RowBatch) {
        if let Some(query) = self.queries.get_mut(&query_id) {
            query.total_rows += batch.rows.len() as u64;
            query.rows.extend(batch.rows);
        }
    }

    /// Mark query as complete
    pub fn complete_query(&mut self, query_id: Uuid, complete: QueryComplete) {
        if let Some(query) = self.queries.get_mut(&query_id) {
            query.status = QueryStatus::Success;
            query.total_rows = complete.total_rows;
            query.elapsed_ms = Some(complete.elapsed_ms);
            query.truncated = complete.truncated;
        }
    }

    /// Mark query as failed
    pub fn fail_query(&mut self, query_id: Uuid, error: QueryError, elapsed_ms: u64) {
        if let Some(query) = self.queries.get_mut(&query_id) {
            query.status = QueryStatus::Error;
            query.error = Some(error);
            query.elapsed_ms = Some(elapsed_ms);
        }
    }

    /// Mark query as cancelled
    pub fn cancel_query(&mut self, query_id: Uuid) {
        if let Some(query) = self.queries.get_mut(&query_id) {
            query.status = QueryStatus::Cancelled;
            query.elapsed_ms = Some(query.started_at.elapsed().as_millis() as u64);
        }
    }

    /// Update from complete result
    pub fn update_from_result(&mut self, result: QueryResult, connection_id: Uuid) {
        if let Some(query) = self.queries.get_mut(&result.query_id) {
            query.status = result.status;
            query.columns = result.columns;
            query.rows = result.rows.unwrap_or_default();
            query.total_rows = result.total_rows.unwrap_or(0);
            query.truncated = result.truncated.unwrap_or(false);
            query.elapsed_ms = Some(result.elapsed_ms);
            query.error = result.error;
            query.plan = result.plan;
        } else {
            // Insert new query from result
            let query = ExecutingQuery::from_result(result.clone(), connection_id);
            self.queries.insert(result.query_id, query);
            self.queries_by_connection
                .entry(connection_id)
                .or_default()
                .push(result.query_id);
            self.active_query.insert(connection_id, result.query_id);
        }
    }

    /// Get query by ID
    pub fn get_query(&self, query_id: Uuid) -> Option<&ExecutingQuery> {
        self.queries.get(&query_id)
    }

    /// Get mutable query by ID
    pub fn get_query_mut(&mut self, query_id: Uuid) -> Option<&mut ExecutingQuery> {
        self.queries.get_mut(&query_id)
    }

    /// Get active query for connection
    pub fn get_active_query(&self, connection_id: Uuid) -> Option<&ExecutingQuery> {
        self.active_query.get(&connection_id)
            .and_then(|id| self.queries.get(id))
    }

    /// Get all queries for connection
    pub fn get_queries_for_connection(&self, connection_id: Uuid) -> Vec<&ExecutingQuery> {
        self.queries_by_connection
            .get(&connection_id)
            .map(|ids| ids.iter().filter_map(|id| self.queries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get running queries
    pub fn get_running_queries(&self) -> Vec<&ExecutingQuery> {
        self.queries.values()
            .filter(|q| q.is_running())
            .collect()
    }

    /// Clear completed queries for connection
    pub fn clear_completed(&mut self, connection_id: Uuid) {
        if let Some(query_ids) = self.queries_by_connection.get_mut(&connection_id) {
            query_ids.retain(|id| {
                self.queries.get(id).map(|q| q.is_running()).unwrap_or(false)
            });
        }

        self.queries.retain(|_, q| {
            q.connection_id != connection_id || q.is_running()
        });
    }

    /// Remove query
    pub fn remove_query(&mut self, query_id: Uuid) {
        if let Some(query) = self.queries.remove(&query_id) {
            if let Some(ids) = self.queries_by_connection.get_mut(&query.connection_id) {
                ids.retain(|id| *id != query_id);
            }
            if self.active_query.get(&query.connection_id) == Some(&query_id) {
                self.active_query.remove(&query.connection_id);
            }
        }
    }
}

impl Global for QueryExecutionState {}

/// GPUI entity for query execution
pub struct QueryExecutor {
    service: Arc<QueryService>,
    connection_pool: Arc<ConnectionPool>,
    connection_id: Uuid,
}

impl QueryExecutor {
    pub fn new(
        service: Arc<QueryService>,
        pool: Arc<ConnectionPool>,
        connection_id: Uuid,
    ) -> Self {
        Self {
            service,
            connection_pool: pool,
            connection_id,
        }
    }

    /// Execute a query and update state
    pub fn execute(&self, sql: String, options: QueryOptions, cx: &mut Context<Self>) {
        let service = self.service.clone();
        let pool = self.connection_pool.clone();
        let connection_id = self.connection_id;

        cx.spawn(|this, mut cx| async move {
            // Create temporary ID for tracking
            let temp_id = Uuid::new_v4();

            // Register in global state
            cx.update_global::<QueryExecutionState, _>(|state, _| {
                state.start_query(temp_id, sql.clone(), connection_id);
            }).ok();

            // Execute query
            let result = service.execute_query(
                pool,
                connection_id,
                sql,
                Vec::new(),
                options,
            );

            // Update state with result
            match result {
                Ok(result) => {
                    cx.update_global::<QueryExecutionState, _>(|state, _| {
                        // Remove temp tracking
                        state.remove_query(temp_id);
                        // Add real result
                        state.update_from_result(result, connection_id);
                    }).ok();
                }
                Err(e) => {
                    let error = QueryError {
                        message: e.to_string(),
                        detail: None,
                        hint: None,
                        position: None,
                        internal_position: None,
                        internal_query: None,
                        code: String::new(),
                        schema: None,
                        table: None,
                        column: None,
                        constraint: None,
                        severity: ErrorSeverity::Error,
                    };
                    cx.update_global::<QueryExecutionState, _>(|state, _| {
                        state.fail_query(temp_id, error, 0);
                    }).ok();
                }
            }

            // Notify UI to refresh
            this.update(&mut cx, |_, cx| {
                cx.notify();
            }).ok();
        }).detach();
    }

    /// Execute query with streaming updates
    pub fn execute_streaming(
        &self,
        sql: String,
        options: QueryOptions,
        cx: &mut Context<Self>,
    ) -> Uuid {
        let service = self.service.clone();
        let pool = self.connection_pool.clone();
        let connection_id = self.connection_id;

        let query_id = Uuid::new_v4();

        // Register in global state
        cx.update_global::<QueryExecutionState, _>(|state, _| {
            state.start_query(query_id, sql.clone(), connection_id);
        });

        // Create callbacks that update GPUI state
        let batch_query_id = query_id;
        let on_batch: RowBatchCallback = Arc::new(move |batch| {
            // This will be called from async context
            // We need to update global state
            // Note: In production, use a channel to communicate with GPUI
            tracing::debug!("Received batch {} with {} rows", batch.batch_num, batch.rows.len());
        });

        let complete_query_id = query_id;
        let on_complete: CompleteCallback = Arc::new(move |complete| {
            tracing::debug!("Query {} complete: {} rows", complete.query_id, complete.total_rows);
        });

        // Start streaming execution
        cx.spawn(|this, mut cx| async move {
            let result = service.execute_query_streaming(
                pool,
                connection_id,
                sql,
                Vec::new(),
                options,
                on_batch,
                on_complete,
            );

            if let Err(e) = result {
                let error = QueryError {
                    message: e.to_string(),
                    detail: None,
                    hint: None,
                    position: None,
                    internal_position: None,
                    internal_query: None,
                    code: String::new(),
                    schema: None,
                    table: None,
                    column: None,
                    constraint: None,
                    severity: ErrorSeverity::Error,
                };
                cx.update_global::<QueryExecutionState, _>(|state, _| {
                    state.fail_query(query_id, error, 0);
                }).ok();
            }
        }).detach();

        query_id
    }

    /// Cancel running query
    pub fn cancel(&self, query_id: Uuid, cx: &mut Context<Self>) {
        let service = self.service.clone();

        if let Err(e) = service.cancel_query(query_id) {
            tracing::warn!("Failed to cancel query {}: {}", query_id, e);
        }

        cx.update_global::<QueryExecutionState, _>(|state, _| {
            state.cancel_query(query_id);
        });

        cx.notify();
    }
}

impl EventEmitter<QueryEvent> for QueryExecutor {}

/// Events emitted by query executor
pub enum QueryEvent {
    Started { query_id: Uuid },
    RowBatch { query_id: Uuid, batch: RowBatch },
    Progress { query_id: Uuid, progress: QueryProgress },
    Completed { query_id: Uuid, result: QueryResult },
    Cancelled { query_id: Uuid },
    Error { query_id: Uuid, error: QueryError },
}
```

### 11.4 Query Execution UI Components

```rust
// src/ui/components/query_status.rs

use gpui::*;
use uuid::Uuid;

use crate::models::query::*;
use crate::ui::state::query_execution::{QueryExecutionState, ExecutingQuery};
use crate::ui::theme::Theme;

/// Displays status of running/completed queries
pub struct QueryStatusBar {
    connection_id: Uuid,
}

impl QueryStatusBar {
    pub fn new(connection_id: Uuid) -> Self {
        Self { connection_id }
    }
}

impl Render for QueryStatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<QueryExecutionState>();

        let query = state.get_active_query(self.connection_id);

        div()
            .h(px(28.0))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .child(
                if let Some(query) = query {
                    self.render_query_status(query, theme)
                } else {
                    self.render_idle(theme)
                }
            )
    }
}

impl QueryStatusBar {
    fn render_idle(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(8.0))
                    .rounded_full()
                    .bg(theme.text_muted)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("Ready")
            )
    }

    fn render_query_status(&self, query: &ExecutingQuery, theme: &Theme) -> impl IntoElement {
        let (indicator_color, status_text) = match query.status {
            QueryStatus::Running => (theme.warning, "Running..."),
            QueryStatus::Success => (theme.success, "Success"),
            QueryStatus::Error => (theme.error, "Error"),
            QueryStatus::Cancelled => (theme.text_muted, "Cancelled"),
        };

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .flex_1()
            .child(
                // Status indicator
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(8.0))
                            .rounded_full()
                            .bg(indicator_color)
                            .when(query.is_running(), |el| {
                                el.with_animation(
                                    "pulse",
                                    Animation::new(Duration::from_millis(1000))
                                        .repeat()
                                        .with_easing(pulsing_opacity()),
                                    |el, delta| el.opacity(0.3 + delta * 0.7)
                                )
                            })
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text)
                            .child(status_text)
                    )
            )
            .child(
                // Timing
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format_duration(query.running_time_ms()))
            )
            .child(
                // Row count
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format_row_count(query.total_rows, query.truncated))
            )
            .when(query.is_running(), |el| {
                el.child(
                    // Cancel button
                    Button::new("cancel")
                        .label("Cancel")
                        .size(ButtonSize::Small)
                        .style(ButtonStyle::Ghost)
                        .on_click(cx.listener(|this, _, cx| {
                            // Handle cancel
                        }))
                )
            })
            .when(query.is_error(), |el| {
                if let Some(error) = &query.error {
                    el.child(
                        div()
                            .text_sm()
                            .text_color(theme.error)
                            .max_w(px(400.0))
                            .truncate()
                            .child(error.message.clone())
                    )
                } else {
                    el
                }
            })
    }
}

/// Format duration for display
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

/// Format row count for display
fn format_row_count(rows: u64, truncated: bool) -> String {
    let count = if rows >= 1_000_000 {
        format!("{:.1}M", rows as f64 / 1_000_000.0)
    } else if rows >= 1000 {
        format!("{:.1}K", rows as f64 / 1000.0)
    } else {
        rows.to_string()
    };

    if truncated {
        format!("{}+ rows", count)
    } else {
        format!("{} rows", count)
    }
}

/// Running queries list panel
pub struct RunningQueriesPanel {
    expanded: bool,
}

impl RunningQueriesPanel {
    pub fn new() -> Self {
        Self { expanded: false }
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }
}

impl Render for RunningQueriesPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<QueryExecutionState>();
        let running = state.get_running_queries();

        if running.is_empty() {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded(px(6.0))
            .shadow_md()
            .child(
                // Header
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, cx| {
                        this.toggle();
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(Icon::new(IconName::Loader).size(px(16.0)).color(theme.warning))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child(format!("{} Running", running.len()))
                            )
                    )
                    .child(
                        Icon::new(if self.expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                            .size(px(16.0))
                            .color(theme.text_muted)
                    )
            )
            .when(self.expanded, |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .max_h(px(200.0))
                        .overflow_y_auto()
                        .children(
                            running.iter().map(|query| {
                                self.render_query_row(query, theme, cx)
                            })
                        )
                )
            })
            .into_any_element()
    }
}

impl RunningQueriesPanel {
    fn render_query_row(&self, query: &ExecutingQuery, theme: &Theme, cx: &Context<Self>) -> impl IntoElement {
        div()
            .px(px(12.0))
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(theme.border)
            .hover(|s| s.bg(theme.surface_hover))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text)
                            .truncate()
                            .child(truncate_sql(&query.sql, 50))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("Running for {}", format_duration(query.running_time_ms())))
                    )
            )
            .child(
                Button::new(format!("cancel-{}", query.query_id))
                    .icon(IconName::X)
                    .size(ButtonSize::Small)
                    .style(ButtonStyle::Ghost)
                    .tooltip("Cancel query")
            )
    }
}

/// Truncate SQL for display
fn truncate_sql(sql: &str, max_len: usize) -> String {
    let cleaned = sql
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if cleaned.len() <= max_len {
        cleaned
    } else {
        format!("{}...", &cleaned[..max_len])
    }
}
```

### 11.5 Error Display Component

```rust
// src/ui/components/query_error.rs

use gpui::*;

use crate::models::query::QueryError;
use crate::ui::theme::Theme;

/// Displays query error with details
pub struct QueryErrorDisplay {
    error: QueryError,
    sql: String,
    expanded: bool,
}

impl QueryErrorDisplay {
    pub fn new(error: QueryError, sql: String) -> Self {
        Self {
            error,
            sql,
            expanded: false,
        }
    }
}

impl Render for QueryErrorDisplay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .p(px(16.0))
            .bg(theme.error.opacity(0.1))
            .border_1()
            .border_color(theme.error.opacity(0.3))
            .rounded(px(6.0))
            .child(
                // Error header
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        Icon::new(IconName::AlertCircle)
                            .size(px(20.0))
                            .color(theme.error)
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.error)
                                    .child(format!("ERROR: {}", self.error.code))
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text)
                                    .mt(px(4.0))
                                    .child(self.error.message.clone())
                            )
                    )
            )
            // Error position in SQL
            .when(self.error.position.is_some(), |el| {
                el.child(self.render_sql_with_error(theme))
            })
            // Detail
            .when(self.error.detail.is_some(), |el| {
                el.child(
                    div()
                        .mt(px(12.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text_muted)
                                .child("DETAIL")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text)
                                .mt(px(4.0))
                                .child(self.error.detail.clone().unwrap())
                        )
                )
            })
            // Hint
            .when(self.error.hint.is_some(), |el| {
                el.child(
                    div()
                        .mt(px(12.0))
                        .p(px(12.0))
                        .bg(theme.surface)
                        .rounded(px(4.0))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.0))
                                .child(
                                    Icon::new(IconName::Lightbulb)
                                        .size(px(16.0))
                                        .color(theme.warning)
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.text)
                                        .child(self.error.hint.clone().unwrap())
                                )
                        )
                )
            })
            // Context info (schema, table, column, constraint)
            .when(
                self.error.schema.is_some() ||
                self.error.table.is_some() ||
                self.error.column.is_some() ||
                self.error.constraint.is_some(),
                |el| {
                    el.child(self.render_context_info(theme))
                }
            )
    }
}

impl QueryErrorDisplay {
    fn render_sql_with_error(&self, theme: &Theme) -> impl IntoElement {
        let position = self.error.position.unwrap() as usize;

        // Find the line containing the error
        let (line_num, col_num) = self.error.get_line_column(&self.sql)
            .unwrap_or((1, position));

        // Get the line with the error
        let lines: Vec<&str> = self.sql.lines().collect();
        let error_line = lines.get(line_num.saturating_sub(1)).unwrap_or(&"");

        // Calculate visual position for marker
        let marker_col = col_num.saturating_sub(1).min(error_line.len());

        div()
            .mt(px(12.0))
            .p(px(12.0))
            .bg(theme.surface)
            .rounded(px(4.0))
            .overflow_x_auto()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .font_family("monospace")
                    .text_sm()
                    // Show context lines before error
                    .when(line_num > 1, |el| {
                        el.children(
                            lines.iter()
                                .take(line_num.saturating_sub(1))
                                .enumerate()
                                .map(|(i, line)| {
                                    self.render_line(i + 1, line, false, 0, theme)
                                })
                        )
                    })
                    // Error line
                    .child(self.render_line(line_num, error_line, true, marker_col, theme))
                    // Error marker
                    .child(
                        div()
                            .flex()
                            .child(
                                // Line number gutter space
                                div()
                                    .w(px(40.0))
                                    .flex_shrink_0()
                            )
                            .child(
                                div()
                                    .child(
                                        format!("{:>width$}^", "", width = marker_col)
                                    )
                                    .text_color(theme.error)
                            )
                    )
                    // Show context lines after error
                    .when(line_num < lines.len(), |el| {
                        el.children(
                            lines.iter()
                                .skip(line_num)
                                .take(2)
                                .enumerate()
                                .map(|(i, line)| {
                                    self.render_line(line_num + i + 1, line, false, 0, theme)
                                })
                        )
                    })
            )
            .child(
                div()
                    .mt(px(8.0))
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child(format!("Line {}, Column {}", line_num, col_num))
            )
    }

    fn render_line(&self, num: usize, content: &str, is_error: bool, _col: usize, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .when(is_error, |el| el.bg(theme.error.opacity(0.1)))
            .child(
                div()
                    .w(px(40.0))
                    .flex_shrink_0()
                    .text_right()
                    .pr(px(8.0))
                    .text_color(theme.text_muted)
                    .child(num.to_string())
            )
            .child(
                div()
                    .text_color(if is_error { theme.text } else { theme.text_muted })
                    .child(content.to_string())
            )
    }

    fn render_context_info(&self, theme: &Theme) -> impl IntoElement {
        div()
            .mt(px(12.0))
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .when(self.error.schema.is_some(), |el| {
                el.child(self.render_context_badge("Schema", self.error.schema.as_ref().unwrap(), theme))
            })
            .when(self.error.table.is_some(), |el| {
                el.child(self.render_context_badge("Table", self.error.table.as_ref().unwrap(), theme))
            })
            .when(self.error.column.is_some(), |el| {
                el.child(self.render_context_badge("Column", self.error.column.as_ref().unwrap(), theme))
            })
            .when(self.error.constraint.is_some(), |el| {
                el.child(self.render_context_badge("Constraint", self.error.constraint.as_ref().unwrap(), theme))
            })
    }

    fn render_context_badge(&self, label: &str, value: &str, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .bg(theme.surface)
            .rounded(px(4.0))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child(format!("{}:", label))
            )
            .child(
                div()
                    .text_xs()
                    .font_family("monospace")
                    .text_color(theme.text)
                    .child(value.to_string())
            )
    }
}
```

### 11.6 Query History Integration

```rust
// src/services/query_history.rs

use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::services::storage::StorageService;
use crate::error::Result;

/// Query history entry
#[derive(Clone, Debug)]
pub struct QueryHistoryEntry {
    pub id: i64,
    pub connection_id: Uuid,
    pub sql: String,
    pub executed_at: DateTime<Utc>,
    pub elapsed_ms: u64,
    pub rows_affected: Option<u64>,
    pub error: Option<String>,
}

impl QueryHistoryEntry {
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn preview(&self, max_len: usize) -> String {
        let cleaned = self.sql
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("--"))
            .collect::<Vec<_>>()
            .join(" ");

        if cleaned.len() <= max_len {
            cleaned
        } else {
            format!("{}...", &cleaned[..max_len])
        }
    }
}

/// Query history service
pub struct QueryHistoryService {
    storage: std::sync::Arc<StorageService>,
}

impl QueryHistoryService {
    pub fn new(storage: std::sync::Arc<StorageService>) -> Self {
        Self { storage }
    }

    /// Get recent history for connection
    pub async fn get_history(
        &self,
        connection_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<QueryHistoryEntry>> {
        self.storage.get_query_history(connection_id, limit, offset).await
    }

    /// Search history
    pub async fn search_history(
        &self,
        connection_id: Option<Uuid>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<QueryHistoryEntry>> {
        self.storage.search_query_history(connection_id, query, limit).await
    }

    /// Get history entry by ID
    pub async fn get_entry(&self, id: i64) -> Result<Option<QueryHistoryEntry>> {
        self.storage.get_query_history_entry(id).await
    }

    /// Delete history entry
    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        self.storage.delete_query_history_entry(id).await
    }

    /// Clear all history for connection
    pub async fn clear_history(&self, connection_id: Uuid) -> Result<()> {
        self.storage.clear_query_history(connection_id).await
    }

    /// Get history statistics
    pub async fn get_stats(&self, connection_id: Uuid) -> Result<QueryHistoryStats> {
        self.storage.get_query_history_stats(connection_id).await
    }
}

/// History statistics
#[derive(Clone, Debug)]
pub struct QueryHistoryStats {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: u64,
}
```

## Acceptance Criteria

1. **Single Query Execution**
   - Execute SELECT queries and return results with column metadata
   - Execute DML queries (INSERT/UPDATE/DELETE) and return affected row count
   - Execute DDL queries (CREATE/ALTER/DROP) successfully
   - Support parameterized queries with all PostgreSQL types

2. **Multiple Statement Execution**
   - Parse and split multiple statements correctly
   - Handle strings, dollar-quotes, and comments properly
   - Execute statements sequentially
   - Stop on error when configured
   - Handle nested dollar-quote tags correctly

3. **Streaming Results**
   - Stream large result sets in configurable batches
   - Use GPUI state updates for each batch
   - Support row limits for safety
   - Properly track progress during streaming

4. **Query Cancellation**
   - Cancel running queries via Postgres cancel protocol
   - Clean up resources on cancellation
   - Update UI to reflect cancelled state
   - Handle cancel during streaming

5. **Timeout Enforcement**
   - Apply statement_timeout before query execution
   - Handle timeout errors gracefully
   - Support per-query timeout configuration

6. **Error Handling**
   - Parse Postgres errors with position information
   - Include DETAIL and HINT when available
   - Map error codes correctly
   - Display error position in SQL
   - Extract schema/table/column/constraint context

7. **History Recording**
   - Record all executed queries to storage
   - Store timing, affected rows, and errors
   - Support history search and retrieval

8. **Type Handling**
   - Handle all standard PostgreSQL types
   - Convert arrays properly
   - Handle JSON/JSONB correctly
   - Support numeric with precision
   - Handle date/time types with proper formatting

## Testing Instructions

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_statements_simple() {
        let sql = "SELECT 1; SELECT 2; SELECT 3;";
        let statements = split_statements(sql).unwrap();
        assert_eq!(statements.len(), 3);
    }

    #[test]
    fn test_split_statements_with_strings() {
        let sql = "SELECT 'hello; world'; SELECT 'foo';";
        let statements = split_statements(sql).unwrap();
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_split_statements_dollar_quote() {
        let sql = r#"
            CREATE FUNCTION test() RETURNS void AS $$
            BEGIN
                SELECT 1;
            END;
            $$ LANGUAGE plpgsql;
            SELECT 2;
        "#;
        let statements = split_statements(sql).unwrap();
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_split_statements_nested_dollar_quote() {
        let sql = r#"
            CREATE FUNCTION outer() RETURNS void AS $outer$
            DECLARE
                code text := $inner$SELECT 1;$inner$;
            BEGIN
                EXECUTE code;
            END;
            $outer$ LANGUAGE plpgsql;
        "#;
        let statements = split_statements(sql).unwrap();
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn test_split_statements_comments() {
        let sql = r#"
            -- This is a comment with ; semicolon
            SELECT 1;
            /* Block comment
               with ; semicolon */
            SELECT 2;
        "#;
        let statements = split_statements(sql).unwrap();
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_detect_command_type() {
        assert_eq!(detect_command_type("SELECT * FROM users"), "SELECT");
        assert_eq!(detect_command_type("  INSERT INTO users VALUES (1)"), "INSERT");
        assert_eq!(detect_command_type("UPDATE users SET x = 1"), "UPDATE");
        assert_eq!(detect_command_type("DELETE FROM users"), "DELETE");
        assert_eq!(detect_command_type("WITH cte AS (SELECT 1) SELECT * FROM cte"), "WITH");
    }

    #[test]
    fn test_query_error_line_column() {
        let sql = "SELECT *\nFROM users\nWHERE invalid_column = 1";
        let error = QueryError {
            message: "column invalid_column does not exist".to_string(),
            position: Some(25), // Position of "invalid_column"
            ..Default::default()
        };

        let (line, col) = error.get_line_column(sql).unwrap();
        assert_eq!(line, 3);
        assert_eq!(col, 7);
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Null.to_display_string(), "NULL");
        assert_eq!(Value::Bool(true).to_display_string(), "true");
        assert_eq!(Value::Int(42).to_display_string(), "42");
        assert_eq!(Value::Text("hello".to_string()).to_display_string(), "hello");
        assert_eq!(
            Value::Array(vec![Value::Int(1), Value::Int(2)]).to_display_string(),
            "{1,2}"
        );
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_select() {
        let pool = create_test_pool().await;
        let service = QueryService::new(/* ... */);

        let result = service.execute_query(
            pool,
            Uuid::new_v4(),
            "SELECT 1 as num, 'hello' as text".to_string(),
            Vec::new(),
            QueryOptions::default(),
        ).unwrap();

        assert_eq!(result.status, QueryStatus::Success);
        assert_eq!(result.columns.as_ref().unwrap().len(), 2);
        assert_eq!(result.total_rows, Some(1));
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let pool = create_test_pool().await;
        let service = QueryService::new(/* ... */);

        let result = service.execute_query(
            pool,
            Uuid::new_v4(),
            "SELECT pg_sleep(10)".to_string(),
            Vec::new(),
            QueryOptions {
                statement_timeout_ms: Some(100),
                ..Default::default()
            },
        ).unwrap();

        assert_eq!(result.status, QueryStatus::Error);
        assert!(result.error.as_ref().unwrap().message.contains("timeout"));
    }

    #[tokio::test]
    async fn test_execute_multiple() {
        let pool = create_test_pool().await;
        let service = QueryService::new(/* ... */);

        let results = service.execute_multiple(
            pool,
            Uuid::new_v4(),
            "SELECT 1; SELECT 2; SELECT 3;".to_string(),
            QueryOptions::default(),
        ).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.status == QueryStatus::Success));
    }

    #[tokio::test]
    async fn test_cancel_query() {
        let pool = create_test_pool().await;
        let service = QueryService::new(/* ... */);

        // Start long-running query in background
        let query_id = Uuid::new_v4();
        let handle = tokio::spawn(async move {
            service.execute_query(/* pg_sleep(60) */)
        });

        // Wait a bit then cancel
        tokio::time::sleep(Duration::from_millis(100)).await;
        service.cancel_query(query_id).unwrap();

        let result = handle.await.unwrap();
        assert_eq!(result.unwrap().status, QueryStatus::Cancelled);
    }
}
```

## Dependencies

- tokio-postgres (Postgres driver with async support)
- futures (async streaming)
- chrono (date/time handling)
- hex (bytea encoding)
- uuid (query IDs)
- rust_decimal (numeric precision)
- serde_json (JSON value handling)
- parking_lot (synchronous locks)
- tracing (logging)

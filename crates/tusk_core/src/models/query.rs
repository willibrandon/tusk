//! Query execution models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Type of SQL query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    /// SELECT query returning rows
    Select,
    /// INSERT operation
    Insert,
    /// UPDATE operation
    Update,
    /// DELETE operation
    Delete,
    /// DDL, COPY, or other operations
    Other,
}

/// Column metadata from query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name
    pub name: String,
    /// PostgreSQL type OID
    pub type_oid: u32,
    /// Human-readable type name
    pub type_name: String,
}

/// Handle for tracking and cancelling a running query (FR-014, FR-015, FR-016).
pub struct QueryHandle {
    /// Unique query identifier
    id: Uuid,
    /// Associated connection
    connection_id: Uuid,
    /// The SQL being executed
    sql: String,
    /// Cancellation token for interrupting the query
    cancel_token: CancellationToken,
    /// Execution start time
    started_at: DateTime<Utc>,
}

impl QueryHandle {
    /// Create a new query handle.
    pub fn new(connection_id: Uuid, sql: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id,
            sql: sql.into(),
            cancel_token: CancellationToken::new(),
            started_at: Utc::now(),
        }
    }

    /// Get the unique query identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the associated connection ID.
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }

    /// Get the SQL being executed.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get when execution started.
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Get elapsed time since execution started.
    pub fn elapsed(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }

    /// Get elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> i64 {
        self.elapsed().num_milliseconds()
    }

    /// Request cancellation of the query.
    pub fn cancel(&self) {
        tracing::debug!(query_id = %self.id, "Cancellation requested");
        self.cancel_token.cancel();
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Wait for cancellation.
    pub async fn cancelled(&self) {
        self.cancel_token.cancelled().await
    }

    /// Get a clone of the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

impl std::fmt::Debug for QueryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryHandle")
            .field("id", &self.id)
            .field("connection_id", &self.connection_id)
            .field("sql", &self.sql)
            .field("started_at", &self.started_at)
            .field("is_cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Results from query execution.
pub struct QueryResult {
    /// The query handle ID
    pub query_id: Uuid,
    /// Column metadata
    pub columns: Vec<ColumnInfo>,
    /// Result rows
    pub rows: Vec<tokio_postgres::Row>,
    /// Rows affected (for INSERT/UPDATE/DELETE)
    pub rows_affected: Option<u64>,
    /// Time to execute in milliseconds
    pub execution_time_ms: u64,
    /// Type of query
    pub query_type: QueryType,
}

impl QueryResult {
    /// Get the number of rows returned.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Check if the result is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get column names.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }
}

impl std::fmt::Debug for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryResult")
            .field("query_id", &self.query_id)
            .field("columns", &self.columns)
            .field("row_count", &self.rows.len())
            .field("rows_affected", &self.rows_affected)
            .field("execution_time_ms", &self.execution_time_ms)
            .field("query_type", &self.query_type)
            .finish()
    }
}

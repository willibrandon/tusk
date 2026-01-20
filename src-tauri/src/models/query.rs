// Query models - Phase 7 (User Story 5)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Status of a query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryStatus {
    /// Query is currently running
    Running,
    /// Query completed successfully
    Completed,
    /// Query was cancelled by user
    Cancelled,
    /// Query timed out
    TimedOut,
    /// Query failed with an error
    Failed,
}

/// Column metadata from a query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column name
    pub name: String,
    /// PostgreSQL data type name
    pub data_type: String,
    /// OID of the PostgreSQL type
    pub type_oid: u32,
    /// Whether the column allows NULL values
    pub nullable: bool,
}

/// A single row of query results.
pub type Row = Vec<JsonValue>;

/// Result of executing a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// Unique query ID
    pub query_id: Uuid,
    /// Column definitions
    pub columns: Vec<Column>,
    /// Result rows
    pub rows: Vec<Row>,
    /// Total number of rows affected/returned
    pub row_count: u64,
    /// Execution time in milliseconds
    pub elapsed_ms: u64,
}

/// Information about a currently executing query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveQuery {
    /// Unique query ID
    pub query_id: Uuid,
    /// Connection pool ID
    pub connection_id: Uuid,
    /// SQL text (truncated to first 100 chars)
    pub sql: String,
    /// When the query started
    pub started_at: DateTime<Utc>,
    /// How long the query has been running (ms)
    pub elapsed_ms: u64,
}

/// Request to execute a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteQueryRequest {
    /// Connection pool ID to execute on
    pub connection_id: String,
    /// SQL to execute
    pub sql: String,
    /// Optional query ID for cancellation (auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
}

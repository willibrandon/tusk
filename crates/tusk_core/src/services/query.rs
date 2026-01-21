//! Query execution with cancellation support.
//!
//! Provides query execution with:
//! - Unique query identifiers for tracking (FR-014)
//! - Cancellation via tokio-util CancellationToken (FR-015)
//! - Query type detection for result handling

use crate::error::TuskError;
use crate::models::{ColumnInfo, QueryHandle, QueryResult, QueryType};
use crate::services::connection::PooledConnection;

use std::time::Instant;
use tokio::select;

/// Service for executing queries with cancellation support.
pub struct QueryService;

impl QueryService {
    /// Execute a query with cancellation support (FR-015, SC-004).
    ///
    /// # Arguments
    /// * `conn` - Pooled database connection
    /// * `sql` - SQL query to execute
    /// * `handle` - Query handle for tracking and cancellation
    ///
    /// # Returns
    /// Query results, or an error if the query failed or was cancelled.
    pub async fn execute(
        conn: &PooledConnection,
        sql: &str,
        handle: &QueryHandle,
    ) -> Result<QueryResult, TuskError> {
        Self::execute_with_params(conn, sql, &[], handle).await
    }

    /// Execute a parameterized query with cancellation support.
    ///
    /// # Arguments
    /// * `conn` - Pooled database connection
    /// * `sql` - SQL query with parameter placeholders ($1, $2, etc.)
    /// * `params` - Query parameters
    /// * `handle` - Query handle for tracking and cancellation
    pub async fn execute_with_params(
        conn: &PooledConnection,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        handle: &QueryHandle,
    ) -> Result<QueryResult, TuskError> {
        let start = Instant::now();
        let query_type = Self::detect_query_type(sql);

        tracing::debug!(
            query_id = %handle.id(),
            query_type = ?query_type,
            "Executing query"
        );

        // Execute with cancellation support
        let result = select! {
            // Query execution
            result = conn.query(sql, params) => {
                result
            }
            // Cancellation check (SC-004: propagation within 50ms)
            _ = handle.cancelled() => {
                tracing::debug!(query_id = %handle.id(), "Query cancelled");
                return Err(TuskError::query_cancelled(handle.id()));
            }
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Handle query completed before cancellation could propagate
        // Per spec: return results normally if query completed (FR race handling)
        let rows = result?;

        // Extract column information
        let columns = if rows.is_empty() {
            Vec::new()
        } else {
            rows.first()
                .map(|row| {
                    row.columns()
                        .iter()
                        .map(|col| ColumnInfo {
                            name: col.name().to_string(),
                            type_oid: col.type_().oid(),
                            type_name: col.type_().name().to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        // Determine rows affected (for non-SELECT queries)
        let rows_affected = match query_type {
            QueryType::Select => None,
            _ => Some(rows.len() as u64),
        };

        tracing::debug!(
            query_id = %handle.id(),
            execution_time_ms,
            row_count = rows.len(),
            "Query completed"
        );

        Ok(QueryResult {
            query_id: handle.id(),
            columns,
            rows,
            rows_affected,
            execution_time_ms,
            query_type,
        })
    }

    /// Detect the type of SQL query.
    pub fn detect_query_type(sql: &str) -> QueryType {
        let trimmed = sql.trim_start().to_uppercase();

        if trimmed.starts_with("SELECT") || trimmed.starts_with("WITH") {
            QueryType::Select
        } else if trimmed.starts_with("INSERT") {
            QueryType::Insert
        } else if trimmed.starts_with("UPDATE") {
            QueryType::Update
        } else if trimmed.starts_with("DELETE") {
            QueryType::Delete
        } else {
            QueryType::Other
        }
    }
}

//! Query execution with cancellation support.
//!
//! Provides query execution with:
//! - Unique query identifiers for tracking (FR-014)
//! - Cancellation via tokio-util CancellationToken (FR-015)
//! - Streaming results via mpsc channels (FR-011, FR-012)
//! - Query type detection for result handling

use crate::error::TuskError;
use crate::models::{ColumnInfo, QueryEvent, QueryHandle, QueryResult, QueryType};
use crate::services::connection::PooledConnection;

use futures_util::StreamExt;
use std::pin::pin;
use std::time::Instant;
use tokio::select;
use tokio::sync::mpsc;

/// Default batch size for streaming results (FR-012).
const DEFAULT_BATCH_SIZE: usize = 1000;

/// Progress update interval (rows) for large queries.
const PROGRESS_INTERVAL: usize = 10000;

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

    /// Execute a query with streaming results via channel (FR-011, FR-012, FR-014).
    ///
    /// Sends QueryEvent messages through the provided channel as results arrive.
    /// Events are sent in order: Columns, Rows (batches), then Complete or Error.
    ///
    /// # Arguments
    /// * `conn` - Pooled database connection
    /// * `sql` - SQL query to execute
    /// * `handle` - Query handle for tracking and cancellation
    /// * `tx` - Channel sender for QueryEvent stream
    ///
    /// # Event Ordering
    /// 1. `Columns` - Sent first with column metadata
    /// 2. `Rows` - Sent in batches of 1000 rows
    /// 3. `Progress` - Sent every 10,000 rows (optional)
    /// 4. `Complete` or `Error` - Exactly one, as final event
    pub async fn execute_streaming(
        conn: &PooledConnection,
        sql: &str,
        handle: &QueryHandle,
        tx: mpsc::Sender<QueryEvent>,
    ) -> Result<(), TuskError> {
        Self::execute_streaming_with_batch_size(conn, sql, handle, tx, DEFAULT_BATCH_SIZE).await
    }

    /// Execute a streaming query with custom batch size.
    pub async fn execute_streaming_with_batch_size(
        conn: &PooledConnection,
        sql: &str,
        handle: &QueryHandle,
        tx: mpsc::Sender<QueryEvent>,
        batch_size: usize,
    ) -> Result<(), TuskError> {
        let start = Instant::now();
        let query_type = Self::detect_query_type(sql);

        tracing::debug!(
            query_id = %handle.id(),
            query_type = ?query_type,
            batch_size,
            "Executing streaming query"
        );

        // Execute query and get row stream
        let row_stream = select! {
            result = conn.query_raw(sql, &[] as &[&(dyn tokio_postgres::types::ToSql + Sync)]) => {
                result
            }
            _ = handle.cancelled() => {
                tracing::debug!(query_id = %handle.id(), "Query cancelled before execution");
                let _ = tx.send(QueryEvent::error(TuskError::query_cancelled(handle.id()))).await;
                return Ok(());
            }
        };

        let row_stream = match row_stream {
            Ok(stream) => stream,
            Err(e) => {
                let error = TuskError::from(e);
                let _ = tx.send(QueryEvent::error(error)).await;
                // Error already sent through channel; return Ok since streaming is "complete"
                return Ok(());
            }
        };

        // Pin the row stream for use with StreamExt::next()
        let mut row_stream = pin!(row_stream);

        // Track if we've sent columns yet
        let mut columns_sent = false;
        let mut batch: Vec<tokio_postgres::Row> = Vec::with_capacity(batch_size);
        let mut total_rows: usize = 0;
        let mut last_progress_at: usize = 0;

        loop {
            // Check for cancellation before each batch
            if handle.is_cancelled() {
                tracing::debug!(
                    query_id = %handle.id(),
                    rows_received = total_rows,
                    "Query cancelled during streaming"
                );
                let _ = tx.send(QueryEvent::error(TuskError::query_cancelled(handle.id()))).await;
                return Ok(());
            }

            // Get next row with cancellation support
            let next_row = select! {
                row = row_stream.next() => row,
                _ = handle.cancelled() => {
                    tracing::debug!(
                        query_id = %handle.id(),
                        rows_received = total_rows,
                        "Query cancelled during streaming"
                    );
                    let _ = tx.send(QueryEvent::error(TuskError::query_cancelled(handle.id()))).await;
                    return Ok(());
                }
            };

            match next_row {
                Some(Ok(row)) => {
                    // Send column metadata on first row (FR-014)
                    if !columns_sent {
                        let columns: Vec<ColumnInfo> = row
                            .columns()
                            .iter()
                            .map(|col| ColumnInfo {
                                name: col.name().to_string(),
                                type_oid: col.type_().oid(),
                                type_name: col.type_().name().to_string(),
                            })
                            .collect();

                        if tx.send(QueryEvent::columns(columns)).await.is_err() {
                            // Receiver dropped, stop streaming
                            return Ok(());
                        }
                        columns_sent = true;
                    }

                    batch.push(row);
                    total_rows += 1;

                    // Send batch when full (FR-012)
                    if batch.len() >= batch_size {
                        let rows_to_send = std::mem::replace(
                            &mut batch,
                            Vec::with_capacity(batch_size),
                        );
                        if tx.send(QueryEvent::rows(rows_to_send, total_rows)).await.is_err() {
                            return Ok(());
                        }
                    }

                    // Send progress update for large queries
                    if total_rows - last_progress_at >= PROGRESS_INTERVAL {
                        last_progress_at = total_rows;
                        let _ = tx.send(QueryEvent::progress(total_rows)).await;
                    }
                }
                Some(Err(e)) => {
                    let error = TuskError::from(e);
                    tracing::warn!(
                        query_id = %handle.id(),
                        error = %error,
                        rows_received = total_rows,
                        "Query error during streaming"
                    );
                    let _ = tx.send(QueryEvent::error(error)).await;
                    // Error already sent through channel; return Ok since streaming is "complete"
                    return Ok(());
                }
                None => {
                    // Stream complete
                    break;
                }
            }
        }

        // Send any remaining rows in the final batch
        if !batch.is_empty() {
            let _ = tx.send(QueryEvent::rows(batch, total_rows)).await;
        }

        // If no rows were received, still send empty columns
        if !columns_sent {
            let _ = tx.send(QueryEvent::columns(Vec::new())).await;
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;
        let rows_affected = match query_type {
            QueryType::Select => None,
            _ => Some(total_rows as u64),
        };

        tracing::debug!(
            query_id = %handle.id(),
            execution_time_ms,
            total_rows,
            "Streaming query completed"
        );

        let _ = tx.send(QueryEvent::complete(total_rows, execution_time_ms, rows_affected)).await;

        Ok(())
    }
}

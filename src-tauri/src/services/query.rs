// Query service - Phase 7 (User Story 5)

use crate::error::{TuskError, TuskResult};
use crate::models::{ActiveQuery, Column, QueryResult, Row};
use crate::services::connection::ConnectionService;
use crate::state::{AppState, QueryHandle};
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::time::Instant;
use tauri::State;
use tokio::select;
use tokio_postgres::types::Type;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Query service for executing SQL and managing query lifecycle.
pub struct QueryService;

impl QueryService {
    /// Execute a SQL query with cancellation support.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `connection_id` - Connection pool ID
    /// * `sql` - SQL to execute
    /// * `query_id` - Optional query ID (auto-generated if not provided)
    ///
    /// # Returns
    ///
    /// Returns the query result with columns and rows.
    pub async fn execute(
        state: &State<'_, AppState>,
        connection_id: Uuid,
        sql: &str,
        query_id: Option<Uuid>,
    ) -> TuskResult<QueryResult> {
        let query_id = query_id.unwrap_or_else(Uuid::new_v4);
        let start = Instant::now();

        // Get the connection pool
        let pool = ConnectionService::get_pool(state, &connection_id).await?;

        // Create cancellation token
        let cancel_token = CancellationToken::new();

        // Register the query for cancellation
        let handle = QueryHandle {
            query_id,
            connection_id,
            sql: truncate_sql(sql, 100),
            started_at: start,
            cancel_token: cancel_token.clone(),
        };

        {
            let mut queries = state.active_queries.write().await;
            queries.insert(query_id, handle);
        }

        // Execute the query with cancellation support
        let result = Self::execute_with_cancellation(&pool, sql, query_id, cancel_token.clone()).await;

        // Remove from active queries
        {
            let mut queries = state.active_queries.write().await;
            queries.remove(&query_id);
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((columns, rows)) => {
                let row_count = rows.len() as u64;
                tracing::info!(
                    "Query {} completed: {} rows in {}ms",
                    query_id,
                    row_count,
                    elapsed_ms
                );
                Ok(QueryResult {
                    query_id,
                    columns,
                    rows,
                    row_count,
                    elapsed_ms,
                })
            }
            Err(e) => {
                tracing::warn!("Query {} failed after {}ms: {}", query_id, elapsed_ms, e);
                Err(e)
            }
        }
    }

    /// Execute the query with cancellation support.
    async fn execute_with_cancellation(
        pool: &deadpool_postgres::Pool,
        sql: &str,
        query_id: Uuid,
        cancel_token: CancellationToken,
    ) -> TuskResult<(Vec<Column>, Vec<Row>)> {
        // Get a client from the pool
        let client = pool.get().await.map_err(|e| {
            TuskError::connection_with_hint(
                format!("Failed to get connection: {}", e),
                "The connection may have been closed. Try reconnecting.",
            )
        })?;

        // Execute with cancellation support
        select! {
            result = client.query(sql, &[]) => {
                match result {
                    Ok(rows) => {
                        if rows.is_empty() {
                            return Ok((vec![], vec![]));
                        }

                        // Extract column metadata from the first row
                        let columns: Vec<Column> = rows[0]
                            .columns()
                            .iter()
                            .map(|col| Column {
                                name: col.name().to_string(),
                                data_type: col.type_().name().to_string(),
                                type_oid: col.type_().oid(),
                                nullable: true, // PostgreSQL doesn't expose this in simple query
                            })
                            .collect();

                        // Convert rows to JSON
                        let result_rows: Vec<Row> = rows
                            .iter()
                            .map(row_to_json)
                            .collect();

                        Ok((columns, result_rows))
                    }
                    Err(e) => Err(TuskError::from(e)),
                }
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Query {} was cancelled", query_id);
                Err(TuskError::QueryCancelled)
            }
        }
    }

    /// Cancel a running query.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `query_id` - The query ID to cancel
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if cancellation was triggered, error if query not found.
    pub async fn cancel(state: &State<'_, AppState>, query_id: &Uuid) -> TuskResult<()> {
        let queries = state.active_queries.read().await;

        if let Some(handle) = queries.get(query_id) {
            handle.cancel_token.cancel();
            tracing::info!("Cancellation requested for query: {}", query_id);
            Ok(())
        } else {
            Err(TuskError::Internal(format!(
                "Query not found or already completed: {}",
                query_id
            )))
        }
    }

    /// Get all currently executing queries.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `connection_id` - Optional filter by connection
    ///
    /// # Returns
    ///
    /// Returns a list of active queries.
    pub async fn get_active_queries(
        state: &State<'_, AppState>,
        connection_id: Option<Uuid>,
    ) -> Vec<ActiveQuery> {
        let queries = state.active_queries.read().await;
        let now = Utc::now();

        queries
            .values()
            .filter(|q| connection_id.map_or(true, |id| q.connection_id == id))
            .map(|handle| {
                ActiveQuery {
                    query_id: handle.query_id,
                    connection_id: handle.connection_id,
                    sql: handle.sql.clone(),
                    started_at: now - chrono::Duration::milliseconds(handle.started_at.elapsed().as_millis() as i64),
                    elapsed_ms: handle.started_at.elapsed().as_millis() as u64,
                }
            })
            .collect()
    }
}

/// Truncate SQL for logging/display.
fn truncate_sql(sql: &str, max_len: usize) -> String {
    if sql.len() <= max_len {
        sql.to_string()
    } else {
        format!("{}...", &sql[..max_len])
    }
}

/// Convert a PostgreSQL row to JSON values.
fn row_to_json(row: &tokio_postgres::Row) -> Row {
    row.columns()
        .iter()
        .enumerate()
        .map(|(i, col)| {
            // Handle NULL values
            if row.try_get::<_, Option<String>>(i).ok().flatten().is_none() {
                // Try various types to check for NULL
                if let Ok(None) = row.try_get::<_, Option<i64>>(i) {
                    return JsonValue::Null;
                }
            }

            // Convert based on PostgreSQL type
            match col.type_().clone() {
                // Boolean
                Type::BOOL => row
                    .try_get::<_, Option<bool>>(i)
                    .ok()
                    .flatten()
                    .map(JsonValue::Bool)
                    .unwrap_or(JsonValue::Null),

                // Integers
                Type::INT2 => row
                    .try_get::<_, Option<i16>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Number(v.into()))
                    .unwrap_or(JsonValue::Null),
                Type::INT4 => row
                    .try_get::<_, Option<i32>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Number(v.into()))
                    .unwrap_or(JsonValue::Null),
                Type::INT8 => row
                    .try_get::<_, Option<i64>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Number(v.into()))
                    .unwrap_or(JsonValue::Null),

                // Floating point
                Type::FLOAT4 => row
                    .try_get::<_, Option<f32>>(i)
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::Number::from_f64(v as f64))
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null),
                Type::FLOAT8 => row
                    .try_get::<_, Option<f64>>(i)
                    .ok()
                    .flatten()
                    .and_then(serde_json::Number::from_f64)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null),

                // UUID
                Type::UUID => row
                    .try_get::<_, Option<Uuid>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::String(v.to_string()))
                    .unwrap_or(JsonValue::Null),

                // JSON/JSONB
                Type::JSON | Type::JSONB => row
                    .try_get::<_, Option<JsonValue>>(i)
                    .ok()
                    .flatten()
                    .unwrap_or(JsonValue::Null),

                // Timestamps
                Type::TIMESTAMP | Type::TIMESTAMPTZ => row
                    .try_get::<_, Option<chrono::DateTime<Utc>>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::String(v.to_rfc3339()))
                    .unwrap_or_else(|| {
                        // Fallback to NaiveDateTime
                        row.try_get::<_, Option<chrono::NaiveDateTime>>(i)
                            .ok()
                            .flatten()
                            .map(|v| JsonValue::String(v.to_string()))
                            .unwrap_or(JsonValue::Null)
                    }),

                // Date/Time
                Type::DATE => row
                    .try_get::<_, Option<chrono::NaiveDate>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::String(v.to_string()))
                    .unwrap_or(JsonValue::Null),
                Type::TIME | Type::TIMETZ => row
                    .try_get::<_, Option<chrono::NaiveTime>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::String(v.to_string()))
                    .unwrap_or(JsonValue::Null),

                // Arrays (convert to JSON arrays)
                Type::INT4_ARRAY => row
                    .try_get::<_, Option<Vec<i32>>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Array(v.into_iter().map(|x| JsonValue::Number(x.into())).collect()))
                    .unwrap_or(JsonValue::Null),
                Type::TEXT_ARRAY => row
                    .try_get::<_, Option<Vec<String>>>(i)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Array(v.into_iter().map(JsonValue::String).collect()))
                    .unwrap_or(JsonValue::Null),

                // Default to string representation for all other types
                _ => row
                    .try_get::<_, Option<String>>(i)
                    .ok()
                    .flatten()
                    .map(JsonValue::String)
                    .unwrap_or(JsonValue::Null),
            }
        })
        .collect()
}

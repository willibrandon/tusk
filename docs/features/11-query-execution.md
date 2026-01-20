# Feature 11: Query Execution Engine

## Overview

The query execution engine is the core backend service that executes SQL queries against connected Postgres databases. It handles single and multiple statement execution, streaming results for large datasets, query cancellation, timeout enforcement, and proper error handling with position information.

## Goals

- Execute queries asynchronously with streaming support for large results
- Support query cancellation at any point during execution
- Enforce statement timeouts to prevent runaway queries
- Parse and split multiple statements correctly (respecting strings and dollar-quoting)
- Provide detailed error information including position in the query
- Track query execution for history

## Dependencies

- Feature 07: Connection Management (active connection pools)
- Feature 04: IPC Layer (command and event infrastructure)
- Feature 05: Local Storage (query history persistence)

## Technical Specification

### 11.1 Rust Backend - Query Service

```rust
// src-tauri/src/services/query.rs

use tokio_postgres::{Client, Row, Statement, types::Type};
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::services::connection::ConnectionManager;
use crate::services::storage::StorageService;
use crate::models::query::{
    QueryResult, QueryStatus, ColumnMeta, Value, RowBatch,
    QueryComplete, QueryError, QueryProgress,
};

/// Active query tracking for cancellation support
struct ActiveQuery {
    cancel_token: tokio_postgres::cancel::CancelToken,
    started_at: Instant,
    sql: String,
}

pub struct QueryService {
    connection_manager: Arc<ConnectionManager>,
    storage: Arc<StorageService>,
    active_queries: RwLock<HashMap<Uuid, ActiveQuery>>,
}

impl QueryService {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        storage: Arc<StorageService>,
    ) -> Self {
        Self {
            connection_manager,
            storage,
            active_queries: RwLock::new(HashMap::new()),
        }
    }

    /// Execute a single SQL query with streaming results
    pub async fn execute_query(
        &self,
        conn_id: Uuid,
        sql: String,
        params: Vec<Value>,
        options: QueryOptions,
        app: tauri::AppHandle,
    ) -> Result<QueryResult> {
        let query_id = Uuid::new_v4();
        let started_at = Instant::now();

        // Get connection from pool
        let pool = self.connection_manager.get_pool(&conn_id).await?;
        let client = pool.get().await.map_err(|e| Error::Connection(e.to_string()))?;

        // Store cancel token for this query
        let cancel_token = client.cancel_token();
        {
            let mut active = self.active_queries.write().await;
            active.insert(query_id, ActiveQuery {
                cancel_token,
                started_at,
                sql: sql.clone(),
            });
        }

        // Set statement timeout if configured
        if let Some(timeout_ms) = options.statement_timeout_ms {
            let timeout_sql = format!("SET statement_timeout = {}", timeout_ms);
            client.execute(&timeout_sql, &[]).await?;
        }

        // Convert params to postgres types
        let pg_params = self.convert_params(&params)?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            pg_params.iter().map(|p| p.as_ref()).collect();

        // Execute based on query type
        let result = self.execute_with_streaming(
            &client,
            &sql,
            &param_refs,
            query_id,
            options,
            &app,
        ).await;

        // Remove from active queries
        {
            let mut active = self.active_queries.write().await;
            active.remove(&query_id);
        }

        // Reset statement timeout
        if options.statement_timeout_ms.is_some() {
            let _ = client.execute("SET statement_timeout = 0", &[]).await;
        }

        let elapsed_ms = started_at.elapsed().as_millis() as u64;

        // Record in history
        self.record_history(conn_id, &sql, elapsed_ms, &result).await;

        match result {
            Ok(mut query_result) => {
                query_result.query_id = query_id.to_string();
                query_result.elapsed_ms = elapsed_ms;
                Ok(query_result)
            }
            Err(e) => {
                let error = self.parse_postgres_error(&e, &sql);
                Ok(QueryResult {
                    query_id: query_id.to_string(),
                    status: QueryStatus::Error,
                    command: String::new(),
                    columns: None,
                    rows: None,
                    total_rows: None,
                    truncated: None,
                    rows_affected: None,
                    plan: None,
                    elapsed_ms,
                    error: Some(error),
                })
            }
        }
    }

    /// Execute query with streaming for large result sets
    async fn execute_with_streaming(
        &self,
        client: &tokio_postgres::Client,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        query_id: Uuid,
        options: QueryOptions,
        app: &tauri::AppHandle,
    ) -> Result<QueryResult> {
        // Prepare statement to get column info
        let statement = client.prepare(sql).await?;
        let columns = self.extract_column_meta(&statement);

        // Determine command type
        let command = self.detect_command_type(sql);

        if command == "SELECT" || command == "TABLE" || command == "VALUES" {
            // For SELECT queries, stream results
            self.execute_select_streaming(
                client,
                &statement,
                params,
                query_id,
                columns,
                options,
                app,
            ).await
        } else {
            // For DML/DDL, execute and return affected rows
            let rows_affected = client.execute_raw(&statement, params.iter().copied()).await?;

            Ok(QueryResult {
                query_id: query_id.to_string(),
                status: QueryStatus::Success,
                command,
                columns: None,
                rows: None,
                total_rows: None,
                truncated: None,
                rows_affected: Some(rows_affected),
                plan: None,
                elapsed_ms: 0,
                error: None,
            })
        }
    }

    /// Stream SELECT results in batches
    async fn execute_select_streaming(
        &self,
        client: &tokio_postgres::Client,
        statement: &Statement,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        query_id: Uuid,
        columns: Vec<ColumnMeta>,
        options: QueryOptions,
        app: &tauri::AppHandle,
    ) -> Result<QueryResult> {
        let batch_size = options.batch_size.unwrap_or(1000);
        let row_limit = options.row_limit;

        // Use cursor for large results
        let row_stream = client.query_raw(statement, params.iter().copied()).await?;
        tokio::pin!(row_stream);

        let mut all_rows: Vec<Vec<Value>> = Vec::new();
        let mut batch: Vec<Vec<Value>> = Vec::with_capacity(batch_size);
        let mut batch_num = 0;
        let mut total_rows = 0u64;
        let mut truncated = false;

        use futures::StreamExt;

        while let Some(row_result) = row_stream.next().await {
            let row = row_result?;

            // Check row limit
            if let Some(limit) = row_limit {
                if total_rows >= limit {
                    truncated = true;
                    break;
                }
            }

            let values = self.row_to_values(&row, &columns)?;
            batch.push(values);
            total_rows += 1;

            // Emit batch when full
            if batch.len() >= batch_size {
                let batch_data = std::mem::replace(&mut batch, Vec::with_capacity(batch_size));

                // Emit via Tauri event for streaming
                app.emit_all("query:rows", RowBatch {
                    query_id,
                    rows: batch_data.clone(),
                    batch_num,
                })?;

                all_rows.extend(batch_data);
                batch_num += 1;
            }
        }

        // Emit final partial batch
        if !batch.is_empty() {
            app.emit_all("query:rows", RowBatch {
                query_id,
                rows: batch.clone(),
                batch_num,
            })?;
            all_rows.extend(batch);
        }

        // Emit completion event
        app.emit_all("query:complete", QueryComplete {
            query_id,
            total_rows,
            elapsed_ms: 0, // Will be set by caller
        })?;

        Ok(QueryResult {
            query_id: query_id.to_string(),
            status: QueryStatus::Success,
            command: "SELECT".to_string(),
            columns: Some(columns),
            rows: Some(all_rows),
            total_rows: Some(total_rows),
            truncated: Some(truncated),
            rows_affected: None,
            plan: None,
            elapsed_ms: 0,
            error: None,
        })
    }

    /// Cancel a running query
    pub async fn cancel_query(&self, query_id: Uuid) -> Result<()> {
        let active = self.active_queries.read().await;

        if let Some(query) = active.get(&query_id) {
            // Send cancel request to Postgres
            let cancel_token = query.cancel_token.clone();
            drop(active); // Release lock before async operation

            tokio::spawn(async move {
                let _ = cancel_token.cancel_query(tokio_postgres::NoTls).await;
            });

            Ok(())
        } else {
            Err(Error::QueryNotFound(query_id.to_string()))
        }
    }

    /// Execute multiple statements sequentially
    pub async fn execute_multiple(
        &self,
        conn_id: Uuid,
        sql: String,
        options: QueryOptions,
        app: tauri::AppHandle,
    ) -> Result<Vec<QueryResult>> {
        let statements = self.split_statements(&sql)?;
        let mut results = Vec::with_capacity(statements.len());

        for statement in statements {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }

            let result = self.execute_query(
                conn_id,
                trimmed.to_string(),
                Vec::new(),
                options.clone(),
                app.clone(),
            ).await?;

            let is_error = result.status == QueryStatus::Error;
            results.push(result);

            // Stop on error if configured
            if is_error && options.stop_on_error {
                break;
            }
        }

        Ok(results)
    }

    /// Split SQL into individual statements
    fn split_statements(&self, sql: &str) -> Result<Vec<String>> {
        let mut statements = Vec::new();
        let mut current = String::new();
        let mut chars = sql.chars().peekable();
        let mut in_string = false;
        let mut string_char = '"';
        let mut in_dollar_quote = false;
        let mut dollar_tag = String::new();

        while let Some(c) = chars.next() {
            current.push(c);

            match c {
                // Handle single quotes and double quotes
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

                // Handle dollar-quoted strings
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
                            for ch in potential_end.chars().skip(1) {
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

                                for ch in tag.chars() {
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

                // Handle line comments
                '-' if !in_string && !in_dollar_quote => {
                    if chars.peek() == Some(&'-') {
                        current.push(chars.next().unwrap());
                        while let Some(&next) = chars.peek() {
                            current.push(chars.next().unwrap());
                            if next == '\n' {
                                break;
                            }
                        }
                    }
                }

                // Handle block comments
                '/' if !in_string && !in_dollar_quote => {
                    if chars.peek() == Some(&'*') {
                        current.push(chars.next().unwrap());
                        let mut depth = 1;
                        while depth > 0 {
                            if let Some(next) = chars.next() {
                                current.push(next);
                                if next == '/' && chars.peek() == Some(&'*') {
                                    current.push(chars.next().unwrap());
                                    depth += 1;
                                } else if next == '*' && chars.peek() == Some(&'/') {
                                    current.push(chars.next().unwrap());
                                    depth -= 1;
                                }
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

    /// Extract column metadata from prepared statement
    fn extract_column_meta(&self, statement: &Statement) -> Vec<ColumnMeta> {
        statement
            .columns()
            .iter()
            .enumerate()
            .map(|(i, col)| ColumnMeta {
                name: col.name().to_string(),
                type_oid: col.type_().oid(),
                type_name: col.type_().name().to_string(),
                type_modifier: -1, // Not directly available
                table_oid: None, // Would need additional query
                column_ordinal: Some(i as i32),
            })
            .collect()
    }

    /// Convert Postgres row to array of Values
    fn row_to_values(&self, row: &Row, columns: &[ColumnMeta]) -> Result<Vec<Value>> {
        let mut values = Vec::with_capacity(columns.len());

        for (i, col) in columns.iter().enumerate() {
            let value = self.extract_value(row, i, &col.type_name)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Extract a typed value from a row column
    fn extract_value(&self, row: &Row, idx: usize, type_name: &str) -> Result<Value> {
        // Handle NULL
        if row.try_get::<_, Option<()>>(idx).is_ok() {
            // This is a hack - try to detect NULL
        }

        match type_name {
            "bool" => Ok(row.try_get::<_, Option<bool>>(idx)?
                .map(Value::Bool)
                .unwrap_or(Value::Null)),

            "int2" => Ok(row.try_get::<_, Option<i16>>(idx)?
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null)),

            "int4" => Ok(row.try_get::<_, Option<i32>>(idx)?
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null)),

            "int8" => Ok(row.try_get::<_, Option<i64>>(idx)?
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null)),

            "float4" => Ok(row.try_get::<_, Option<f32>>(idx)?
                .map(|v| Value::Float(v.into()))
                .unwrap_or(Value::Null)),

            "float8" => Ok(row.try_get::<_, Option<f64>>(idx)?
                .map(Value::Float)
                .unwrap_or(Value::Null)),

            "text" | "varchar" | "bpchar" | "name" => {
                Ok(row.try_get::<_, Option<String>>(idx)?
                    .map(Value::String)
                    .unwrap_or(Value::Null))
            }

            "json" | "jsonb" => {
                Ok(row.try_get::<_, Option<serde_json::Value>>(idx)?
                    .map(Value::Json)
                    .unwrap_or(Value::Null))
            }

            "bytea" => {
                Ok(row.try_get::<_, Option<Vec<u8>>>(idx)?
                    .map(|v| Value::Bytea { hex: hex::encode(&v) })
                    .unwrap_or(Value::Null))
            }

            "uuid" => {
                Ok(row.try_get::<_, Option<uuid::Uuid>>(idx)?
                    .map(|v| Value::String(v.to_string()))
                    .unwrap_or(Value::Null))
            }

            "timestamp" | "timestamptz" => {
                Ok(row.try_get::<_, Option<chrono::NaiveDateTime>>(idx)
                    .ok()
                    .flatten()
                    .map(|v| Value::String(v.to_string()))
                    .unwrap_or(Value::Null))
            }

            "date" => {
                Ok(row.try_get::<_, Option<chrono::NaiveDate>>(idx)
                    .ok()
                    .flatten()
                    .map(|v| Value::String(v.to_string()))
                    .unwrap_or(Value::Null))
            }

            "time" | "timetz" => {
                Ok(row.try_get::<_, Option<chrono::NaiveTime>>(idx)
                    .ok()
                    .flatten()
                    .map(|v| Value::String(v.to_string()))
                    .unwrap_or(Value::Null))
            }

            // Array types
            t if t.starts_with('_') => {
                // PostgreSQL array types start with underscore
                let inner_type = &t[1..];
                self.extract_array_value(row, idx, inner_type)
            }

            // Unknown types - convert to string representation
            _ => {
                // Try to get as string representation
                Ok(Value::Unknown {
                    text: format!("({})", type_name),
                })
            }
        }
    }

    /// Extract array value
    fn extract_array_value(&self, row: &Row, idx: usize, inner_type: &str) -> Result<Value> {
        match inner_type {
            "int4" => Ok(row.try_get::<_, Option<Vec<i32>>>(idx)?
                .map(|v| Value::Array(v.into_iter().map(|i| Value::Number(i.into())).collect()))
                .unwrap_or(Value::Null)),

            "text" | "varchar" => Ok(row.try_get::<_, Option<Vec<String>>>(idx)?
                .map(|v| Value::Array(v.into_iter().map(Value::String).collect()))
                .unwrap_or(Value::Null)),

            _ => Ok(Value::Unknown { text: "(array)".to_string() }),
        }
    }

    /// Detect command type from SQL
    fn detect_command_type(&self, sql: &str) -> String {
        let sql_upper = sql.trim().to_uppercase();

        for cmd in &["SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER",
                     "DROP", "TRUNCATE", "GRANT", "REVOKE", "VACUUM", "ANALYZE",
                     "EXPLAIN", "TABLE", "VALUES", "WITH"] {
            if sql_upper.starts_with(cmd) {
                return cmd.to_string();
            }
        }

        "UNKNOWN".to_string()
    }

    /// Parse Postgres error into structured format
    fn parse_postgres_error(&self, error: &tokio_postgres::Error, sql: &str) -> QueryError {
        let db_error = error.as_db_error();

        QueryError {
            message: error.to_string(),
            detail: db_error.and_then(|e| e.detail().map(String::from)),
            hint: db_error.and_then(|e| e.hint().map(String::from)),
            position: db_error.and_then(|e| {
                e.position().map(|p| match p {
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos as i32,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position as i32,
                })
            }),
            code: db_error
                .map(|e| e.code().code().to_string())
                .unwrap_or_default(),
        }
    }

    /// Convert frontend Values to Postgres params
    fn convert_params(&self, params: &[Value]) -> Result<Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>> {
        let mut pg_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();

        for param in params {
            let boxed: Box<dyn tokio_postgres::types::ToSql + Sync + Send> = match param {
                Value::Null => Box::new(None::<String>),
                Value::Bool(b) => Box::new(*b),
                Value::Number(n) => Box::new(*n as i64),
                Value::Float(f) => Box::new(*f),
                Value::String(s) => Box::new(s.clone()),
                Value::Json(j) => Box::new(j.clone()),
                _ => Box::new(None::<String>),
            };
            pg_params.push(boxed);
        }

        Ok(pg_params)
    }

    /// Record query in history
    async fn record_history(
        &self,
        conn_id: Uuid,
        sql: &str,
        elapsed_ms: u64,
        result: &Result<QueryResult>,
    ) {
        let (rows_affected, error) = match result {
            Ok(r) => (r.rows_affected.or(r.total_rows.map(|t| t as u64)), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let _ = self.storage.record_query_history(
            conn_id,
            sql,
            elapsed_ms,
            rows_affected,
            error,
        ).await;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryOptions {
    pub statement_timeout_ms: Option<u32>,
    pub row_limit: Option<u64>,
    pub batch_size: Option<usize>,
    pub stop_on_error: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            statement_timeout_ms: None,
            row_limit: Some(10000),
            batch_size: Some(1000),
            stop_on_error: true,
        }
    }
}
```

### 11.2 Query Models

```rust
// src-tauri/src/models/query.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryStatus {
    Success,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: String,
    pub status: QueryStatus,
    pub command: String,

    // For SELECT queries
    pub columns: Option<Vec<ColumnMeta>>,
    pub rows: Option<Vec<Vec<Value>>>,
    pub total_rows: Option<u64>,
    pub truncated: Option<bool>,

    // For DML queries
    pub rows_affected: Option<u64>,

    // For EXPLAIN
    pub plan: Option<QueryPlan>,

    // Timing
    pub elapsed_ms: u64,

    // Errors
    pub error: Option<QueryError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub type_oid: u32,
    pub type_name: String,
    pub type_modifier: i32,
    pub table_oid: Option<u32>,
    pub column_ordinal: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(i64),
    Float(f64),
    String(String),
    Json(serde_json::Value),
    Array(Vec<Value>),
    Bytea { hex: String },
    Interval { iso: String },
    Point { x: f64, y: f64 },
    Unknown { text: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryError {
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<i32>,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RowBatch {
    pub query_id: Uuid,
    pub rows: Vec<Vec<Value>>,
    pub batch_num: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryComplete {
    pub query_id: Uuid,
    pub total_rows: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlan {
    pub raw: String,
    pub format: PlanFormat,
    pub root: Option<PlanNode>,
    pub planning_time_ms: f64,
    pub execution_time_ms: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanFormat {
    Text,
    Json,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanNode {
    pub node_type: String,
    pub relation_name: Option<String>,
    pub alias: Option<String>,
    pub index_name: Option<String>,
    pub startup_cost: f64,
    pub total_cost: f64,
    pub plan_rows: u64,
    pub plan_width: u32,
    pub actual_startup_time: Option<f64>,
    pub actual_total_time: Option<f64>,
    pub actual_rows: Option<u64>,
    pub actual_loops: Option<u64>,
    pub filter: Option<String>,
    pub index_cond: Option<String>,
    pub children: Vec<PlanNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryProgress {
    pub query_id: Uuid,
    pub rows_processed: u64,
    pub bytes_processed: u64,
}
```

### 11.3 IPC Commands

```rust
// src-tauri/src/commands/query.rs

use tauri::State;
use uuid::Uuid;

use crate::error::Result;
use crate::models::query::{QueryResult, Value};
use crate::services::query::{QueryService, QueryOptions};
use crate::state::AppState;

#[tauri::command]
pub async fn execute_query(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    conn_id: String,
    sql: String,
    params: Option<Vec<Value>>,
    options: Option<QueryOptions>,
) -> Result<QueryResult> {
    let conn_uuid = Uuid::parse_str(&conn_id)?;
    let opts = options.unwrap_or_default();
    let parameters = params.unwrap_or_default();

    state
        .query_service
        .execute_query(conn_uuid, sql, parameters, opts, app)
        .await
}

#[tauri::command]
pub async fn execute_query_multiple(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    conn_id: String,
    sql: String,
    options: Option<QueryOptions>,
) -> Result<Vec<QueryResult>> {
    let conn_uuid = Uuid::parse_str(&conn_id)?;
    let opts = options.unwrap_or_default();

    state
        .query_service
        .execute_multiple(conn_uuid, sql, opts, app)
        .await
}

#[tauri::command]
pub async fn cancel_query(
    state: State<'_, AppState>,
    query_id: String,
) -> Result<()> {
    let query_uuid = Uuid::parse_str(&query_id)?;
    state.query_service.cancel_query(query_uuid).await
}

#[tauri::command]
pub async fn explain_query(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    conn_id: String,
    sql: String,
    analyze: bool,
    buffers: bool,
    verbose: bool,
    costs: bool,
    timing: bool,
    format: String,
) -> Result<QueryResult> {
    let conn_uuid = Uuid::parse_str(&conn_id)?;

    let mut explain_parts = vec!["EXPLAIN"];
    let mut options = Vec::new();

    if analyze { options.push("ANALYZE"); }
    if buffers { options.push("BUFFERS"); }
    if verbose { options.push("VERBOSE"); }
    if !costs { options.push("COSTS OFF"); }
    if !timing && analyze { options.push("TIMING OFF"); }
    options.push(&format!("FORMAT {}", format.to_uppercase()));

    let explain_sql = if options.is_empty() {
        format!("EXPLAIN {}", sql)
    } else {
        format!("EXPLAIN ({}) {}", options.join(", "), sql)
    };

    state
        .query_service
        .execute_query(conn_uuid, explain_sql, Vec::new(), QueryOptions::default(), app)
        .await
}
```

### 11.4 Frontend Query Service

```typescript
// src/lib/services/query.ts

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface QueryResult {
	query_id: string;
	status: 'success' | 'error' | 'cancelled';
	command: string;
	columns?: ColumnMeta[];
	rows?: Value[][];
	total_rows?: number;
	truncated?: boolean;
	rows_affected?: number;
	plan?: QueryPlan;
	elapsed_ms: number;
	error?: QueryError;
}

export interface ColumnMeta {
	name: string;
	type_oid: number;
	type_name: string;
	type_modifier: number;
	table_oid?: number;
	column_ordinal?: number;
}

export type Value =
	| null
	| boolean
	| number
	| string
	| object
	| { hex: string } // bytea
	| { iso: string } // interval
	| { x: number; y: number } // point
	| { text: string }; // unknown

export interface QueryError {
	message: string;
	detail?: string;
	hint?: string;
	position?: number;
	code: string;
}

export interface QueryOptions {
	statement_timeout_ms?: number;
	row_limit?: number;
	batch_size?: number;
	stop_on_error?: boolean;
}

export interface RowBatch {
	query_id: string;
	rows: Value[][];
	batch_num: number;
}

export interface QueryComplete {
	query_id: string;
	total_rows: number;
	elapsed_ms: number;
}

export interface QueryPlan {
	raw: string;
	format: 'text' | 'json';
	root?: PlanNode;
	planning_time_ms: number;
	execution_time_ms?: number;
}

export interface PlanNode {
	node_type: string;
	relation_name?: string;
	alias?: string;
	index_name?: string;
	startup_cost: number;
	total_cost: number;
	plan_rows: number;
	plan_width: number;
	actual_startup_time?: number;
	actual_total_time?: number;
	actual_rows?: number;
	actual_loops?: number;
	filter?: string;
	index_cond?: string;
	children: PlanNode[];
}

class QueryService {
	private rowListeners: Map<string, UnlistenFn> = new Map();
	private completeListeners: Map<string, UnlistenFn> = new Map();

	async executeQuery(
		connId: string,
		sql: string,
		params?: Value[],
		options?: QueryOptions
	): Promise<QueryResult> {
		return invoke<QueryResult>('execute_query', {
			connId,
			sql,
			params,
			options
		});
	}

	async executeMultiple(
		connId: string,
		sql: string,
		options?: QueryOptions
	): Promise<QueryResult[]> {
		return invoke<QueryResult[]>('execute_query_multiple', {
			connId,
			sql,
			options
		});
	}

	async cancelQuery(queryId: string): Promise<void> {
		return invoke('cancel_query', { queryId });
	}

	async explainQuery(
		connId: string,
		sql: string,
		options: {
			analyze?: boolean;
			buffers?: boolean;
			verbose?: boolean;
			costs?: boolean;
			timing?: boolean;
			format?: 'text' | 'json';
		} = {}
	): Promise<QueryResult> {
		return invoke<QueryResult>('explain_query', {
			connId,
			sql,
			analyze: options.analyze ?? false,
			buffers: options.buffers ?? false,
			verbose: options.verbose ?? false,
			costs: options.costs ?? true,
			timing: options.timing ?? true,
			format: options.format ?? 'json'
		});
	}

	async subscribeToResults(
		queryId: string,
		onRows: (batch: RowBatch) => void,
		onComplete: (info: QueryComplete) => void
	): Promise<() => void> {
		const rowUnlisten = await listen<RowBatch>('query:rows', (event) => {
			if (event.payload.query_id === queryId) {
				onRows(event.payload);
			}
		});

		const completeUnlisten = await listen<QueryComplete>('query:complete', (event) => {
			if (event.payload.query_id === queryId) {
				onComplete(event.payload);
				// Auto-cleanup
				rowUnlisten();
				completeUnlisten();
			}
		});

		this.rowListeners.set(queryId, rowUnlisten);
		this.completeListeners.set(queryId, completeUnlisten);

		return () => {
			rowUnlisten();
			completeUnlisten();
			this.rowListeners.delete(queryId);
			this.completeListeners.delete(queryId);
		};
	}
}

export const queryService = new QueryService();
```

### 11.5 Query Execution Store

```typescript
// src/lib/stores/queryExecution.svelte.ts

import {
	queryService,
	type QueryResult,
	type QueryOptions,
	type RowBatch,
	type QueryComplete
} from '$lib/services/query';

interface ExecutingQuery {
	id: string;
	sql: string;
	connId: string;
	startedAt: Date;
	rows: any[][];
	columns: any[];
	status: 'running' | 'success' | 'error' | 'cancelled';
	error?: any;
	totalRows?: number;
	elapsedMs?: number;
}

class QueryExecutionStore {
	executingQueries = $state<Map<string, ExecutingQuery>>(new Map());

	async execute(connId: string, sql: string, options?: QueryOptions): Promise<QueryResult> {
		const tempId = crypto.randomUUID();

		// Track as executing
		this.executingQueries.set(tempId, {
			id: tempId,
			sql,
			connId,
			startedAt: new Date(),
			rows: [],
			columns: [],
			status: 'running'
		});

		try {
			// Subscribe to streaming results first
			let resolveComplete: (result: QueryResult) => void;
			const completePromise = new Promise<QueryResult>((resolve) => {
				resolveComplete = resolve;
			});

			// Execute query
			const result = await queryService.executeQuery(connId, sql, undefined, options);

			// Update tracking
			const query = this.executingQueries.get(tempId);
			if (query) {
				query.id = result.query_id;
				query.status = result.status;
				query.columns = result.columns ?? [];
				query.rows = result.rows ?? [];
				query.totalRows = result.total_rows;
				query.elapsedMs = result.elapsed_ms;
				query.error = result.error;

				// Re-key with real query ID
				this.executingQueries.delete(tempId);
				this.executingQueries.set(result.query_id, query);
			}

			return result;
		} catch (error) {
			const query = this.executingQueries.get(tempId);
			if (query) {
				query.status = 'error';
				query.error = error;
			}
			throw error;
		}
	}

	async executeMultiple(
		connId: string,
		sql: string,
		options?: QueryOptions
	): Promise<QueryResult[]> {
		return queryService.executeMultiple(connId, sql, options);
	}

	async cancel(queryId: string): Promise<void> {
		await queryService.cancelQuery(queryId);

		const query = this.executingQueries.get(queryId);
		if (query) {
			query.status = 'cancelled';
		}
	}

	getExecutingQuery(queryId: string): ExecutingQuery | undefined {
		return this.executingQueries.get(queryId);
	}

	clearCompleted(): void {
		for (const [id, query] of this.executingQueries) {
			if (query.status !== 'running') {
				this.executingQueries.delete(id);
			}
		}
	}
}

export const queryExecutionStore = new QueryExecutionStore();
```

## Acceptance Criteria

1. **Single Query Execution**
   - Execute SELECT queries and return results with column metadata
   - Execute DML queries (INSERT/UPDATE/DELETE) and return affected row count
   - Execute DDL queries (CREATE/ALTER/DROP) successfully

2. **Multiple Statement Execution**
   - Parse and split multiple statements correctly
   - Handle strings, dollar-quotes, and comments
   - Execute statements sequentially
   - Stop on error when configured

3. **Streaming Results**
   - Stream large result sets in configurable batches
   - Emit Tauri events for each batch
   - Support virtual scrolling on frontend

4. **Query Cancellation**
   - Cancel running queries via Postgres cancel protocol
   - Clean up resources on cancellation
   - Update UI to reflect cancelled state

5. **Timeout Enforcement**
   - Apply statement_timeout before query execution
   - Reset timeout after query completes
   - Handle timeout errors gracefully

6. **Error Handling**
   - Parse Postgres errors with position information
   - Include DETAIL and HINT when available
   - Map error codes correctly

7. **History Recording**
   - Record all executed queries in SQLite
   - Store timing, affected rows, and errors

## MCP Testing Instructions

### Using Tauri MCP Server

```typescript
// Test query execution
const result = await mcp.ipc_execute_command({
	command: 'execute_query',
	args: {
		conn_id: connectionId,
		sql: 'SELECT * FROM users LIMIT 10',
		options: { row_limit: 10 }
	}
});

// Verify result structure
assert(result.status === 'success');
assert(result.columns.length > 0);
assert(result.rows.length <= 10);

// Test query cancellation
const longQueryResult = mcp.ipc_execute_command({
	command: 'execute_query',
	args: {
		conn_id: connectionId,
		sql: 'SELECT pg_sleep(60)'
	}
});

// Cancel after short delay
await sleep(100);
await mcp.ipc_execute_command({
	command: 'cancel_query',
	args: { query_id: longQueryResult.query_id }
});

// Test multiple statements
const multiResult = await mcp.ipc_execute_command({
	command: 'execute_query_multiple',
	args: {
		conn_id: connectionId,
		sql: `
      SELECT 1 as a;
      SELECT 2 as b;
      SELECT 3 as c;
    `
	}
});

assert(multiResult.length === 3);
```

## Dependencies

- tokio-postgres (Postgres driver)
- futures (async streaming)
- chrono (date/time handling)
- hex (bytea encoding)
- uuid (query IDs)

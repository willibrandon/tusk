// Query commands - Phase 7 (User Story 5)

use crate::error::TuskError;
use crate::models::{ActiveQuery, QueryResult};
use crate::services::query::QueryService;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

/// Execute a SQL query.
#[tauri::command]
pub async fn execute_query(
    state: State<'_, AppState>,
    connection_id: String,
    sql: String,
    query_id: Option<String>,
) -> Result<QueryResult, TuskError> {
    // Parse connection ID
    let conn_uuid = Uuid::parse_str(&connection_id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    // Parse optional query ID
    let query_uuid = query_id
        .map(|id| {
            Uuid::parse_str(&id)
                .map_err(|e| TuskError::validation(format!("Invalid query ID: {}", e)))
        })
        .transpose()?;

    QueryService::execute(&state, conn_uuid, &sql, query_uuid).await
}

/// Cancel a running query.
#[tauri::command]
pub async fn cancel_query(
    state: State<'_, AppState>,
    query_id: String,
) -> Result<(), TuskError> {
    let uuid = Uuid::parse_str(&query_id)
        .map_err(|e| TuskError::validation(format!("Invalid query ID: {}", e)))?;

    QueryService::cancel(&state, &uuid).await
}

/// Get all currently executing queries.
#[tauri::command]
pub async fn get_running_queries(
    state: State<'_, AppState>,
    connection_id: Option<String>,
) -> Result<Vec<ActiveQuery>, TuskError> {
    // Parse optional connection ID
    let conn_uuid = connection_id
        .map(|id| {
            Uuid::parse_str(&id)
                .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))
        })
        .transpose()?;

    Ok(QueryService::get_active_queries(&state, conn_uuid).await)
}

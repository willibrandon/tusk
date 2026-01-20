// Connection commands - Phase 6 (User Story 4)

use crate::error::TuskError;
use crate::models::{ActiveConnection, ConnectionConfig, ConnectionTestResult};
use crate::services::connection::ConnectionService;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

/// Test a connection configuration without saving.
#[tauri::command]
pub async fn test_connection(
    config: ConnectionConfig,
    password: String,
) -> ConnectionTestResult {
    ConnectionService::test_connection(&config, &password).await
}

/// Establish a connection pool for a saved configuration.
#[tauri::command]
pub async fn connect(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, TuskError> {
    // Parse the connection ID
    let uuid = Uuid::parse_str(&id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    // Get the connection configuration from storage
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    let config = storage
        .get_connection(&uuid)?
        .ok_or_else(|| TuskError::storage_with_hint(
            format!("Connection not found: {}", id),
            "The connection configuration may have been deleted",
        ))?;

    // Get the password from keychain
    let password = state
        .credentials
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Credential service not initialized"))?
        .get_password(&uuid)?
        .ok_or_else(|| TuskError::credential_with_hint(
            "Password not found in keychain",
            "The password may not have been saved. Edit the connection and re-enter the password.",
        ))?;

    // Connect using the service
    let pool_id = ConnectionService::connect(&state, &config, &password).await?;

    Ok(pool_id.to_string())
}

/// Close a connection pool and release resources.
#[tauri::command]
pub async fn disconnect(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), TuskError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    ConnectionService::disconnect(&state, &uuid).await
}

/// List all currently active connection pools.
#[tauri::command]
pub async fn get_active_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ActiveConnection>, TuskError> {
    Ok(ConnectionService::get_active_connections(&state).await)
}

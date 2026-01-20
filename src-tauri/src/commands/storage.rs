use crate::error::TuskError;
use crate::models::{ConnectionConfig, DatabaseHealth};
use crate::state::AppState;
use chrono::Utc;
use serde_json::Value as JsonValue;
use tauri::State;
use uuid::Uuid;

/// List all saved connection configurations.
#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>) -> Result<Vec<ConnectionConfig>, TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    storage.list_connections()
}

/// Get a single connection configuration by ID.
#[tauri::command]
pub async fn get_connection(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ConnectionConfig>, TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    let uuid = Uuid::parse_str(&id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    storage.get_connection(&uuid)
}

/// Save a connection configuration (create or update).
/// If password is provided, it will be stored in the OS keychain.
#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    mut config: ConnectionConfig,
    password: Option<String>,
) -> Result<ConnectionConfig, TuskError> {
    // Validate the configuration
    config.validate()?;

    // Update timestamp
    config.updated_at = Utc::now();

    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    // Save to database
    storage.save_connection(&config)?;

    // Store password in keychain if provided (US6 integration)
    if let Some(pwd) = password {
        if let Some(credentials) = state.credentials.as_ref() {
            credentials.store_password(&config.id, &pwd)?;
        }
    }

    tracing::info!("Connection saved: {} ({})", config.name, config.id);
    Ok(config)
}

/// Delete a connection configuration.
#[tauri::command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    let uuid = Uuid::parse_str(&id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    // Delete from database
    storage.delete_connection(&uuid)?;

    // Delete password from keychain (US6 integration, non-fatal)
    if let Some(credentials) = state.credentials.as_ref() {
        if let Err(e) = credentials.delete_password(&uuid) {
            tracing::warn!("Failed to delete password from keychain: {}", e);
        }
    }

    tracing::info!("Connection deleted: {}", id);
    Ok(())
}

/// Get a stored user preference.
#[tauri::command]
pub async fn get_preference(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<JsonValue>, TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    storage.get_preference(&key)
}

/// Set a user preference.
#[tauri::command]
pub async fn set_preference(
    state: State<'_, AppState>,
    key: String,
    value: JsonValue,
) -> Result<(), TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    storage.set_preference(&key, &value)
}

/// Check local SQLite database integrity.
#[tauri::command]
pub async fn check_database_health(state: State<'_, AppState>) -> Result<DatabaseHealth, TuskError> {
    let storage = state
        .storage
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Storage service not initialized"))?;

    storage.check_integrity()
}

/// Check if OS keychain is available.
#[tauri::command]
pub fn check_keychain_available() -> bool {
    // Try to create a test entry - if it succeeds, keychain is available
    // We use a predictable test key that we immediately delete
    let test_service = "tusk-keychain-test";
    let test_user = "availability-check";

    keyring::Entry::new(test_service, test_user).is_ok()
}

/// Check if a password is stored for a connection.
#[tauri::command]
pub async fn has_stored_password(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<bool, TuskError> {
    let uuid = Uuid::parse_str(&connection_id)
        .map_err(|e| TuskError::validation(format!("Invalid connection ID: {}", e)))?;

    let has_pwd = state
        .credentials
        .as_ref()
        .and_then(|creds| creds.has_password(&uuid).ok())
        .unwrap_or(false);

    Ok(has_pwd)
}

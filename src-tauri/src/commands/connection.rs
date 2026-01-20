// Connection commands - Phase 6 (User Story 4)

use crate::error::TuskError;
use crate::models::{ActiveConnection, ConnectionConfig, ConnectionTestResult, SshAuthMethod};
use crate::services::connection::ConnectionService;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

/// Test a connection configuration without saving.
///
/// For SSH tunnel connections, the appropriate SSH credentials should be provided:
/// - For password auth: pass the SSH password in `ssh_password`
/// - For key file auth with encrypted key: pass the passphrase in `ssh_key_passphrase`
/// - For agent auth: no SSH credentials needed
#[tauri::command]
pub async fn test_connection(
    config: ConnectionConfig,
    password: String,
    ssh_password: Option<String>,
    ssh_key_passphrase: Option<String>,
) -> ConnectionTestResult {
    ConnectionService::test_connection(
        &config,
        &password,
        ssh_password.as_deref(),
        ssh_key_passphrase.as_deref(),
    )
    .await
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

    // Get the credential service
    let credentials = state
        .credentials
        .as_ref()
        .ok_or_else(|| TuskError::initialization("Credential service not initialized"))?;

    // Get the database password from keychain
    let password = credentials
        .get_password(&uuid)?
        .ok_or_else(|| TuskError::credential_with_hint(
            "Password not found in keychain",
            "The password may not have been saved. Edit the connection and re-enter the password.",
        ))?;

    // Get SSH credentials if SSH tunnel is configured
    let (ssh_password, ssh_key_passphrase) = if let Some(ref tunnel) = config.ssh_tunnel {
        match &tunnel.auth_method {
            SshAuthMethod::Password => {
                let ssh_pass = credentials.get_ssh_password(&uuid)?;
                (ssh_pass, None)
            }
            SshAuthMethod::KeyFile { .. } => {
                let passphrase = credentials.get_ssh_key_passphrase(&uuid)?;
                (None, passphrase)
            }
            SshAuthMethod::Agent => (None, None),
        }
    } else {
        (None, None)
    };

    // Connect using the service
    let pool_id = ConnectionService::connect(
        &state,
        &config,
        &password,
        ssh_password.as_deref(),
        ssh_key_passphrase.as_deref(),
    )
    .await?;

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

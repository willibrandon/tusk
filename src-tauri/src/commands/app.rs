use crate::models::AppInfo;
use crate::state::AppState;
use tauri::State;

/// Health check command that verifies the backend is operational.
/// This is an infallible command that returns basic application metadata.
///
/// # Returns
///
/// Returns `AppInfo` containing application name, version, Tauri version, and platform.
#[tauri::command]
pub fn health_check() -> AppInfo {
    tracing::debug!("health_check called");
    AppInfo::current()
}

/// Get application information.
/// Returns the same data as health_check but with a more descriptive name for frontend use.
///
/// # Returns
///
/// Returns `AppInfo` containing application name, version, Tauri version, and platform.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    tracing::debug!("get_app_info called");
    AppInfo::current()
}

/// Get the path to the application log directory.
/// Users can use this path to access log files for troubleshooting.
///
/// # Returns
///
/// Returns the absolute path to the log directory as a string.
#[tauri::command]
pub fn get_log_directory(state: State<'_, AppState>) -> String {
    let log_dir = state.log_dir.to_string_lossy().to_string();
    tracing::debug!("get_log_directory called, returning: {}", log_dir);
    log_dir
}

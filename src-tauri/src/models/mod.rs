use serde::Serialize;

/// Application information returned by the get_app_info command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Tauri runtime version
    pub tauri_version: String,
    /// Current platform (macos, windows, linux)
    pub platform: String,
}

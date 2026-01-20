use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application information returned by the health_check command.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl AppInfo {
    /// Create AppInfo from the current environment
    pub fn current() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            tauri_version: tauri::VERSION.to_string(),
            platform: std::env::consts::OS.to_string(),
        }
    }
}

/// SQLite database health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    /// Whether database passed integrity check
    pub is_healthy: bool,
    /// List of integrity errors (empty if healthy)
    pub errors: Vec<String>,
    /// Path to backup if repair failed and database was reset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<PathBuf>,
}

impl DatabaseHealth {
    /// Create a healthy status
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            errors: Vec::new(),
            backup_path: None,
        }
    }

    /// Create an unhealthy status with errors
    pub fn unhealthy(errors: Vec<String>) -> Self {
        Self {
            is_healthy: false,
            errors,
            backup_path: None,
        }
    }

    /// Create a status indicating the database was repaired via backup
    pub fn repaired_via_backup(backup_path: PathBuf) -> Self {
        Self {
            is_healthy: true,
            errors: Vec::new(),
            backup_path: Some(backup_path),
        }
    }
}

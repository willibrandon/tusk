use crate::error::{TuskError, TuskResult};
use std::path::PathBuf;
use tauri::Manager;

/// Application directory paths resolved from Tauri's path resolver.
/// These paths are platform-appropriate:
/// - macOS: ~/Library/Application Support/com.tusk
/// - Windows: C:\Users\<user>\AppData\Roaming\com.tusk
/// - Linux: ~/.config/com.tusk (or XDG_CONFIG_HOME)
#[derive(Debug, Clone)]
pub struct AppDirs {
    /// Data directory for SQLite database and other persistent data
    pub data_dir: PathBuf,
    /// Log directory for rotating log files
    pub log_dir: PathBuf,
    /// Config directory for user configuration
    pub config_dir: PathBuf,
}

impl AppDirs {
    /// Create AppDirs from a Tauri App handle, creating directories if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Path resolution fails
    /// - Directory creation fails (e.g., permission denied)
    pub fn new(app: &tauri::App) -> TuskResult<Self> {
        let resolver = app.path();

        let data_dir = resolver
            .app_data_dir()
            .map_err(|e| TuskError::initialization(format!("Failed to resolve data directory: {}", e)))?;

        let log_dir = resolver
            .app_log_dir()
            .map_err(|e| TuskError::initialization(format!("Failed to resolve log directory: {}", e)))?;

        let config_dir = resolver
            .app_config_dir()
            .map_err(|e| TuskError::initialization(format!("Failed to resolve config directory: {}", e)))?;

        let dirs = Self {
            data_dir,
            log_dir,
            config_dir,
        };

        // Create all directories
        dirs.create_all()?;

        Ok(dirs)
    }

    /// Create all required directories.
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails due to permissions or other I/O errors.
    pub fn create_all(&self) -> TuskResult<()> {
        for (name, dir) in [
            ("data", &self.data_dir),
            ("log", &self.log_dir),
            ("config", &self.config_dir),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| {
                TuskError::initialization_with_hint(
                    format!("Failed to create {} directory at {:?}: {}", name, dir, e),
                    match e.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            "Check that you have write permissions to your home directory."
                        }
                        std::io::ErrorKind::NotFound => {
                            "Parent directory does not exist."
                        }
                        _ => "Check disk space and permissions.",
                    },
                )
            })?;

            tracing::debug!("Ensured {} directory exists: {:?}", name, dir);
        }

        Ok(())
    }

    /// Get the path to the SQLite database file.
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("tusk.db")
    }

    /// Get the path to the connections database file.
    /// This is separate from the main database for potential future use.
    pub fn connections_db_path(&self) -> PathBuf {
        self.data_dir.join("tusk.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_all_creates_directories() {
        let temp = tempdir().unwrap();
        let dirs = AppDirs {
            data_dir: temp.path().join("data"),
            log_dir: temp.path().join("logs"),
            config_dir: temp.path().join("config"),
        };

        dirs.create_all().unwrap();

        assert!(dirs.data_dir.exists());
        assert!(dirs.log_dir.exists());
        assert!(dirs.config_dir.exists());
    }

    #[test]
    fn test_database_path() {
        let temp = tempdir().unwrap();
        let dirs = AppDirs {
            data_dir: temp.path().join("data"),
            log_dir: temp.path().join("logs"),
            config_dir: temp.path().join("config"),
        };

        assert_eq!(dirs.database_path(), temp.path().join("data").join("tusk.db"));
    }
}

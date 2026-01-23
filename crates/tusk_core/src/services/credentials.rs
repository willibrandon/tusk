//! Credential storage with configurable providers.
//!
//! Provides secure password storage via pluggable credential providers:
//!
//! ## Provider Selection (T100, T101)
//!
//! - **Debug builds**: File-based storage at `~/.config/tusk/dev_credentials.json`
//!   - Avoids keychain popup issues with unsigned development builds
//!   - Override with `TUSK_USE_KEYCHAIN=1` to force keychain usage
//! - **Release builds**: OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
//!   - Code-signed release builds have stable identity for keychain ACLs
//!
//! See `/specs/004-service-integration/keychain-popup-analysis.md` for background.

use crate::error::TuskError;

use keyring::Entry;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// Service name used for keychain entries.
const KEYRING_SERVICE: &str = "dev.tusk.Tusk";

/// Environment variable to force keychain usage in debug builds (T101).
const FORCE_KEYCHAIN_ENV: &str = "TUSK_USE_KEYCHAIN";

// ============================================================================
// CredentialsProvider Trait (T097)
// ============================================================================

/// Trait for credential storage providers (T097).
///
/// Enables pluggable credential storage backends:
/// - File-based for development builds
/// - Keychain-based for release builds
pub trait CredentialsProvider: Send + Sync {
    /// Store a credential (T097).
    fn store(&self, key: &str, value: &str) -> Result<(), TuskError>;

    /// Get a credential (T097).
    fn get(&self, key: &str) -> Result<Option<String>, TuskError>;

    /// Delete a credential (T097).
    fn delete(&self, key: &str) -> Result<(), TuskError>;

    /// Check if a credential exists.
    fn exists(&self, key: &str) -> Result<bool, TuskError> {
        Ok(self.get(key)?.is_some())
    }

    /// Provider name for logging.
    fn name(&self) -> &'static str;
}

// ============================================================================
// FileCredentialsProvider (T098)
// ============================================================================

/// File-based credential storage for development builds (T098).
///
/// Stores credentials in a JSON file at `~/.config/tusk/dev_credentials.json`.
/// This avoids keychain popup issues with unsigned development builds.
#[derive(Debug)]
pub struct FileCredentialsProvider {
    /// Path to the credentials file.
    file_path: PathBuf,
    /// In-memory cache of credentials.
    cache: RwLock<HashMap<String, String>>,
}

/// Credentials file format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CredentialsFile {
    credentials: HashMap<String, String>,
}

impl FileCredentialsProvider {
    /// Create a new file-based credentials provider (T098).
    ///
    /// Uses `~/.config/tusk/dev_credentials.json` as the storage location.
    pub fn new() -> Result<Self, TuskError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| TuskError::storage("Could not determine config directory", None))?
            .join("tusk");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                TuskError::storage(
                    format!("Failed to create config directory: {e}"),
                    Some("Check permissions for ~/.config/tusk"),
                )
            })?;
        }

        let file_path = config_dir.join("dev_credentials.json");
        let provider = Self { file_path, cache: RwLock::new(HashMap::new()) };

        // Load existing credentials if file exists
        provider.load_from_file()?;

        Ok(provider)
    }

    /// Create with a custom file path (for testing).
    #[cfg(test)]
    pub fn with_path(file_path: PathBuf) -> Result<Self, TuskError> {
        let provider = Self { file_path, cache: RwLock::new(HashMap::new()) };
        provider.load_from_file()?;
        Ok(provider)
    }

    /// Load credentials from file into cache.
    fn load_from_file(&self) -> Result<(), TuskError> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let mut file = fs::File::open(&self.file_path).map_err(|e| {
            TuskError::storage(format!("Failed to open credentials file: {e}"), None)
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            TuskError::storage(format!("Failed to read credentials file: {e}"), None)
        })?;

        if contents.is_empty() {
            return Ok(());
        }

        let creds_file: CredentialsFile = serde_json::from_str(&contents).map_err(|e| {
            TuskError::storage(format!("Invalid credentials file format: {e}"), None)
        })?;

        *self.cache.write() = creds_file.credentials;
        Ok(())
    }

    /// Save credentials from cache to file (T102).
    ///
    /// Creates file with restricted permissions (600) on Unix systems.
    fn save_to_file(&self) -> Result<(), TuskError> {
        let creds_file = CredentialsFile { credentials: self.cache.read().clone() };

        let json = serde_json::to_string_pretty(&creds_file).map_err(|e| {
            TuskError::storage(format!("Failed to serialize credentials: {e}"), None)
        })?;

        // Create file with restricted permissions on Unix (T102)
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600) // Owner read/write only
                .open(&self.file_path)
                .map_err(|e| {
                    TuskError::storage(format!("Failed to create credentials file: {e}"), None)
                })?;
            file.write_all(json.as_bytes()).map_err(|e| {
                TuskError::storage(format!("Failed to write credentials file: {e}"), None)
            })?;
        }

        #[cfg(not(unix))]
        {
            fs::write(&self.file_path, json).map_err(|e| {
                TuskError::storage(format!("Failed to write credentials file: {e}"), None)
            })?;
        }

        Ok(())
    }
}

impl CredentialsProvider for FileCredentialsProvider {
    fn store(&self, key: &str, value: &str) -> Result<(), TuskError> {
        self.cache.write().insert(key.to_string(), value.to_string());
        self.save_to_file()?;
        tracing::debug!(key = key, "Credential stored in file");
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, TuskError> {
        Ok(self.cache.read().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), TuskError> {
        self.cache.write().remove(key);
        self.save_to_file()?;
        tracing::debug!(key = key, "Credential deleted from file");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "FileCredentialsProvider"
    }
}

// ============================================================================
// KeychainCredentialsProvider (T099)
// ============================================================================

/// Keychain-based credential storage for release builds (T099).
///
/// Uses the OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service).
#[derive(Debug)]
pub struct KeychainCredentialsProvider {
    /// Service name for keychain entries.
    service: String,
}

impl Default for KeychainCredentialsProvider {
    fn default() -> Self {
        Self { service: KEYRING_SERVICE.to_string() }
    }
}

impl KeychainCredentialsProvider {
    /// Create a new keychain credentials provider (T099).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a custom service name (for testing).
    #[cfg(test)]
    pub fn with_service(service: impl Into<String>) -> Self {
        Self { service: service.into() }
    }
}

impl CredentialsProvider for KeychainCredentialsProvider {
    fn store(&self, key: &str, value: &str) -> Result<(), TuskError> {
        Entry::new(&self.service, key)
            .map_err(|e| TuskError::keyring(e.to_string(), None))?
            .set_password(value)
            .map_err(|e| {
                TuskError::keyring(e.to_string(), Some("Grant Tusk access in system preferences"))
            })?;
        tracing::debug!(key = key, "Credential stored in keychain");
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, TuskError> {
        match Entry::new(&self.service, key) {
            Ok(entry) => match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(TuskError::keyring(
                    e.to_string(),
                    Some("Grant Tusk access in system preferences"),
                )),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    fn delete(&self, key: &str) -> Result<(), TuskError> {
        match Entry::new(&self.service, key) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) => {
                    tracing::debug!(key = key, "Credential deleted from keychain");
                    Ok(())
                }
                Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
                Err(e) => Err(TuskError::keyring(e.to_string(), None)),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    fn name(&self) -> &'static str {
        "KeychainCredentialsProvider"
    }
}

// ============================================================================
// SessionCredentialsProvider (Fallback)
// ============================================================================

/// In-memory session storage for when both file and keychain are unavailable.
///
/// Credentials are lost when the application exits.
#[derive(Debug, Default)]
pub struct SessionCredentialsProvider {
    store: RwLock<HashMap<String, String>>,
}

impl SessionCredentialsProvider {
    pub fn new() -> Self {
        Self { store: RwLock::new(HashMap::new()) }
    }
}

impl CredentialsProvider for SessionCredentialsProvider {
    fn store(&self, key: &str, value: &str) -> Result<(), TuskError> {
        self.store.write().insert(key.to_string(), value.to_string());
        tracing::debug!(key = key, "Credential stored in session");
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, TuskError> {
        Ok(self.store.read().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), TuskError> {
        self.store.write().remove(key);
        tracing::debug!(key = key, "Credential deleted from session");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "SessionCredentialsProvider"
    }
}

// ============================================================================
// CredentialService (Updated)
// ============================================================================

/// Select the appropriate credentials provider (T100, T101, T103).
fn select_provider() -> Box<dyn CredentialsProvider> {
    // Check for environment variable override (T101)
    let force_keychain = std::env::var(FORCE_KEYCHAIN_ENV).map(|v| v == "1").unwrap_or(false);

    // In debug builds, use file-based storage unless overridden (T100)
    #[cfg(debug_assertions)]
    {
        if force_keychain {
            tracing::debug!(
                provider = "KeychainCredentialsProvider",
                reason = "TUSK_USE_KEYCHAIN=1",
                "Using keychain provider (override)"
            );
            return Box::new(KeychainCredentialsProvider::new());
        }

        match FileCredentialsProvider::new() {
            Ok(provider) => {
                tracing::debug!(
                    provider = "FileCredentialsProvider",
                    reason = "debug build",
                    "Using file-based credential storage"
                );
                Box::new(provider)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create file provider, falling back to session");
                tracing::debug!(
                    provider = "SessionCredentialsProvider",
                    reason = "file provider failed",
                    "Using session-only credential storage"
                );
                Box::new(SessionCredentialsProvider::new())
            }
        }
    }

    // In release builds, use keychain (T100)
    #[cfg(not(debug_assertions))]
    {
        tracing::debug!(
            provider = "KeychainCredentialsProvider",
            reason = "release build",
            "Using keychain credential storage"
        );
        Box::new(KeychainCredentialsProvider::new())
    }
}

/// Secure credential storage service (T100).
///
/// Uses a pluggable provider system:
/// - Debug builds: File-based storage (avoids keychain popup issues)
/// - Release builds: OS keychain storage
///
/// The provider is selected automatically based on build type.
pub struct CredentialService {
    /// The active credential provider.
    provider: Box<dyn CredentialsProvider>,
}

impl CredentialService {
    /// Create a new credential service (T100, T103).
    ///
    /// Automatically selects the appropriate provider based on build type.
    /// Logs the selected provider at DEBUG level (T103).
    pub fn new() -> Self {
        let provider = select_provider();
        tracing::info!(provider = provider.name(), "Credential service initialized");
        Self { provider }
    }

    /// Get the name of the active provider.
    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    /// Check if using file-based storage.
    pub fn is_using_file_storage(&self) -> bool {
        self.provider.name() == "FileCredentialsProvider"
    }

    /// Check if using keychain storage.
    pub fn is_using_keychain(&self) -> bool {
        self.provider.name() == "KeychainCredentialsProvider"
    }

    /// Check if using session-only storage.
    pub fn is_using_session(&self) -> bool {
        self.provider.name() == "SessionCredentialsProvider"
    }

    /// Store a password for a database connection (FR-017, FR-018, SC-005).
    ///
    /// The password is stored securely via the active provider.
    ///
    /// # Arguments
    /// * `connection_id` - Unique identifier for the connection
    /// * `password` - The password to store (NEVER logged per FR-018)
    pub fn store_password(&self, connection_id: Uuid, password: &str) -> Result<(), TuskError> {
        let key = format!("db:{connection_id}");
        self.provider.store(&key, password)?;
        tracing::debug!(connection_id = %connection_id, "Password stored");
        Ok(())
    }

    /// Retrieve a password for a database connection (FR-019, SC-005).
    ///
    /// Returns None if no password is stored for this connection.
    pub fn get_password(&self, connection_id: Uuid) -> Result<Option<String>, TuskError> {
        let key = format!("db:{connection_id}");
        self.provider.get(&key)
    }

    /// Delete a stored password (FR-019).
    pub fn delete_password(&self, connection_id: Uuid) -> Result<(), TuskError> {
        let key = format!("db:{connection_id}");
        self.provider.delete(&key)?;
        tracing::debug!(connection_id = %connection_id, "Password deleted");
        Ok(())
    }

    /// Check if a password exists for a connection (FR-019).
    pub fn has_password(&self, connection_id: Uuid) -> Result<bool, TuskError> {
        let key = format!("db:{connection_id}");
        self.provider.exists(&key)
    }

    /// Store an SSH passphrase.
    pub fn store_ssh_passphrase(&self, tunnel_id: Uuid, passphrase: &str) -> Result<(), TuskError> {
        let key = format!("ssh:{tunnel_id}");
        self.provider.store(&key, passphrase)
    }

    /// Retrieve an SSH passphrase.
    pub fn get_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<Option<String>, TuskError> {
        let key = format!("ssh:{tunnel_id}");
        self.provider.get(&key)
    }

    /// Delete an SSH passphrase.
    pub fn delete_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<(), TuskError> {
        let key = format!("ssh:{tunnel_id}");
        self.provider.delete(&key)
    }
}

impl Default for CredentialService {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CredentialService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialService").field("provider", &self.provider.name()).finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_provider_store_and_get() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_creds.json");
        let provider = FileCredentialsProvider::with_path(file_path.clone()).unwrap();

        // Store a credential
        provider.store("test_key", "test_value").unwrap();

        // Retrieve it
        let value = provider.get("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Delete it
        provider.delete("test_key").unwrap();
        let value = provider.get("test_key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_file_provider_persistence() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_creds.json");

        // Store with one provider instance
        {
            let provider = FileCredentialsProvider::with_path(file_path.clone()).unwrap();
            provider.store("persist_key", "persist_value").unwrap();
        }

        // Read with a new provider instance
        {
            let provider = FileCredentialsProvider::with_path(file_path).unwrap();
            let value = provider.get("persist_key").unwrap();
            assert_eq!(value, Some("persist_value".to_string()));
        }
    }

    #[test]
    fn test_session_provider() {
        let provider = SessionCredentialsProvider::new();

        provider.store("session_key", "session_value").unwrap();
        let value = provider.get("session_key").unwrap();
        assert_eq!(value, Some("session_value".to_string()));

        provider.delete("session_key").unwrap();
        let value = provider.get("session_key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_credential_service_connection_password() {
        let service = CredentialService::new();
        let connection_id = Uuid::new_v4();

        // Store password
        service.store_password(connection_id, "test_password").unwrap();

        // Check exists
        assert!(service.has_password(connection_id).unwrap());

        // Retrieve
        let password = service.get_password(connection_id).unwrap();
        assert_eq!(password, Some("test_password".to_string()));

        // Delete
        service.delete_password(connection_id).unwrap();
        assert!(!service.has_password(connection_id).unwrap());
    }

    #[test]
    fn test_credential_service_ssh_passphrase() {
        let service = CredentialService::new();
        let tunnel_id = Uuid::new_v4();

        // Store passphrase
        service.store_ssh_passphrase(tunnel_id, "ssh_pass").unwrap();

        // Retrieve
        let passphrase = service.get_ssh_passphrase(tunnel_id).unwrap();
        assert_eq!(passphrase, Some("ssh_pass".to_string()));

        // Delete
        service.delete_ssh_passphrase(tunnel_id).unwrap();
        let passphrase = service.get_ssh_passphrase(tunnel_id).unwrap();
        assert_eq!(passphrase, None);
    }
}

//! OS keychain integration for secure credential storage.
//!
//! Provides secure password storage via the operating system's native keychain:
//! - macOS: Keychain Services
//! - Windows: Credential Manager
//! - Linux: Secret Service (via D-Bus)
//!
//! Falls back to in-memory session storage when keychain is unavailable (FR-019a).

use crate::error::TuskError;

use keyring::Entry;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Service name used for keychain entries.
const KEYRING_SERVICE: &str = "dev.tusk.Tusk";

/// Secure credential storage service.
///
/// Stores passwords in the OS keychain. If keychain access is denied,
/// falls back to in-memory session storage (cleared on app exit).
pub struct CredentialService {
    /// Whether keychain is accessible
    available: bool,
    /// Reason why keychain is unavailable (if applicable)
    fallback_reason: Option<String>,
    /// In-memory fallback when keychain unavailable
    session_store: Option<RwLock<HashMap<String, String>>>,
}

impl CredentialService {
    /// Create a new credential service.
    ///
    /// Performs a startup availability check to determine if the OS keychain
    /// is accessible. If not, falls back to session storage with a warning.
    pub fn new() -> Self {
        let (available, fallback_reason) = Self::check_availability();

        let session_store = if !available {
            tracing::warn!(
                reason = fallback_reason.as_deref(),
                "Keyring unavailable, using in-memory session storage. \
                 Passwords will be lost when the app closes."
            );
            Some(RwLock::new(HashMap::new()))
        } else {
            tracing::debug!("Keyring available, using OS keychain for credentials");
            None
        };

        Self { available, fallback_reason, session_store }
    }

    /// Check if keychain is available by attempting a test operation.
    fn check_availability() -> (bool, Option<String>) {
        match Entry::new(KEYRING_SERVICE, "__availability_check__") {
            Ok(entry) => match entry.set_password("test") {
                Ok(()) => {
                    // Clean up the test entry
                    let _ = entry.delete_credential();
                    (true, None)
                }
                Err(e) => (false, Some(e.to_string())),
            },
            Err(e) => (false, Some(e.to_string())),
        }
    }

    /// Check if the OS keychain is accessible.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get the reason why keychain is unavailable (if applicable).
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    /// Check if we're using the in-memory fallback.
    pub fn is_using_fallback(&self) -> bool {
        self.session_store.is_some()
    }

    /// Store a password for a database connection (FR-017, FR-018, SC-005).
    ///
    /// The password is stored securely in the OS keychain, or in session
    /// memory if the keychain is unavailable.
    ///
    /// # Arguments
    /// * `connection_id` - Unique identifier for the connection
    /// * `password` - The password to store (NEVER logged per FR-018)
    pub fn store_password(&self, connection_id: Uuid, password: &str) -> Result<(), TuskError> {
        let key = format!("db:{connection_id}");

        // Use session store if keychain unavailable
        if let Some(ref store) = self.session_store {
            store.write().insert(key, password.to_string());
            tracing::debug!(connection_id = %connection_id, "Password stored in session");
            return Ok(());
        }

        // Store in keychain
        Entry::new(KEYRING_SERVICE, &key)
            .map_err(|e| TuskError::keyring(e.to_string(), None))?
            .set_password(password)
            .map_err(|e| {
                TuskError::keyring(e.to_string(), Some("Grant Tusk access in system preferences"))
            })?;

        tracing::debug!(connection_id = %connection_id, "Password stored in keychain");
        Ok(())
    }

    /// Retrieve a password for a database connection (FR-019, SC-005).
    ///
    /// Returns None if no password is stored for this connection.
    pub fn get_password(&self, connection_id: Uuid) -> Result<Option<String>, TuskError> {
        let key = format!("db:{connection_id}");

        // Check session store if keychain unavailable
        if let Some(ref store) = self.session_store {
            let guard = store.read();
            return Ok(guard.get(&key).cloned());
        }

        // Retrieve from keychain
        match Entry::new(KEYRING_SERVICE, &key) {
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

    /// Delete a stored password (FR-019).
    pub fn delete_password(&self, connection_id: Uuid) -> Result<(), TuskError> {
        let key = format!("db:{connection_id}");

        // Remove from session store if keychain unavailable
        if let Some(ref store) = self.session_store {
            store.write().remove(&key);
            tracing::debug!(connection_id = %connection_id, "Password removed from session");
            return Ok(());
        }

        // Remove from keychain
        match Entry::new(KEYRING_SERVICE, &key) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) => {
                    tracing::debug!(connection_id = %connection_id, "Password removed from keychain");
                    Ok(())
                }
                Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
                Err(e) => Err(TuskError::keyring(e.to_string(), None)),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    /// Check if a password exists for a connection (FR-019).
    pub fn has_password(&self, connection_id: Uuid) -> Result<bool, TuskError> {
        let key = format!("db:{connection_id}");

        // Check session store if keychain unavailable
        if let Some(ref store) = self.session_store {
            return Ok(store.read().contains_key(&key));
        }

        // Check keychain
        match Entry::new(KEYRING_SERVICE, &key) {
            Ok(entry) => match entry.get_password() {
                Ok(_) => Ok(true),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(e) => Err(TuskError::keyring(e.to_string(), None)),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    /// Store an SSH passphrase.
    pub fn store_ssh_passphrase(&self, tunnel_id: Uuid, passphrase: &str) -> Result<(), TuskError> {
        let key = format!("ssh:{tunnel_id}");

        if let Some(ref store) = self.session_store {
            store.write().insert(key, passphrase.to_string());
            return Ok(());
        }

        Entry::new(KEYRING_SERVICE, &key)
            .map_err(|e| TuskError::keyring(e.to_string(), None))?
            .set_password(passphrase)
            .map_err(|e| TuskError::keyring(e.to_string(), None))
    }

    /// Retrieve an SSH passphrase.
    pub fn get_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<Option<String>, TuskError> {
        let key = format!("ssh:{tunnel_id}");

        if let Some(ref store) = self.session_store {
            return Ok(store.read().get(&key).cloned());
        }

        match Entry::new(KEYRING_SERVICE, &key) {
            Ok(entry) => match entry.get_password() {
                Ok(passphrase) => Ok(Some(passphrase)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(TuskError::keyring(e.to_string(), None)),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    /// Delete an SSH passphrase.
    pub fn delete_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<(), TuskError> {
        let key = format!("ssh:{tunnel_id}");

        if let Some(ref store) = self.session_store {
            store.write().remove(&key);
            return Ok(());
        }

        match Entry::new(KEYRING_SERVICE, &key) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(TuskError::keyring(e.to_string(), None)),
            },
            Err(e) => Err(TuskError::keyring(e.to_string(), None)),
        }
    }

    /// Clear all session credentials (called on app exit).
    ///
    /// Only affects in-memory session storage. Keychain credentials persist.
    pub fn clear_session_credentials(&self) {
        if let Some(ref store) = self.session_store {
            store.write().clear();
            tracing::debug!("Session credentials cleared");
        }
    }
}

impl Default for CredentialService {
    fn default() -> Self {
        Self::new()
    }
}

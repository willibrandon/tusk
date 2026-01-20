// Credentials service - full implementation in Phase 8 (User Story 6)

use crate::error::{TuskError, TuskResult};
use uuid::Uuid;

/// OS keychain credential service for secure password storage.
///
/// This service uses the system keychain (macOS Keychain, Windows Credential Manager,
/// or Linux Secret Service) to securely store database passwords.
pub struct CredentialService {
    /// Service name used as keychain namespace
    service_name: String,
}

impl CredentialService {
    /// Create a new credential service.
    ///
    /// # Arguments
    ///
    /// * `app_name` - Application name used as keychain namespace
    pub fn new(app_name: &str) -> Self {
        Self {
            service_name: format!("{}-credentials", app_name),
        }
    }

    /// Store a password in the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID used as the key
    /// * `password` - The password to store
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the keychain operation fails.
    pub fn store_password(&self, connection_id: &Uuid, password: &str) -> TuskResult<()> {
        let entry = keyring::Entry::new(&self.service_name, &connection_id.to_string())
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(password)
            .map_err(|e| TuskError::credential(format!("Failed to store password: {}", e)))?;

        tracing::debug!("Stored password for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve a password from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID used as the key
    ///
    /// # Returns
    ///
    /// Returns `Some(password)` if found, `None` if not found, or an error on failure.
    pub fn get_password(&self, connection_id: &Uuid) -> TuskResult<Option<String>> {
        let entry = keyring::Entry::new(&self.service_name, &connection_id.to_string())
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::credential(format!("Failed to retrieve password: {}", e))),
        }
    }

    /// Delete a password from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID used as the key
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (including if the password didn't exist).
    pub fn delete_password(&self, connection_id: &Uuid) -> TuskResult<()> {
        let entry = keyring::Entry::new(&self.service_name, &connection_id.to_string())
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted password for connection: {}", connection_id);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Password didn't exist, that's fine
                Ok(())
            }
            Err(e) => Err(TuskError::credential(format!("Failed to delete password: {}", e))),
        }
    }

    /// Check if a password exists for a connection.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID to check
    ///
    /// # Returns
    ///
    /// Returns `true` if a password is stored, `false` otherwise.
    pub fn has_password(&self, connection_id: &Uuid) -> TuskResult<bool> {
        match self.get_password(connection_id)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    // ================== SSH Credential Methods ==================

    /// Store an SSH password in the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    /// * `password` - The SSH password to store
    pub fn store_ssh_password(&self, connection_id: &Uuid, password: &str) -> TuskResult<()> {
        let key = format!("{}-ssh-password", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(password)
            .map_err(|e| TuskError::credential(format!("Failed to store SSH password: {}", e)))?;

        tracing::debug!("Stored SSH password for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve an SSH password from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    ///
    /// # Returns
    ///
    /// Returns `Some(password)` if found, `None` if not found.
    pub fn get_ssh_password(&self, connection_id: &Uuid) -> TuskResult<Option<String>> {
        let key = format!("{}-ssh-password", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::credential(format!("Failed to retrieve SSH password: {}", e))),
        }
    }

    /// Delete an SSH password from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    pub fn delete_ssh_password(&self, connection_id: &Uuid) -> TuskResult<()> {
        let key = format!("{}-ssh-password", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted SSH password for connection: {}", connection_id);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TuskError::credential(format!("Failed to delete SSH password: {}", e))),
        }
    }

    /// Store an SSH key passphrase in the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    /// * `passphrase` - The SSH key passphrase to store
    pub fn store_ssh_key_passphrase(&self, connection_id: &Uuid, passphrase: &str) -> TuskResult<()> {
        let key = format!("{}-ssh-key-passphrase", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(passphrase)
            .map_err(|e| TuskError::credential(format!("Failed to store SSH key passphrase: {}", e)))?;

        tracing::debug!("Stored SSH key passphrase for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve an SSH key passphrase from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    ///
    /// # Returns
    ///
    /// Returns `Some(passphrase)` if found, `None` if not found.
    pub fn get_ssh_key_passphrase(&self, connection_id: &Uuid) -> TuskResult<Option<String>> {
        let key = format!("{}-ssh-key-passphrase", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::credential(format!("Failed to retrieve SSH key passphrase: {}", e))),
        }
    }

    /// Delete an SSH key passphrase from the OS keychain.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    pub fn delete_ssh_key_passphrase(&self, connection_id: &Uuid) -> TuskResult<()> {
        let key = format!("{}-ssh-key-passphrase", connection_id);
        let entry = keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| TuskError::credential(format!("Failed to create keychain entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted SSH key passphrase for connection: {}", connection_id);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TuskError::credential(format!("Failed to delete SSH key passphrase: {}", e))),
        }
    }

    /// Delete all credentials (database password, SSH password, SSH passphrase) for a connection.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection UUID
    pub fn delete_all_credentials(&self, connection_id: &Uuid) -> TuskResult<()> {
        self.delete_password(connection_id)?;
        self.delete_ssh_password(connection_id)?;
        self.delete_ssh_key_passphrase(connection_id)?;
        Ok(())
    }
}

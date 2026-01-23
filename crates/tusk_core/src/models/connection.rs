//! Connection configuration and pool status models.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Current state of a database connection (FR-006).
///
/// Tracks the lifecycle of a connection from disconnected through connected,
/// with error states for failed connections.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    /// No active connection
    #[default]
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Active, healthy connection
    Connected,
    /// Connection failed or lost
    Error {
        /// Human-readable error message
        message: String,
        /// Whether the error is recoverable (can retry)
        recoverable: bool,
    },
}

impl ConnectionStatus {
    /// Create an error status with a recoverable flag.
    pub fn error(message: impl Into<String>, recoverable: bool) -> Self {
        Self::Error { message: message.into(), recoverable }
    }

    /// Check if the connection is active.
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if the connection is in an error state.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if the connection is connecting.
    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting)
    }

    /// Check if the connection is disconnected.
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    /// Get the error message if in error state.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Check if the error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Error { recoverable, .. } => *recoverable,
            _ => false,
        }
    }
}

/// SSL mode for database connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SslMode {
    /// No SSL
    Disable,
    /// Use SSL if available (default)
    #[default]
    Prefer,
    /// Require SSL, accept any certificate
    Require,
    /// Require SSL, verify CA
    VerifyCa,
    /// Require SSL, verify CA and hostname
    VerifyFull,
}

impl SslMode {
    /// Convert to string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require => "require",
            Self::VerifyCa => "verify-ca",
            Self::VerifyFull => "verify-full",
        }
    }

    /// Parse from string representation.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "disable" => Self::Disable,
            "require" => Self::Require,
            "verify-ca" | "verify_ca" => Self::VerifyCa,
            "verify-full" | "verify_full" => Self::VerifyFull,
            _ => Self::Prefer,
        }
    }
}

/// SSH authentication method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SshAuthMethod {
    /// Private key authentication
    Key,
    /// Password authentication (from keychain)
    Password,
    /// SSH agent authentication
    #[default]
    Agent,
}

impl SshAuthMethod {
    /// Convert to string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Password => "password",
            Self::Agent => "agent",
        }
    }

    /// Parse from string representation.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "key" => Self::Key,
            "password" => Self::Password,
            _ => Self::Agent,
        }
    }
}

/// SSH tunnel configuration for secure remote access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnelConfig {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// SSH server hostname
    pub host: String,
    /// SSH server port (default 22)
    pub port: u16,
    /// SSH username
    pub username: String,
    /// Authentication method
    pub auth_method: SshAuthMethod,
    /// Path to private key (required if auth_method = Key)
    pub key_path: Option<PathBuf>,
}

impl SshTunnelConfig {
    /// Create a new SSH tunnel configuration.
    pub fn new(
        name: impl Into<String>,
        host: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            host: host.into(),
            port: 22,
            username: username.into(),
            auth_method: SshAuthMethod::Agent,
            key_path: None,
        }
    }

    /// Set the port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set key-based authentication.
    pub fn with_key(mut self, key_path: impl Into<PathBuf>) -> Self {
        self.auth_method = SshAuthMethod::Key;
        self.key_path = Some(key_path.into());
        self
    }

    /// Set password-based authentication.
    pub fn with_password(mut self) -> Self {
        self.auth_method = SshAuthMethod::Password;
        self.key_path = None;
        self
    }
}

/// Additional connection options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    /// Connection timeout in seconds
    pub connect_timeout_secs: u32,
    /// Query timeout in seconds (None = no timeout)
    pub statement_timeout_secs: Option<u32>,
    /// Read-only mode
    pub read_only: bool,
    /// Application name sent to PostgreSQL
    pub application_name: String,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
            statement_timeout_secs: None,
            read_only: false,
            application_name: "Tusk".to_string(),
        }
    }
}

/// Configuration for a database connection (FR-012).
///
/// Note: Passwords are stored separately in the OS keychain via CredentialService,
/// never in this struct or in storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Unique identifier
    pub id: Uuid,
    /// Display name (1-255 chars)
    pub name: String,
    /// Server hostname or IP
    pub host: String,
    /// Server port (default 5432)
    pub port: u16,
    /// Database name (1-63 chars)
    pub database: String,
    /// Login username
    pub username: String,
    /// SSL configuration
    pub ssl_mode: SslMode,
    /// Optional SSH tunnel settings
    pub ssh_tunnel: Option<SshTunnelConfig>,
    /// Additional options
    pub options: ConnectionOptions,
    /// UI accent color (hex format, e.g., "#FF5733")
    pub color: Option<String>,
}

impl ConnectionConfig {
    /// Create a new connection configuration with required fields.
    pub fn new(
        name: impl Into<String>,
        host: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            host: host.into(),
            port: 5432,
            database: database.into(),
            username: username.into(),
            ssl_mode: SslMode::default(),
            ssh_tunnel: None,
            options: ConnectionOptions::default(),
            color: None,
        }
    }

    /// Create a builder for complex configurations.
    pub fn builder() -> ConnectionConfigBuilder {
        ConnectionConfigBuilder::default()
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() || self.name.len() > 255 {
            return Err("Name must be 1-255 characters".to_string());
        }
        if self.host.is_empty() {
            return Err("Host is required".to_string());
        }
        if self.database.is_empty() || self.database.len() > 63 {
            return Err("Database name must be 1-63 characters".to_string());
        }
        if self.username.is_empty() {
            return Err("Username is required".to_string());
        }
        if let Some(ref color) = self.color {
            if !color.starts_with('#') || color.len() != 7 {
                return Err("Color must be in hex format (#RRGGBB)".to_string());
            }
        }
        if let Some(ref tunnel) = self.ssh_tunnel {
            if tunnel.auth_method == SshAuthMethod::Key && tunnel.key_path.is_none() {
                return Err("Key path is required for key-based SSH authentication".to_string());
            }
        }
        Ok(())
    }

    /// Get the display connection string (without password).
    pub fn display_url(&self) -> String {
        format!("postgresql://{}@{}:{}/{}", self.username, self.host, self.port, self.database)
    }
}

/// Builder for ConnectionConfig.
#[derive(Debug, Default)]
pub struct ConnectionConfigBuilder {
    name: Option<String>,
    host: Option<String>,
    port: u16,
    database: Option<String>,
    username: Option<String>,
    ssl_mode: SslMode,
    ssh_tunnel: Option<SshTunnelConfig>,
    options: ConnectionOptions,
    color: Option<String>,
}

impl ConnectionConfigBuilder {
    /// Set the connection name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the database name.
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the username.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the SSL mode.
    pub fn ssl_mode(mut self, ssl_mode: SslMode) -> Self {
        self.ssl_mode = ssl_mode;
        self
    }

    /// Set the SSH tunnel.
    pub fn ssh_tunnel(mut self, tunnel: SshTunnelConfig) -> Self {
        self.ssh_tunnel = Some(tunnel);
        self
    }

    /// Set connection options.
    pub fn options(mut self, options: ConnectionOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the UI color.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout_secs(mut self, secs: u32) -> Self {
        self.options.connect_timeout_secs = secs;
        self
    }

    /// Set the statement timeout.
    pub fn statement_timeout_secs(mut self, secs: u32) -> Self {
        self.options.statement_timeout_secs = Some(secs);
        self
    }

    /// Set read-only mode.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.options.read_only = read_only;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> Result<ConnectionConfig, String> {
        let config = ConnectionConfig {
            id: Uuid::new_v4(),
            name: self.name.ok_or("Name is required")?,
            host: self.host.ok_or("Host is required")?,
            port: if self.port == 0 { 5432 } else { self.port },
            database: self.database.ok_or("Database is required")?,
            username: self.username.ok_or("Username is required")?,
            ssl_mode: self.ssl_mode,
            ssh_tunnel: self.ssh_tunnel,
            options: self.options,
            color: self.color,
        };
        config.validate()?;
        Ok(config)
    }
}

/// Connection pool status (FR-013, SC-010).
#[derive(Debug, Clone, Copy)]
pub struct PoolStatus {
    /// Maximum pool capacity
    pub max_size: usize,
    /// Current connections (idle + active)
    pub size: usize,
    /// Idle connections (can be negative during contention)
    pub available: isize,
    /// Tasks waiting for connections
    pub waiting: usize,
}

impl PoolStatus {
    /// Check if the pool is healthy.
    pub fn is_healthy(&self) -> bool {
        self.available >= 0 && self.waiting == 0
    }

    /// Get the number of active (in-use) connections.
    pub fn active(&self) -> usize {
        self.size.saturating_sub(self.available.max(0) as usize)
    }

    /// Get pool utilization as a percentage.
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.active() as f64 / self.max_size as f64) * 100.0
        }
    }
}

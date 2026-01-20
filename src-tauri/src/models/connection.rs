use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSL/TLS mode for PostgreSQL connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SslMode {
    /// Do not use SSL
    Disable,
    /// Try SSL, fall back to non-SSL if unavailable
    #[default]
    Prefer,
    /// Require SSL connection
    Require,
    /// Require SSL and verify server certificate against CA
    #[serde(rename = "verify-ca")]
    VerifyCa,
    /// Require SSL, verify CA, and verify server hostname
    #[serde(rename = "verify-full")]
    VerifyFull,
}

impl SslMode {
    /// Convert to string for PostgreSQL connection string
    pub fn as_str(&self) -> &'static str {
        match self {
            SslMode::Disable => "disable",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCa => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        }
    }
}

/// SSH authentication method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SshAuthMethod {
    /// Password authentication (password stored in keychain)
    Password,
    /// Private key file authentication
    KeyFile {
        /// Path to the private key file
        path: String,
    },
    /// SSH agent forwarding
    Agent,
}

/// SSH tunnel configuration for connections through an SSH server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnel {
    /// SSH server hostname
    pub host: String,
    /// SSH server port (default: 22)
    pub port: u16,
    /// SSH username
    pub username: String,
    /// Authentication method
    pub auth_method: SshAuthMethod,
    /// Local port for the tunnel (auto-assigned if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_port: Option<u16>,
}

impl Default for SshTunnel {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_method: SshAuthMethod::Agent,
            local_port: None,
        }
    }
}

/// Saved database connection configuration.
/// Passwords are NOT stored here - they are stored in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionConfig {
    /// Unique identifier
    pub id: Uuid,
    /// User-friendly display name
    pub name: String,
    /// PostgreSQL server hostname
    pub host: String,
    /// PostgreSQL server port (default: 5432)
    pub port: u16,
    /// Database name
    pub database: String,
    /// Authentication username
    pub username: String,
    /// SSL/TLS mode
    pub ssl_mode: SslMode,
    /// Path to CA certificate for verify-ca/verify-full modes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ca_cert: Option<String>,
    /// SSH tunnel configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_tunnel: Option<SshTunnel>,
    /// Whether to enforce read-only mode
    pub read_only: bool,
    /// Default statement timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_timeout_ms: Option<u64>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            host: "localhost".to_string(),
            port: 5432,
            database: String::new(),
            username: String::new(),
            ssl_mode: SslMode::Prefer,
            ssl_ca_cert: None,
            ssh_tunnel: None,
            read_only: false,
            statement_timeout_ms: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl ConnectionConfig {
    /// Create a new connection configuration with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Validate the connection configuration.
    pub fn validate(&self) -> Result<(), crate::error::TuskError> {
        if self.name.trim().is_empty() {
            return Err(crate::error::TuskError::validation_with_hint(
                "Connection name is required",
                "Enter a name to identify this connection",
            ));
        }
        if self.host.trim().is_empty() {
            return Err(crate::error::TuskError::validation_with_hint(
                "Host is required",
                "Enter the PostgreSQL server hostname or IP address",
            ));
        }
        if self.database.trim().is_empty() {
            return Err(crate::error::TuskError::validation_with_hint(
                "Database name is required",
                "Enter the name of the database to connect to",
            ));
        }
        if self.username.trim().is_empty() {
            return Err(crate::error::TuskError::validation_with_hint(
                "Username is required",
                "Enter the PostgreSQL username",
            ));
        }
        if self.port == 0 {
            return Err(crate::error::TuskError::validation_with_hint(
                "Invalid port number",
                "Port must be between 1 and 65535 (default: 5432)",
            ));
        }
        Ok(())
    }
}

/// Result of testing a connection without saving.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    /// Whether the connection succeeded
    pub success: bool,
    /// PostgreSQL server version if connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
    /// Connection latency in milliseconds
    pub latency_ms: u64,
    /// Error if connection failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::models::ErrorResponse>,
}

/// Information about an active connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConnection {
    /// Pool ID (same as config ID)
    pub id: Uuid,
    /// Connection name from config
    pub config_name: String,
    /// When the connection was established
    pub connected_at: DateTime<Utc>,
    /// Number of currently executing queries
    pub active_queries: usize,
}

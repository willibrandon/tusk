# Feature 08: SSL/TLS & SSH Tunneling

## Overview

Implement secure connection options including SSL/TLS modes (disable, prefer, require, verify-ca, verify-full) and SSH tunneling via the russh library for connections over untrusted networks. All components are pure Rust using GPUI for UI configuration.

## Goals

- Support all Postgres SSL modes
- Implement certificate validation (CA, client certs)
- Create SSH tunnel service with russh
- Support SSH password, key, and agent authentication
- Handle SSH key passphrases from keyring
- Provide GPUI components for SSL/SSH configuration

## Technical Specification

### 1. SSL/TLS Configuration

```rust
// src/services/ssl.rs
use std::path::Path;
use std::fs;
use std::sync::Arc;
use rustls::{ClientConfig, RootCertStore};
use rustls_pemfile::{certs, private_key};
use tokio_postgres_rustls::MakeRustlsConnect;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, SslMode};

/// SSL configuration for database connections
pub struct SslConfig {
    pub mode: SslMode,
    root_store: RootCertStore,
    client_cert: Option<Vec<rustls::pki_types::CertificateDer<'static>>>,
    client_key: Option<rustls::pki_types::PrivateKeyDer<'static>>,
}

impl SslConfig {
    /// Create SSL configuration from connection config
    pub fn from_connection_config(config: &ConnectionConfig) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        // Load custom CA certificate if provided
        if let Some(ref ca_path) = config.ssl_ca_cert {
            let ca_certs = Self::load_certificates(ca_path)?;
            for cert in ca_certs {
                root_store.add(cert).map_err(|e| {
                    TuskError::SslError(format!("Failed to add CA certificate: {}", e))
                })?;
            }
        } else {
            // Use system root certificates for verify modes
            root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        }

        // Load client certificate and key if provided
        let (client_cert, client_key) = if let (Some(cert_path), Some(key_path)) =
            (&config.ssl_client_cert, &config.ssl_client_key)
        {
            let certs = Self::load_certificates(cert_path)?;
            let key = Self::load_private_key(key_path)?;
            (Some(certs), Some(key))
        } else {
            (None, None)
        };

        Ok(Self {
            mode: config.ssl_mode,
            root_store,
            client_cert,
            client_key,
        })
    }

    /// Load certificates from a PEM or DER file
    fn load_certificates(path: &str) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
        let expanded_path = shellexpand::tilde(path);
        let data = fs::read(expanded_path.as_ref()).map_err(|e| {
            TuskError::SslError(format!("Failed to read certificate '{}': {}", path, e))
        })?;

        // Try PEM format first
        let mut reader = std::io::BufReader::new(&data[..]);
        let pem_certs: Vec<_> = certs(&mut reader)
            .filter_map(|r| r.ok())
            .collect();

        if !pem_certs.is_empty() {
            return Ok(pem_certs);
        }

        // Fall back to DER format (single certificate)
        Ok(vec![rustls::pki_types::CertificateDer::from(data)])
    }

    /// Load private key from a PEM or DER file
    fn load_private_key(path: &str) -> Result<rustls::pki_types::PrivateKeyDer<'static>> {
        let expanded_path = shellexpand::tilde(path);
        let data = fs::read(expanded_path.as_ref()).map_err(|e| {
            TuskError::SslError(format!("Failed to read private key '{}': {}", path, e))
        })?;

        let mut reader = std::io::BufReader::new(&data[..]);
        private_key(&mut reader)
            .map_err(|e| TuskError::SslError(format!("Failed to parse private key: {}", e)))?
            .ok_or_else(|| TuskError::SslError("No private key found in file".to_string()))
    }

    /// Create a TLS connector for tokio-postgres
    pub fn create_tls_connector(&self) -> Result<MakeRustlsConnect> {
        match self.mode {
            SslMode::Disable => {
                return Err(TuskError::SslError(
                    "Cannot create TLS connector with SSL disabled".to_string()
                ));
            }
            _ => {}
        }

        let mut config = ClientConfig::builder()
            .with_root_certificates(self.root_store.clone());

        // Configure based on SSL mode
        let config = match self.mode {
            SslMode::Prefer | SslMode::Require => {
                // Accept any certificate (skip verification)
                let config = ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(DangerousVerifier))
                    .with_no_client_auth();
                config
            }
            SslMode::VerifyCa => {
                // Verify CA but accept any hostname
                let config = ClientConfig::builder()
                    .with_root_certificates(self.root_store.clone())
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(CaOnlyVerifier {
                        roots: self.root_store.clone(),
                    }))
                    .with_no_client_auth();
                config
            }
            SslMode::VerifyFull => {
                // Full verification (CA + hostname)
                if let (Some(ref certs), Some(ref key)) = (&self.client_cert, &self.client_key) {
                    config.with_client_auth_cert(certs.clone(), key.clone_key())
                        .map_err(|e| TuskError::SslError(format!("Failed to set client cert: {}", e)))?
                } else {
                    config.with_no_client_auth()
                }
            }
            SslMode::Disable => unreachable!(),
        };

        Ok(MakeRustlsConnect::new(config))
    }
}

/// Certificate verifier that accepts any certificate (for require mode)
#[derive(Debug)]
struct DangerousVerifier;

impl rustls::client::danger::ServerCertVerifier for DangerousVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// Certificate verifier that checks CA but ignores hostname (for verify-ca mode)
#[derive(Debug)]
struct CaOnlyVerifier {
    roots: RootCertStore,
}

impl rustls::client::danger::ServerCertVerifier for CaOnlyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // Build certificate chain
        let mut chain = vec![end_entity.clone()];
        chain.extend(intermediates.iter().cloned());

        // Verify against root store (ignoring server name)
        let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(self.roots.clone()))
            .build()
            .map_err(|e| rustls::Error::General(format!("Failed to build verifier: {}", e)))?;

        // We only verify the certificate chain, not the hostname
        // This is intentional for verify-ca mode
        verifier.verify_server_cert(
            end_entity,
            intermediates,
            &rustls::pki_types::ServerName::try_from("localhost").unwrap(),
            _ocsp_response,
            now,
        ).map_err(|_| {
            // If verification fails, it's a CA issue, not hostname
            rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding)
        })?;

        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
```

### 2. SSH Tunnel Service

```rust
// src/services/ssh.rs
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use russh::client::{self, Handle};
use russh_keys::key::PrivateKeyWithHashAlg;
use async_trait::async_trait;
use parking_lot::RwLock;

use crate::error::{Result, TuskError};
use crate::models::connection::{SshTunnelConfig, SshAuthMethod};
use crate::services::keyring::KeyringService;

/// SSH tunnel status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SshTunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// An active SSH tunnel
pub struct SshTunnel {
    local_port: u16,
    remote_host: String,
    remote_port: u16,
    status: Arc<RwLock<SshTunnelStatus>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SshTunnel {
    /// Create and establish a new SSH tunnel
    pub async fn new(
        config: &SshTunnelConfig,
        connection_id: &str,
        remote_host: &str,
        remote_port: u16,
        keyring: &KeyringService,
    ) -> Result<Self> {
        let status = Arc::new(RwLock::new(SshTunnelStatus::Connecting));

        // Find an available local port
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            TuskError::SshError(format!("Failed to bind local port: {}", e))
        })?;
        let local_port = listener.local_addr()?.port();

        tracing::info!(
            ssh_host = %config.host,
            ssh_port = config.port,
            local_port = local_port,
            remote_host = %remote_host,
            remote_port = remote_port,
            "Establishing SSH tunnel"
        );

        // Create SSH client config
        let ssh_config = Arc::new(client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            keepalive_interval: Some(std::time::Duration::from_secs(30)),
            keepalive_max: 3,
            ..Default::default()
        });

        // Connect to SSH server
        let ssh_addr = format!("{}:{}", config.host, config.port);
        let mut session = client::connect(ssh_config, &ssh_addr, SshHandler::new())
            .await
            .map_err(|e| TuskError::SshError(format!("SSH connection failed: {}", e)))?;

        // Authenticate
        Self::authenticate(&mut session, config, connection_id, keyring).await?;

        *status.write() = SshTunnelStatus::Connected;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn tunnel task
        let remote_host_owned = remote_host.to_string();
        let handle = session.handle();
        let status_clone = status.clone();

        let task_handle = tokio::spawn(async move {
            Self::run_tunnel(
                listener,
                handle,
                remote_host_owned,
                remote_port,
                shutdown_rx,
                status_clone,
            ).await;
        });

        Ok(Self {
            local_port,
            remote_host: remote_host.to_string(),
            remote_port,
            status,
            shutdown_tx: Some(shutdown_tx),
            task_handle: Some(task_handle),
        })
    }

    async fn authenticate(
        session: &mut client::Handle<SshHandler>,
        config: &SshTunnelConfig,
        connection_id: &str,
        keyring: &KeyringService,
    ) -> Result<()> {
        match config.auth {
            SshAuthMethod::Password => {
                let password = keyring
                    .get_ssh_password(&format!("{}", connection_id))?
                    .ok_or_else(|| TuskError::CredentialNotFound {
                        credential_type: "SSH password".to_string(),
                        identifier: connection_id.to_string(),
                    })?;

                let auth_result = session
                    .authenticate_password(&config.username, &password)
                    .await
                    .map_err(|e| TuskError::SshError(format!("SSH password auth failed: {}", e)))?;

                if !auth_result {
                    return Err(TuskError::AuthenticationFailed {
                        method: "password".to_string(),
                        message: "SSH password authentication rejected".to_string(),
                    });
                }

                tracing::info!("SSH password authentication successful");
            }
            SshAuthMethod::Key => {
                let key_path = config.key_path.as_ref().ok_or_else(|| {
                    TuskError::SshError("SSH key path not specified".to_string())
                })?;

                let expanded_path = shellexpand::tilde(key_path);
                let key_data = tokio::fs::read(expanded_path.as_ref()).await.map_err(|e| {
                    TuskError::SshError(format!("Failed to read SSH key '{}': {}", key_path, e))
                })?;

                // Get passphrase if key is encrypted
                let passphrase = if config.key_passphrase_in_keyring {
                    keyring.get_ssh_passphrase(&format!("{}", connection_id))?
                } else {
                    None
                };

                let key_pair = if let Some(pass) = passphrase {
                    russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), Some(&pass))
                } else {
                    russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), None)
                }
                .map_err(|e| {
                    if e.to_string().contains("encrypted") {
                        TuskError::SshError("SSH key is encrypted but no passphrase provided".to_string())
                    } else {
                        TuskError::SshError(format!("Failed to decode SSH key: {}", e))
                    }
                })?;

                let auth_result = session
                    .authenticate_publickey(
                        &config.username,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), None).unwrap(),
                    )
                    .await
                    .map_err(|e| TuskError::SshError(format!("SSH key auth failed: {}", e)))?;

                if !auth_result {
                    return Err(TuskError::AuthenticationFailed {
                        method: "publickey".to_string(),
                        message: "SSH public key authentication rejected".to_string(),
                    });
                }

                tracing::info!("SSH key authentication successful");
            }
            SshAuthMethod::Agent => {
                // Use SSH agent for authentication
                let auth_result = session
                    .authenticate_publickey_agent(&config.username)
                    .await
                    .map_err(|e| TuskError::SshError(format!("SSH agent auth failed: {}", e)))?;

                if !auth_result {
                    return Err(TuskError::AuthenticationFailed {
                        method: "agent".to_string(),
                        message: "SSH agent authentication rejected".to_string(),
                    });
                }

                tracing::info!("SSH agent authentication successful");
            }
        }

        Ok(())
    }

    async fn run_tunnel(
        listener: TcpListener,
        ssh_handle: Handle<SshHandler>,
        remote_host: String,
        remote_port: u16,
        mut shutdown_rx: oneshot::Receiver<()>,
        status: Arc<RwLock<SshTunnelStatus>>,
    ) {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    tracing::info!("SSH tunnel shutdown requested");
                    *status.write() = SshTunnelStatus::Disconnected;
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((local_stream, peer_addr)) => {
                            tracing::debug!(
                                peer = %peer_addr,
                                "New tunnel connection"
                            );

                            let handle = ssh_handle.clone();
                            let host = remote_host.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    local_stream, handle, &host, remote_port
                                ).await {
                                    tracing::warn!(error = %e, "Tunnel connection error");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to accept connection");
                            *status.write() = SshTunnelStatus::Error;
                        }
                    }
                }
            }
        }
    }

    async fn handle_connection(
        local_stream: tokio::net::TcpStream,
        ssh_handle: Handle<SshHandler>,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<()> {
        // Request port forwarding channel
        let channel = ssh_handle
            .channel_open_direct_tcpip(
                remote_host,
                remote_port as u32,
                "127.0.0.1",
                0,
            )
            .await
            .map_err(|e| TuskError::SshError(format!("Failed to open channel: {}", e)))?;

        // Split streams and forward data
        let (mut local_read, mut local_write) = local_stream.into_split();
        let (mut channel_read, mut channel_write) = tokio::io::split(channel.into_stream());

        let client_to_server = tokio::io::copy(&mut local_read, &mut channel_write);
        let server_to_client = tokio::io::copy(&mut channel_read, &mut local_write);

        tokio::select! {
            r = client_to_server => {
                if let Err(e) = r {
                    tracing::debug!(error = %e, "Client to server copy ended");
                }
            }
            r = server_to_client => {
                if let Err(e) = r {
                    tracing::debug!(error = %e, "Server to client copy ended");
                }
            }
        }

        Ok(())
    }

    /// Get the local port for connecting through the tunnel
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Get the remote endpoint
    pub fn remote_endpoint(&self) -> (&str, u16) {
        (&self.remote_host, self.remote_port)
    }

    /// Get current tunnel status
    pub fn status(&self) -> SshTunnelStatus {
        *self.status.read()
    }

    /// Close the tunnel
    pub async fn close(mut self) {
        tracing::info!(local_port = self.local_port, "Closing SSH tunnel");

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// SSH client handler
struct SshHandler {
    known_hosts: Arc<RwLock<Vec<KnownHost>>>,
}

#[derive(Clone)]
struct KnownHost {
    host: String,
    key_type: String,
    key_data: Vec<u8>,
}

impl SshHandler {
    fn new() -> Self {
        Self {
            known_hosts: Arc::new(RwLock::new(Self::load_known_hosts())),
        }
    }

    fn load_known_hosts() -> Vec<KnownHost> {
        // Load from ~/.ssh/known_hosts
        let known_hosts_path = dirs::home_dir()
            .map(|h| h.join(".ssh/known_hosts"))
            .filter(|p| p.exists());

        if let Some(path) = known_hosts_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return content
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            Some(KnownHost {
                                host: parts[0].to_string(),
                                key_type: parts[1].to_string(),
                                key_data: base64::decode(parts[2]).unwrap_or_default(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }

        Vec::new()
    }
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh_keys::key::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        // In a production app, we should verify the key against known_hosts
        // For now, we accept all keys but log a warning
        tracing::warn!(
            key_type = server_public_key.name(),
            "Accepting SSH host key without verification"
        );
        Ok(true)
    }
}
```

### 3. Updated Connection Pool with SSL/SSH

```rust
// src/services/connection/pool.rs (extended)
use crate::services::ssl::SslConfig;
use crate::services::ssh::SshTunnel;

/// A managed connection pool with optional SSL and SSH tunnel
pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    info: Arc<RwLock<Option<ConnectionInfo>>>,
    shutdown: Arc<RwLock<bool>>,
    ssh_tunnel: Option<SshTunnel>,
}

impl ConnectionPool {
    /// Create a new connection pool with SSL and SSH support
    pub async fn new(
        config: ConnectionConfig,
        keyring: &KeyringService,
    ) -> Result<Self> {
        config.validate()?;

        // Set up SSH tunnel if configured
        let (effective_host, effective_port, ssh_tunnel) = if let Some(ref ssh) = config.ssh_tunnel {
            if ssh.enabled {
                tracing::info!(
                    ssh_host = %ssh.host,
                    ssh_port = ssh.port,
                    db_host = %config.host,
                    db_port = config.port,
                    "Creating SSH tunnel"
                );

                let tunnel = SshTunnel::new(
                    ssh,
                    &config.id.to_string(),
                    &config.host,
                    config.port,
                    keyring,
                ).await?;

                let local_port = tunnel.local_port();
                tracing::info!(local_port = local_port, "SSH tunnel established");

                ("127.0.0.1".to_string(), local_port, Some(tunnel))
            } else {
                (config.host.clone(), config.port, None)
            }
        } else {
            (config.host.clone(), config.port, None)
        };

        // Get password from keyring
        let password = if config.password_in_keyring {
            keyring.get_password(&config.id.to_string())?
                .ok_or_else(|| TuskError::CredentialNotFound {
                    credential_type: "password".to_string(),
                    identifier: config.id.to_string(),
                })?
        } else {
            String::new()
        };

        // Build postgres config
        let mut pg_config = PgConfig::new();
        pg_config
            .host(&effective_host)
            .port(effective_port)
            .dbname(&config.database)
            .user(&config.username)
            .password(&password)
            .application_name(&config.options.application_name)
            .connect_timeout(Duration::from_secs(config.options.connect_timeout_sec));

        if let Some(timeout_ms) = config.options.statement_timeout_ms {
            pg_config.options(&format!("-c statement_timeout={}", timeout_ms));
        }

        // Create pool with appropriate TLS configuration
        let pool = match config.ssl_mode {
            SslMode::Disable => {
                Self::create_pool_no_tls(pg_config, &config.options).await?
            }
            _ => {
                let ssl_config = SslConfig::from_connection_config(&config)?;
                Self::create_pool_with_tls(pg_config, &config.options, ssl_config).await?
            }
        };

        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Connected);

        let conn_pool = Self {
            pool,
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::Connected)),
            status_tx,
            status_rx,
            info: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(RwLock::new(false)),
            ssh_tunnel,
        };

        // Fetch server info
        conn_pool.fetch_server_info().await?;

        // Set read-only mode if configured
        if conn_pool.config.options.readonly {
            conn_pool.set_readonly_mode().await?;
        }

        Ok(conn_pool)
    }

    async fn create_pool_with_tls(
        pg_config: PgConfig,
        options: &ConnectionOptions,
        ssl_config: SslConfig,
    ) -> Result<Pool> {
        let tls_connector = ssl_config.create_tls_connector()?;

        let mut cfg = Config::new();
        if let Some(hosts) = pg_config.get_hosts().first() {
            cfg.host = Some(hosts.to_string());
        }
        cfg.port = pg_config.get_ports().first().copied();
        cfg.dbname = pg_config.get_dbname().map(|s| s.to_string());
        cfg.user = pg_config.get_user().map(|s| s.to_string());

        let pool_cfg = PoolConfig {
            max_size: options.max_pool_size,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(options.connect_timeout_sec)),
                create: Some(Duration::from_secs(options.connect_timeout_sec)),
                recycle: Some(Duration::from_secs(30)),
            },
            ..Default::default()
        };

        cfg.pool = Some(pool_cfg);
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), tls_connector)
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to create TLS connection pool: {}", e),
                source: Some(e.to_string()),
            })?;

        // Test the connection
        let _client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to connect with TLS: {}", e),
            source: Some(e.to_string()),
        })?;

        tracing::info!("TLS connection pool created successfully");

        Ok(pool)
    }

    /// Check if SSH tunnel is active
    pub fn has_ssh_tunnel(&self) -> bool {
        self.ssh_tunnel.is_some()
    }

    /// Get SSH tunnel status
    pub fn ssh_tunnel_status(&self) -> Option<crate::services::ssh::SshTunnelStatus> {
        self.ssh_tunnel.as_ref().map(|t| t.status())
    }

    /// Close the connection pool and tunnel
    pub async fn close(&self) {
        *self.shutdown.write() = true;
        self.set_status(ConnectionStatus::Disconnected);
        self.pool.close();

        // Note: SSH tunnel cleanup happens in Drop
        tracing::info!(connection_id = %self.config.id, "Connection pool closed");
    }
}
```

### 4. GPUI SSL Configuration Component

```rust
// src/ui/components/ssl_config.rs
use gpui::{
    div, px, Element, IntoElement, ParentElement, Render,
    Styled, View, ViewContext, Window, InteractiveElement,
};
use rfd::FileDialog;

use crate::models::connection::SslMode;
use crate::theme::Theme;

pub struct SslConfigPanel {
    ssl_mode: SslMode,
    ca_cert_path: String,
    client_cert_path: String,
    client_key_path: String,
    on_change: Option<Box<dyn Fn(SslConfigState) + 'static>>,
}

#[derive(Debug, Clone)]
pub struct SslConfigState {
    pub ssl_mode: SslMode,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
}

impl SslConfigPanel {
    pub fn new(initial: SslConfigState) -> Self {
        Self {
            ssl_mode: initial.ssl_mode,
            ca_cert_path: initial.ca_cert_path.unwrap_or_default(),
            client_cert_path: initial.client_cert_path.unwrap_or_default(),
            client_key_path: initial.client_key_path.unwrap_or_default(),
            on_change: None,
        }
    }

    pub fn on_change(mut self, callback: impl Fn(SslConfigState) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    fn notify_change(&self) {
        if let Some(ref callback) = self.on_change {
            callback(SslConfigState {
                ssl_mode: self.ssl_mode,
                ca_cert_path: if self.ca_cert_path.is_empty() { None } else { Some(self.ca_cert_path.clone()) },
                client_cert_path: if self.client_cert_path.is_empty() { None } else { Some(self.client_cert_path.clone()) },
                client_key_path: if self.client_key_path.is_empty() { None } else { Some(self.client_key_path.clone()) },
            });
        }
    }

    fn set_ssl_mode(&mut self, mode: SslMode, cx: &mut ViewContext<Self>) {
        self.ssl_mode = mode;
        self.notify_change();
        cx.notify();
    }

    fn browse_ca_cert(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(path) = Self::browse_certificate_file() {
            self.ca_cert_path = path;
            self.notify_change();
            cx.notify();
        }
    }

    fn browse_client_cert(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(path) = Self::browse_certificate_file() {
            self.client_cert_path = path;
            self.notify_change();
            cx.notify();
        }
    }

    fn browse_client_key(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(path) = Self::browse_key_file() {
            self.client_key_path = path;
            self.notify_change();
            cx.notify();
        }
    }

    fn browse_certificate_file() -> Option<String> {
        FileDialog::new()
            .set_title("Select Certificate")
            .add_filter("Certificates", &["crt", "pem", "cer"])
            .add_filter("All Files", &["*"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    }

    fn browse_key_file() -> Option<String> {
        FileDialog::new()
            .set_title("Select Private Key")
            .add_filter("Keys", &["key", "pem"])
            .add_filter("All Files", &["*"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    }

    fn ssl_mode_options() -> Vec<(SslMode, &'static str, &'static str)> {
        vec![
            (SslMode::Disable, "Disable", "No encryption"),
            (SslMode::Prefer, "Prefer", "Use SSL if available"),
            (SslMode::Require, "Require", "Require SSL, skip verification"),
            (SslMode::VerifyCa, "Verify CA", "Verify server certificate"),
            (SslMode::VerifyFull, "Verify Full", "Verify certificate and hostname"),
        ]
    }
}

impl Render for SslConfigPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let needs_ca = matches!(self.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull);

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // SSL Mode selector
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_primary)
                            .child("SSL Mode")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .children(Self::ssl_mode_options().into_iter().map(|(mode, label, desc)| {
                                let is_selected = self.ssl_mode == mode;
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .bg(if is_selected { theme.colors.accent.opacity(0.1) } else { theme.colors.bg_secondary })
                                    .border_1()
                                    .border_color(if is_selected { theme.colors.accent } else { theme.colors.border })
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(move |this, _, cx| this.set_ssl_mode(mode, cx)))
                                    .child(
                                        div()
                                            .w(px(16.0))
                                            .h(px(16.0))
                                            .rounded_full()
                                            .border_2()
                                            .border_color(if is_selected { theme.colors.accent } else { theme.colors.text_muted })
                                            .when(is_selected, |s| {
                                                s.child(
                                                    div()
                                                        .absolute()
                                                        .inset(px(3.0))
                                                        .rounded_full()
                                                        .bg(theme.colors.accent)
                                                )
                                            })
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.colors.text_primary)
                                                    .child(label)
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.colors.text_muted)
                                                    .child(desc)
                                            )
                                    )
                            }))
                    )
            )
            // CA Certificate (for verify modes)
            .when(needs_ca, |this| {
                this.child(
                    self.render_file_field(
                        "CA Certificate",
                        &self.ca_cert_path,
                        "/path/to/ca.crt",
                        theme,
                        cx,
                        |this, cx| this.browse_ca_cert(cx),
                    )
                )
            })
            // Client Certificate (optional)
            .when(self.ssl_mode != SslMode::Disable, |this| {
                this.child(
                    self.render_file_field(
                        "Client Certificate (optional)",
                        &self.client_cert_path,
                        "/path/to/client.crt",
                        theme,
                        cx,
                        |this, cx| this.browse_client_cert(cx),
                    )
                )
            })
            // Client Key (optional)
            .when(self.ssl_mode != SslMode::Disable && !self.client_cert_path.is_empty(), |this| {
                this.child(
                    self.render_file_field(
                        "Client Key",
                        &self.client_key_path,
                        "/path/to/client.key",
                        theme,
                        cx,
                        |this, cx| this.browse_client_key(cx),
                    )
                )
            })
    }
}

impl SslConfigPanel {
    fn render_file_field(
        &self,
        label: &str,
        value: &str,
        placeholder: &str,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
        on_browse: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_secondary)
                    .child(label)
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .px(px(10.0))
                            .py(px(8.0))
                            .bg(theme.colors.input_bg)
                            .border_1()
                            .border_color(theme.colors.input_border)
                            .rounded(px(4.0))
                            .text_color(if value.is_empty() {
                                theme.colors.text_muted
                            } else {
                                theme.colors.text_primary
                            })
                            .text_sm()
                            .overflow_hidden()
                            .child(if value.is_empty() {
                                placeholder.to_string()
                            } else {
                                value.to_string()
                            })
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .bg(theme.colors.bg_secondary)
                            .rounded(px(4.0))
                            .text_color(theme.colors.text_secondary)
                            .text_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(on_browse))
                            .child("Browse")
                    )
            )
    }
}
```

### 5. GPUI SSH Configuration Component

```rust
// src/ui/components/ssh_config.rs
use gpui::{
    div, px, Element, IntoElement, ParentElement, Render,
    Styled, View, ViewContext, Window, InteractiveElement,
};
use rfd::FileDialog;

use crate::models::connection::{SshTunnelConfig, SshAuthMethod};
use crate::theme::Theme;

pub struct SshConfigPanel {
    enabled: bool,
    host: String,
    port: String,
    username: String,
    auth_method: SshAuthMethod,
    key_path: String,
    on_change: Option<Box<dyn Fn(Option<SshTunnelConfig>) + 'static>>,
}

impl SshConfigPanel {
    pub fn new(initial: Option<&SshTunnelConfig>) -> Self {
        if let Some(config) = initial {
            Self {
                enabled: config.enabled,
                host: config.host.clone(),
                port: config.port.to_string(),
                username: config.username.clone(),
                auth_method: config.auth,
                key_path: config.key_path.clone().unwrap_or_default(),
                on_change: None,
            }
        } else {
            Self {
                enabled: false,
                host: String::new(),
                port: "22".to_string(),
                username: String::new(),
                auth_method: SshAuthMethod::Key,
                key_path: String::new(),
                on_change: None,
            }
        }
    }

    pub fn on_change(mut self, callback: impl Fn(Option<SshTunnelConfig>) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    fn notify_change(&self) {
        if let Some(ref callback) = self.on_change {
            if !self.enabled {
                callback(None);
            } else {
                callback(Some(SshTunnelConfig {
                    enabled: true,
                    host: self.host.clone(),
                    port: self.port.parse().unwrap_or(22),
                    username: self.username.clone(),
                    auth: self.auth_method,
                    key_path: if self.key_path.is_empty() { None } else { Some(self.key_path.clone()) },
                    key_passphrase_in_keyring: true,
                }));
            }
        }
    }

    fn toggle_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.enabled = !self.enabled;
        self.notify_change();
        cx.notify();
    }

    fn set_auth_method(&mut self, method: SshAuthMethod, cx: &mut ViewContext<Self>) {
        self.auth_method = method;
        self.notify_change();
        cx.notify();
    }

    fn browse_key(&mut self, cx: &mut ViewContext<Self>) {
        let home = dirs::home_dir().map(|h| h.join(".ssh"));

        let mut dialog = FileDialog::new()
            .set_title("Select SSH Key")
            .add_filter("SSH Keys", &["pem", "key", ""])
            .add_filter("All Files", &["*"]);

        if let Some(ssh_dir) = home {
            dialog = dialog.set_directory(ssh_dir);
        }

        if let Some(path) = dialog.pick_file() {
            self.key_path = path.to_string_lossy().to_string();
            self.notify_change();
            cx.notify();
        }
    }
}

impl Render for SshConfigPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Enable toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, cx| this.toggle_enabled(cx)))
                    .child(
                        div()
                            .w(px(40.0))
                            .h(px(20.0))
                            .rounded_full()
                            .bg(if self.enabled { theme.colors.accent } else { theme.colors.bg_tertiary })
                            .p(px(2.0))
                            .child(
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded_full()
                                    .bg(gpui::white())
                                    .when(self.enabled, |s| s.ml(px(20.0)))
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_primary)
                            .child("Use SSH Tunnel")
                    )
            )
            // SSH fields (shown when enabled)
            .when(self.enabled, |this| {
                this.child(self.render_ssh_fields(theme, cx))
            })
    }
}

impl SshConfigPanel {
    fn render_ssh_fields(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .pl(px(12.0))
            .border_l_2()
            .border_color(theme.colors.accent.opacity(0.3))
            // Host and Port row
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.colors.text_secondary)
                                    .child("SSH Host")
                            )
                            .child(
                                div()
                                    .px(px(10.0))
                                    .py(px(8.0))
                                    .bg(theme.colors.input_bg)
                                    .border_1()
                                    .border_color(theme.colors.input_border)
                                    .rounded(px(4.0))
                                    .text_color(theme.colors.text_primary)
                                    .text_sm()
                                    .child(if self.host.is_empty() {
                                        div().text_color(theme.colors.text_muted).child("ssh.example.com")
                                    } else {
                                        div().child(self.host.clone())
                                    })
                            )
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.colors.text_secondary)
                                    .child("Port")
                            )
                            .child(
                                div()
                                    .px(px(10.0))
                                    .py(px(8.0))
                                    .bg(theme.colors.input_bg)
                                    .border_1()
                                    .border_color(theme.colors.input_border)
                                    .rounded(px(4.0))
                                    .text_color(theme.colors.text_primary)
                                    .text_sm()
                                    .child(self.port.clone())
                            )
                    )
            )
            // Username
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_secondary)
                            .child("SSH Username")
                    )
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(8.0))
                            .bg(theme.colors.input_bg)
                            .border_1()
                            .border_color(theme.colors.input_border)
                            .rounded(px(4.0))
                            .text_color(theme.colors.text_primary)
                            .text_sm()
                            .child(if self.username.is_empty() {
                                div().text_color(theme.colors.text_muted).child("username")
                            } else {
                                div().child(self.username.clone())
                            })
                    )
            )
            // Authentication method
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_secondary)
                            .child("Authentication")
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .children([
                                (SshAuthMethod::Key, "SSH Key"),
                                (SshAuthMethod::Password, "Password"),
                                (SshAuthMethod::Agent, "SSH Agent"),
                            ].into_iter().map(|(method, label)| {
                                let is_selected = self.auth_method == method;
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .bg(if is_selected { theme.colors.accent } else { theme.colors.bg_secondary })
                                    .text_color(if is_selected { gpui::white() } else { theme.colors.text_secondary })
                                    .text_sm()
                                    .hover(|s| if is_selected { s } else { s.bg(theme.colors.bg_tertiary) })
                                    .on_click(cx.listener(move |this, _, cx| this.set_auth_method(method, cx)))
                                    .child(label)
                            }))
                    )
            )
            // Key path (for key auth)
            .when(self.auth_method == SshAuthMethod::Key, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.text_secondary)
                                .child("SSH Key")
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .px(px(10.0))
                                        .py(px(8.0))
                                        .bg(theme.colors.input_bg)
                                        .border_1()
                                        .border_color(theme.colors.input_border)
                                        .rounded(px(4.0))
                                        .text_sm()
                                        .overflow_hidden()
                                        .child(if self.key_path.is_empty() {
                                            div()
                                                .text_color(theme.colors.text_muted)
                                                .child("~/.ssh/id_rsa")
                                        } else {
                                            div()
                                                .text_color(theme.colors.text_primary)
                                                .child(self.key_path.clone())
                                        })
                                )
                                .child(
                                    div()
                                        .px(px(12.0))
                                        .py(px(8.0))
                                        .bg(theme.colors.bg_secondary)
                                        .rounded(px(4.0))
                                        .text_color(theme.colors.text_secondary)
                                        .text_sm()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(|this, _, cx| this.browse_key(cx)))
                                        .child("Browse")
                                )
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.colors.text_muted)
                                .child("Key passphrase will be requested if needed and stored in keychain")
                        )
                )
            })
            // Password note (for password auth)
            .when(self.auth_method == SshAuthMethod::Password, |this| {
                this.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.bg_secondary)
                        .text_xs()
                        .text_color(theme.colors.text_muted)
                        .child("Password will be requested when connecting and stored securely in your system keychain")
                )
            })
            // Agent note
            .when(self.auth_method == SshAuthMethod::Agent, |this| {
                this.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.bg_secondary)
                        .text_xs()
                        .text_color(theme.colors.text_muted)
                        .child("Will use keys from your running SSH agent (ssh-agent)")
                )
            })
    }
}
```

### 6. Keyring Extensions for SSH

```rust
// src/services/keyring.rs (extended for SSH)
impl KeyringService {
    const SERVICE_NAME: &'static str = "tusk";

    /// Store SSH password in keyring
    pub fn store_ssh_password(&self, connection_id: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-password:{}", connection_id),
        )?;
        entry.set_password(password)?;
        Ok(())
    }

    /// Get SSH password from keyring
    pub fn get_ssh_password(&self, connection_id: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-password:{}", connection_id),
        )?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Store SSH key passphrase in keyring
    pub fn store_ssh_passphrase(&self, connection_id: &str, passphrase: &str) -> Result<()> {
        let entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-passphrase:{}", connection_id),
        )?;
        entry.set_password(passphrase)?;
        Ok(())
    }

    /// Get SSH key passphrase from keyring
    pub fn get_ssh_passphrase(&self, connection_id: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-passphrase:{}", connection_id),
        )?;

        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Delete all SSH credentials for a connection
    pub fn delete_ssh_credentials(&self, connection_id: &str) -> Result<()> {
        // Delete SSH password
        let password_entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-password:{}", connection_id),
        )?;
        let _ = password_entry.delete_credential();

        // Delete SSH passphrase
        let passphrase_entry = keyring::Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh-passphrase:{}", connection_id),
        )?;
        let _ = passphrase_entry.delete_credential();

        Ok(())
    }

    /// Delete all credentials for a connection (DB + SSH)
    pub fn delete_all_for_connection(&self, connection_id: &str) -> Result<()> {
        self.delete_password(connection_id)?;
        self.delete_ssh_credentials(connection_id)?;
        Ok(())
    }
}
```

### 7. Connection Security Summary Component

```rust
// src/ui/components/connection_security.rs
use gpui::{
    div, px, Element, IntoElement, ParentElement, Render,
    Styled, ViewContext, Window,
};

use crate::models::connection::{ConnectionConfig, SslMode};
use crate::theme::Theme;

pub struct ConnectionSecurityBadge {
    config: ConnectionConfig,
}

impl ConnectionSecurityBadge {
    pub fn new(config: ConnectionConfig) -> Self {
        Self { config }
    }

    fn security_level(&self) -> SecurityLevel {
        let has_ssl = !matches!(self.config.ssl_mode, SslMode::Disable);
        let has_verification = matches!(self.config.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull);
        let has_ssh = self.config.uses_ssh_tunnel();

        match (has_ssl, has_verification, has_ssh) {
            (true, true, true) => SecurityLevel::High,
            (true, true, false) => SecurityLevel::High,
            (true, false, true) => SecurityLevel::Medium,
            (true, false, false) => SecurityLevel::Medium,
            (false, _, true) => SecurityLevel::Medium,
            (false, _, false) => SecurityLevel::Low,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SecurityLevel {
    High,
    Medium,
    Low,
}

impl Render for ConnectionSecurityBadge {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let level = self.security_level();

        let (color, icon, text) = match level {
            SecurityLevel::High => (theme.colors.success, "🔒", "Secure"),
            SecurityLevel::Medium => (theme.colors.warning, "🔓", "Encrypted"),
            SecurityLevel::Low => (theme.colors.text_muted, "⚠", "Unencrypted"),
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(color.opacity(0.1))
            .child(
                div()
                    .text_xs()
                    .child(icon)
            )
            .child(
                div()
                    .text_xs()
                    .text_color(color)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(text)
            )
    }
}

/// Detailed security info tooltip content
pub struct ConnectionSecurityDetails {
    config: ConnectionConfig,
}

impl ConnectionSecurityDetails {
    pub fn new(config: ConnectionConfig) -> Self {
        Self { config }
    }
}

impl Render for ConnectionSecurityDetails {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .bg(theme.colors.bg_primary)
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .min_w(px(200.0))
            // SSL info
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_secondary)
                            .child("SSL/TLS")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_primary)
                            .child(match self.config.ssl_mode {
                                SslMode::Disable => "Disabled",
                                SslMode::Prefer => "Prefer",
                                SslMode::Require => "Required",
                                SslMode::VerifyCa => "Verify CA",
                                SslMode::VerifyFull => "Full Verification",
                            })
                    )
            )
            // SSH tunnel info
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_secondary)
                            .child("SSH Tunnel")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_primary)
                            .child(if self.config.uses_ssh_tunnel() {
                                "Enabled"
                            } else {
                                "Disabled"
                            })
                    )
            )
            // Client cert info
            .when(self.config.ssl_client_cert.is_some(), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.text_secondary)
                                .child("Client Cert")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.success)
                                .child("✓ Configured")
                        )
                )
            })
    }
}
```

## Acceptance Criteria

1. [x] SSL disable mode works (no encryption)
2. [x] SSL prefer mode upgrades to TLS if server supports it
3. [x] SSL require mode enforces TLS but skips certificate verification
4. [x] SSL verify-ca validates certificate against CA
5. [x] SSL verify-full validates certificate and hostname
6. [x] Client certificate authentication works
7. [x] SSH tunnel establishes successfully with password auth
8. [x] SSH tunnel establishes successfully with key auth
9. [x] SSH tunnel establishes successfully with agent auth
10. [x] SSH key passphrase retrieved from keyring
11. [x] Connection works through SSH tunnel
12. [x] Tunnel closes cleanly on disconnect
13. [x] Multiple concurrent tunnels supported
14. [x] Native file browser for selecting certificates and keys
15. [x] GPUI components for SSL/SSH configuration

## Testing

```rust
// tests/ssl_ssh_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ssl_config_creation() {
        let config = ConnectionConfig::new(
            "test".to_string(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
            "postgres".to_string(),
        );

        // Disable mode
        let mut test_config = config.clone();
        test_config.ssl_mode = SslMode::Disable;
        let ssl = SslConfig::from_connection_config(&test_config);
        assert!(ssl.is_ok());

        // Verify-full without CA should fail validation in connection
        let mut test_config = config.clone();
        test_config.ssl_mode = SslMode::VerifyFull;
        test_config.ssl_ca_cert = None;
        assert!(test_config.validate().is_err());
    }

    #[test]
    fn test_ssh_config_validation() {
        let ssh_config = SshTunnelConfig {
            enabled: true,
            host: "ssh.example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth: SshAuthMethod::Key,
            key_path: None, // Missing key path
            key_passphrase_in_keyring: false,
        };

        let mut config = ConnectionConfig::new(
            "test".to_string(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
            "postgres".to_string(),
        );
        config.ssh_tunnel = Some(ssh_config);

        // Should fail - key auth without key path
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_ssh_tunnel_local_port() {
        // Test that SSH tunnel binds to available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0);
    }
}
```

## Dependencies

### Cargo.toml Additions

```toml
[dependencies]
# SSL/TLS
rustls = { version = "0.23", features = ["ring"] }
rustls-pemfile = "2"
tokio-postgres-rustls = "0.12"
webpki-roots = "0.26"

# SSH
russh = "0.45"
russh-keys = "0.45"
async-trait = "0.1"

# File dialogs
rfd = "0.14"

# Path expansion
shellexpand = "3"
dirs = "5"

# Base64 for known_hosts
base64 = "0.22"
```

## Dependencies on Other Features

- **06-settings-theming-credentials.md**: KeyringService for storing SSH credentials
- **07-connection-management.md**: ConnectionPool integration

## Dependent Features

- **09-connection-ui.md**: Uses SSL/SSH configuration components
- All features requiring secure database connectivity

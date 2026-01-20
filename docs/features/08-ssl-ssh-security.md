# Feature 08: SSL/TLS & SSH Tunneling

## Overview

Implement secure connection options including SSL/TLS modes (disable, prefer, require, verify-ca, verify-full) and SSH tunneling via the russh library for connections over untrusted networks.

## Goals

- Support all Postgres SSL modes
- Implement certificate validation (CA, client certs)
- Create SSH tunnel service with russh
- Support SSH password and key authentication
- Handle SSH key passphrases from keyring

## Technical Specification

### 1. SSL/TLS Implementation

```rust
// services/ssl.rs
use std::path::Path;
use std::fs;
use native_tls::{Certificate, Identity, TlsConnector as NativeTlsConnector};
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Config as PgConfig;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, SslMode};

pub struct SslConfig {
    pub mode: SslMode,
    pub ca_cert: Option<Certificate>,
    pub client_identity: Option<Identity>,
}

impl SslConfig {
    pub fn from_connection_config(config: &ConnectionConfig) -> Result<Self> {
        let ca_cert = if let Some(ref ca_path) = config.ssl_ca_cert {
            Some(Self::load_ca_cert(ca_path)?)
        } else {
            None
        };

        let client_identity = if let (Some(ref cert_path), Some(ref key_path)) =
            (&config.ssl_client_cert, &config.ssl_client_key)
        {
            Some(Self::load_client_identity(cert_path, key_path)?)
        } else {
            None
        };

        Ok(Self {
            mode: config.ssl_mode.clone(),
            ca_cert,
            client_identity,
        })
    }

    fn load_ca_cert(path: &str) -> Result<Certificate> {
        let cert_data = fs::read(path).map_err(|e| {
            TuskError::SslError(format!("Failed to read CA certificate '{}': {}", path, e))
        })?;

        // Try PEM format first, then DER
        Certificate::from_pem(&cert_data)
            .or_else(|_| Certificate::from_der(&cert_data))
            .map_err(|e| TuskError::SslError(format!("Invalid CA certificate: {}", e)))
    }

    fn load_client_identity(cert_path: &str, key_path: &str) -> Result<Identity> {
        let cert_data = fs::read(cert_path).map_err(|e| {
            TuskError::SslError(format!("Failed to read client certificate '{}': {}", cert_path, e))
        })?;

        let key_data = fs::read(key_path).map_err(|e| {
            TuskError::SslError(format!("Failed to read client key '{}': {}", key_path, e))
        })?;

        // Combine cert and key into PKCS#12
        Identity::from_pkcs8(&cert_data, &key_data).map_err(|e| {
            TuskError::SslError(format!("Failed to create client identity: {}", e))
        })
    }

    pub fn create_tls_connector(&self) -> Result<MakeTlsConnector> {
        let mut builder = NativeTlsConnector::builder();

        match self.mode {
            SslMode::Disable => {
                return Err(TuskError::SslError(
                    "Cannot create TLS connector with SSL disabled".to_string()
                ));
            }
            SslMode::Prefer | SslMode::Require => {
                // Accept any certificate
                builder.danger_accept_invalid_certs(true);
                builder.danger_accept_invalid_hostnames(true);
            }
            SslMode::VerifyCa => {
                // Verify CA but not hostname
                if let Some(ref ca) = self.ca_cert {
                    builder.add_root_certificate(ca.clone());
                }
                builder.danger_accept_invalid_hostnames(true);
            }
            SslMode::VerifyFull => {
                // Full verification
                if let Some(ref ca) = self.ca_cert {
                    builder.add_root_certificate(ca.clone());
                }
            }
        }

        // Add client certificate if provided
        if let Some(ref identity) = self.client_identity {
            builder.identity(identity.clone());
        }

        let connector = builder.build().map_err(|e| {
            TuskError::SslError(format!("Failed to build TLS connector: {}", e))
        })?;

        Ok(MakeTlsConnector::new(connector))
    }
}
```

### 2. SSH Tunnel Service

```rust
// services/ssh.rs
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use russh::client::{self, Handle};
use russh_keys::key::PrivateKeyWithHashAlg;
use async_trait::async_trait;

use crate::error::{Result, TuskError};
use crate::models::connection::{SshTunnelConfig, SshAuthMethod};
use crate::services::keyring::KeyringService;

pub struct SshTunnel {
    local_port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SshTunnel {
    pub async fn new(
        config: &SshTunnelConfig,
        connection_id: &str,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<Self> {
        // Find an available local port
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            TuskError::SshError(format!("Failed to bind local port: {}", e))
        })?;
        let local_port = listener.local_addr()?.port();

        // Create SSH client config
        let ssh_config = Arc::new(client::Config::default());

        // Connect to SSH server
        let ssh_addr = format!("{}:{}", config.host, config.port);
        let mut session = client::connect(ssh_config, &ssh_addr, SshHandler)
            .await
            .map_err(|e| TuskError::SshError(format!("SSH connection failed: {}", e)))?;

        // Authenticate
        Self::authenticate(&mut session, config, connection_id).await?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn tunnel task
        let remote_host = remote_host.to_string();
        let handle = session.handle();

        let task_handle = tokio::spawn(async move {
            Self::run_tunnel(listener, handle, remote_host, remote_port, shutdown_rx).await;
        });

        Ok(Self {
            local_port,
            shutdown_tx: Some(shutdown_tx),
            task_handle: Some(task_handle),
        })
    }

    async fn authenticate(
        session: &mut client::Handle<SshHandler>,
        config: &SshTunnelConfig,
        connection_id: &str,
    ) -> Result<()> {
        match config.auth {
            SshAuthMethod::Password => {
                let password = if config.passphrase_in_keyring {
                    KeyringService::get_password(&format!("ssh:{}", connection_id))?
                        .ok_or_else(|| TuskError::CredentialNotFound(
                            format!("SSH password for {}", connection_id)
                        ))?
                } else {
                    return Err(TuskError::SshError(
                        "SSH password not available".to_string()
                    ));
                };

                let auth_result = session
                    .authenticate_password(&config.username, &password)
                    .await
                    .map_err(|e| TuskError::SshError(format!("SSH password auth failed: {}", e)))?;

                if !auth_result {
                    return Err(TuskError::AuthenticationFailed(
                        "SSH password authentication rejected".to_string()
                    ));
                }
            }
            SshAuthMethod::Key => {
                let key_path = config.key_path.as_ref().ok_or_else(|| {
                    TuskError::SshError("SSH key path not specified".to_string())
                })?;

                // Load private key
                let key_data = tokio::fs::read(key_path).await.map_err(|e| {
                    TuskError::SshError(format!("Failed to read SSH key '{}': {}", key_path, e))
                })?;

                // Get passphrase if key is encrypted
                let passphrase = if config.passphrase_in_keyring {
                    KeyringService::get_ssh_passphrase(&format!("{}", connection_id))?
                } else {
                    None
                };

                let key_pair = if let Some(pass) = passphrase {
                    russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), Some(&pass))
                } else {
                    russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), None)
                }
                .map_err(|e| TuskError::SshError(format!("Failed to decode SSH key: {}", e)))?;

                let auth_result = session
                    .authenticate_publickey(
                        &config.username,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), None).unwrap(),
                    )
                    .await
                    .map_err(|e| TuskError::SshError(format!("SSH key auth failed: {}", e)))?;

                if !auth_result {
                    return Err(TuskError::AuthenticationFailed(
                        "SSH public key authentication rejected".to_string()
                    ));
                }
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
    ) {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    tracing::info!("SSH tunnel shutdown requested");
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((local_stream, _)) => {
                            let handle = ssh_handle.clone();
                            let host = remote_host.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    local_stream, handle, &host, remote_port
                                ).await {
                                    tracing::warn!("Tunnel connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
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
                    tracing::debug!("Client to server copy error: {}", e);
                }
            }
            r = server_to_client => {
                if let Err(e) = r {
                    tracing::debug!("Server to client copy error: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn close(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}

// SSH client handler
struct SshHandler;

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        // TODO: Implement proper host key verification
        // For now, accept all keys (like StrictHostKeyChecking=no)
        Ok(true)
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
```

### 3. Updated Connection Pool with SSL/SSH

```rust
// services/connection.rs (updated)
use crate::services::ssl::SslConfig;
use crate::services::ssh::SshTunnel;

pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
    ssh_tunnel: Option<SshTunnel>,
}

impl ConnectionPool {
    pub async fn new(config: ConnectionConfig) -> Result<Self> {
        // Set up SSH tunnel if configured
        let (effective_host, effective_port, ssh_tunnel) = if let Some(ref ssh) = config.ssh_tunnel {
            if ssh.enabled {
                tracing::info!("Creating SSH tunnel to {}:{}", ssh.host, ssh.port);

                let tunnel = SshTunnel::new(
                    ssh,
                    &config.id.to_string(),
                    &config.host,
                    config.port,
                ).await?;

                let local_port = tunnel.local_port();
                tracing::info!("SSH tunnel established on local port {}", local_port);

                ("127.0.0.1".to_string(), local_port, Some(tunnel))
            } else {
                (config.host.clone(), config.port, None)
            }
        } else {
            (config.host.clone(), config.port, None)
        };

        // Get password from keyring if needed
        let password = if config.password_in_keyring {
            KeyringService::get_password(&config.id.to_string())?
                .ok_or_else(|| TuskError::CredentialNotFound(config.id.to_string()))?
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

        // Create pool with appropriate TLS
        let pool = match config.ssl_mode {
            SslMode::Disable => {
                Self::create_pool_no_tls(pg_config, &config.options).await?
            }
            _ => {
                let ssl_config = SslConfig::from_connection_config(&config)?;
                Self::create_pool_with_tls(pg_config, &config.options, ssl_config).await?
            }
        };

        let status = Arc::new(RwLock::new(ConnectionStatus::Connected));

        let mut conn_pool = Self {
            pool,
            config,
            status,
            keepalive_handle: None,
            ssh_tunnel,
        };

        conn_pool.start_keepalive();

        Ok(conn_pool)
    }

    async fn create_pool_with_tls(
        pg_config: PgConfig,
        options: &ConnectionOptions,
        ssl_config: SslConfig,
    ) -> Result<Pool> {
        let tls_connector = ssl_config.create_tls_connector()?;

        let mut pool_config = Config::new();
        pool_config.host = Some(pg_config.get_hosts()[0].to_string());
        pool_config.port = pg_config.get_ports().first().copied();
        pool_config.dbname = pg_config.get_dbname().map(|s| s.to_string());
        pool_config.user = pg_config.get_user().map(|s| s.to_string());

        let pool_cfg = PoolConfig {
            max_size: 10,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(options.connect_timeout_sec)),
                create: Some(Duration::from_secs(options.connect_timeout_sec)),
                recycle: Some(Duration::from_secs(30)),
            },
            ..Default::default()
        };

        pool_config.pool = Some(pool_cfg);

        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1), tls_connector)
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to create TLS connection pool: {}", e),
                source: Some(Box::new(e)),
            })?;

        // Test the connection
        let client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to connect with TLS: {}", e),
            source: None,
        })?;

        if options.readonly {
            client.execute("SET default_transaction_read_only = ON", &[]).await?;
        }

        Ok(pool)
    }

    pub async fn close(&mut self) {
        if let Some(handle) = self.keepalive_handle.take() {
            handle.abort();
        }
        *self.status.write().await = ConnectionStatus::Disconnected;
        self.pool.close();

        // Close SSH tunnel
        if let Some(tunnel) = self.ssh_tunnel.take() {
            tunnel.close().await;
        }
    }
}
```

### 4. Connection Dialog SSL/SSH Fields

```svelte
<!-- components/dialogs/ConnectionDialog.svelte (SSL section) -->
<script lang="ts">
	// ... existing code ...

	const sslModes = [
		{ value: 'disable', label: 'Disable', description: 'No SSL' },
		{ value: 'prefer', label: 'Prefer', description: 'Use SSL if available' },
		{ value: 'require', label: 'Require', description: 'Require SSL, skip verification' },
		{ value: 'verify-ca', label: 'Verify CA', description: 'Verify server certificate' },
		{ value: 'verify-full', label: 'Verify Full', description: 'Verify certificate and hostname' }
	];
</script>

<!-- SSL Tab -->
<div class="space-y-4">
	<FormField label="SSL Mode">
		<Select options={sslModes} bind:value={config.sslMode} />
	</FormField>

	{#if config.sslMode === 'verify-ca' || config.sslMode === 'verify-full'}
		<FormField label="CA Certificate">
			<div class="flex gap-2">
				<Input
					type="text"
					bind:value={config.sslCaCert}
					placeholder="/path/to/ca.crt"
					class="flex-1"
				/>
				<Button variant="secondary" onclick={browseCaCert}>Browse</Button>
			</div>
		</FormField>
	{/if}

	<FormField label="Client Certificate (optional)">
		<div class="flex gap-2">
			<Input
				type="text"
				bind:value={config.sslClientCert}
				placeholder="/path/to/client.crt"
				class="flex-1"
			/>
			<Button variant="secondary" onclick={browseClientCert}>Browse</Button>
		</div>
	</FormField>

	<FormField label="Client Key (optional)">
		<div class="flex gap-2">
			<Input
				type="text"
				bind:value={config.sslClientKey}
				placeholder="/path/to/client.key"
				class="flex-1"
			/>
			<Button variant="secondary" onclick={browseClientKey}>Browse</Button>
		</div>
	</FormField>
</div>

<!-- SSH Tunnel Tab -->
<div class="space-y-4">
	<Checkbox bind:checked={config.sshTunnel.enabled} label="Use SSH Tunnel" />

	{#if config.sshTunnel?.enabled}
		<div class="grid grid-cols-2 gap-4">
			<FormField label="SSH Host">
				<Input
					type="text"
					bind:value={config.sshTunnel.host}
					placeholder="ssh.example.com"
					required
				/>
			</FormField>

			<FormField label="SSH Port">
				<Input type="number" bind:value={config.sshTunnel.port} min="1" max="65535" />
			</FormField>
		</div>

		<FormField label="SSH Username">
			<Input type="text" bind:value={config.sshTunnel.username} required />
		</FormField>

		<FormField label="Authentication Method">
			<Select
				options={[
					{ value: 'password', label: 'Password' },
					{ value: 'key', label: 'SSH Key' }
				]}
				bind:value={config.sshTunnel.auth}
			/>
		</FormField>

		{#if config.sshTunnel.auth === 'password'}
			<FormField label="SSH Password">
				<Input type="password" bind:value={sshPassword} autocomplete="off" />
				<p class="text-xs text-gray-500 mt-1">Stored securely in your system keychain</p>
			</FormField>
		{:else}
			<FormField label="SSH Key">
				<div class="flex gap-2">
					<Input
						type="text"
						bind:value={config.sshTunnel.keyPath}
						placeholder="~/.ssh/id_rsa"
						class="flex-1"
					/>
					<Button variant="secondary" onclick={browseSshKey}>Browse</Button>
				</div>
			</FormField>

			<FormField label="Key Passphrase (if encrypted)">
				<Input type="password" bind:value={sshPassphrase} autocomplete="off" />
			</FormField>
		{/if}
	{/if}
</div>
```

### 5. File Browser Integration

```typescript
// services/dialog.ts
import { open } from '@tauri-apps/plugin-dialog';

export async function browseFile(options?: {
	title?: string;
	filters?: { name: string; extensions: string[] }[];
	defaultPath?: string;
}): Promise<string | null> {
	const result = await open({
		title: options?.title || 'Select File',
		filters: options?.filters,
		defaultPath: options?.defaultPath,
		multiple: false,
		directory: false
	});

	return result as string | null;
}

export async function browseCertificate(): Promise<string | null> {
	return browseFile({
		title: 'Select Certificate',
		filters: [
			{ name: 'Certificates', extensions: ['crt', 'pem', 'cer'] },
			{ name: 'All Files', extensions: ['*'] }
		]
	});
}

export async function browseSshKey(): Promise<string | null> {
	return browseFile({
		title: 'Select SSH Key',
		filters: [
			{ name: 'SSH Keys', extensions: ['pem', 'key'] },
			{ name: 'All Files', extensions: ['*'] }
		],
		defaultPath: '~/.ssh'
	});
}
```

## Acceptance Criteria

1. [ ] SSL disable mode works (no encryption)
2. [ ] SSL prefer mode upgrades to TLS if server supports it
3. [ ] SSL require mode enforces TLS but skips certificate verification
4. [ ] SSL verify-ca validates certificate against CA
5. [ ] SSL verify-full validates certificate and hostname
6. [ ] Client certificate authentication works
7. [ ] SSH tunnel establishes successfully with password auth
8. [ ] SSH tunnel establishes successfully with key auth
9. [ ] SSH key passphrase retrieved from keyring
10. [ ] Connection works through SSH tunnel
11. [ ] Tunnel closes cleanly on disconnect
12. [ ] Multiple concurrent tunnels supported
13. [ ] File browser allows selecting certificates and keys

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Create connection with SSL:
   ipc_execute_command command="save_connection" args={
     config: { sslMode: "require", ... }
   }
4. Connect and verify TLS:
   ipc_execute_command command="connect"
5. Test SSH tunnel:
   ipc_execute_command command="save_connection" args={
     config: { sshTunnel: { enabled: true, ... }, ... }
   }
6. Connect through tunnel:
   ipc_execute_command command="connect"
7. Verify tunnel port in logs
```

## Dependencies on Other Features

- 06-settings-theming-credentials.md
- 07-connection-management.md

## Dependent Features

- 09-connection-ui.md

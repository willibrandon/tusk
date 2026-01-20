// SSH Tunnel service for secure remote connections

use crate::error::{TuskError, TuskResult};
use crate::models::{ConnectionConfig, SshAuthMethod, SshTunnel};
use async_trait::async_trait;
use russh::client;
use russh_keys::agent::client::AgentClient;
use russh_keys::key::KeyPair;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Handle to an active SSH tunnel.
pub struct SshTunnelHandle {
    /// Connection ID this tunnel belongs to
    pub connection_id: Uuid,
    /// Local port the tunnel is listening on
    pub local_port: u16,
    /// Remote host being tunneled to
    pub remote_host: String,
    /// Remote port being tunneled to
    pub remote_port: u16,
    /// Cancellation token to stop the tunnel
    cancel_token: CancellationToken,
}

impl SshTunnelHandle {
    /// Stop the SSH tunnel.
    pub fn stop(&self) {
        self.cancel_token.cancel();
        tracing::info!(
            "SSH tunnel stopped for connection {} (local port {})",
            self.connection_id,
            self.local_port
        );
    }
}

impl Drop for SshTunnelHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

/// SSH client handler for russh.
struct SshClientHandler;

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys for now
        // In a production environment, implement known_hosts verification
        Ok(true)
    }
}

/// SSH Tunnel service for establishing tunnels.
pub struct SshTunnelService;

impl SshTunnelService {
    /// Establish an SSH tunnel for a database connection.
    ///
    /// # Arguments
    ///
    /// * `config` - The connection configuration with SSH tunnel settings
    /// * `ssh_password` - The SSH password if using password authentication (from keychain)
    /// * `key_passphrase` - The passphrase for the SSH key if using key authentication (from keychain)
    ///
    /// # Returns
    ///
    /// Returns a handle to the active tunnel with the local port to connect to.
    pub async fn establish(
        config: &ConnectionConfig,
        ssh_password: Option<&str>,
        key_passphrase: Option<&str>,
    ) -> TuskResult<SshTunnelHandle> {
        let tunnel_config = config.ssh_tunnel.as_ref().ok_or_else(|| {
            TuskError::connection("No SSH tunnel configured for this connection")
        })?;

        tracing::info!(
            "Establishing SSH tunnel to {}:{} for connection {}",
            tunnel_config.host,
            tunnel_config.port,
            config.id
        );

        // Connect to SSH server
        let ssh_client = Self::connect_ssh(tunnel_config, ssh_password, key_passphrase).await?;
        let ssh_client = Arc::new(Mutex::new(ssh_client));

        // Find a local port for the tunnel
        let local_port = tunnel_config.local_port.unwrap_or(0);
        let listener = TcpListener::bind(format!("127.0.0.1:{}", local_port))
            .await
            .map_err(|e| {
                TuskError::connection_with_hint(
                    format!("Failed to bind local port for SSH tunnel: {}", e),
                    "Try a different local port or let the system choose one automatically",
                )
            })?;

        let actual_local_port = listener.local_addr()?.port();
        tracing::info!(
            "SSH tunnel listening on local port {} for connection {}",
            actual_local_port,
            config.id
        );

        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        // Remote host/port to tunnel to (the PostgreSQL server)
        let remote_host = config.host.clone();
        let remote_port = config.port;
        let connection_id = config.id;

        // Spawn the tunnel forwarding task
        tokio::spawn(async move {
            Self::run_tunnel(
                listener,
                ssh_client,
                remote_host.clone(),
                remote_port,
                connection_id,
                cancel_token_clone,
            )
            .await;
        });

        Ok(SshTunnelHandle {
            connection_id: config.id,
            local_port: actual_local_port,
            remote_host: config.host.clone(),
            remote_port: config.port,
            cancel_token,
        })
    }

    /// Connect to the SSH server.
    async fn connect_ssh(
        tunnel_config: &SshTunnel,
        ssh_password: Option<&str>,
        key_passphrase: Option<&str>,
    ) -> TuskResult<client::Handle<SshClientHandler>> {
        let config = client::Config::default();
        let config = Arc::new(config);

        let addr = format!("{}:{}", tunnel_config.host, tunnel_config.port);
        let addr: SocketAddr = addr.parse().map_err(|e| {
            TuskError::connection_with_hint(
                format!("Invalid SSH server address: {}", e),
                "Check the SSH host and port settings",
            )
        })?;

        let mut handle = client::connect(config, addr, SshClientHandler)
            .await
            .map_err(|e| {
                TuskError::connection_with_hint(
                    format!("Failed to connect to SSH server: {}", e),
                    "Verify the SSH server is running and reachable",
                )
            })?;

        // Authenticate based on the configured method
        let authenticated = match &tunnel_config.auth_method {
            SshAuthMethod::Password => {
                Self::authenticate_password(&mut handle, &tunnel_config.username, ssh_password)
                    .await?
            }
            SshAuthMethod::KeyFile { path } => {
                Self::authenticate_key_file(
                    &mut handle,
                    &tunnel_config.username,
                    path,
                    key_passphrase,
                )
                .await?
            }
            SshAuthMethod::Agent => {
                Self::authenticate_with_agent(&mut handle, &tunnel_config.username).await?
            }
        };

        if !authenticated {
            return Err(TuskError::connection_with_hint(
                "SSH authentication failed",
                "Check your credentials and try again",
            ));
        }

        tracing::info!(
            "SSH authentication successful for {}@{}",
            tunnel_config.username,
            tunnel_config.host
        );

        Ok(handle)
    }

    /// Authenticate using password.
    async fn authenticate_password(
        handle: &mut client::Handle<SshClientHandler>,
        username: &str,
        password: Option<&str>,
    ) -> TuskResult<bool> {
        let password = password.ok_or_else(|| TuskError::credential("SSH password not provided"))?;

        handle
            .authenticate_password(username, password)
            .await
            .map_err(|e| {
                TuskError::connection_with_hint(
                    format!("SSH password authentication failed: {}", e),
                    "Check your SSH username and password",
                )
            })
    }

    /// Authenticate using a private key file.
    async fn authenticate_key_file(
        handle: &mut client::Handle<SshClientHandler>,
        username: &str,
        path: &str,
        passphrase: Option<&str>,
    ) -> TuskResult<bool> {
        let key = Self::load_private_key(path, passphrase).await?;

        handle
            .authenticate_publickey(username, key)
            .await
            .map_err(|e| {
                TuskError::connection_with_hint(
                    format!("SSH key authentication failed: {}", e),
                    "Verify your SSH key is valid and has the correct permissions",
                )
            })
    }

    /// Load a private key from a file.
    async fn load_private_key(path: &str, passphrase: Option<&str>) -> TuskResult<Arc<KeyPair>> {
        let key_data = tokio::fs::read(path).await.map_err(|e| {
            TuskError::connection_with_hint(
                format!("Failed to read SSH key file: {}", e),
                "Check that the key file exists and is readable",
            )
        })?;

        let key = if let Some(passphrase) = passphrase {
            russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), Some(passphrase))
                .map_err(|e| {
                    TuskError::connection_with_hint(
                        format!("Failed to decrypt SSH key: {}", e),
                        "Check your key passphrase is correct",
                    )
                })?
        } else {
            russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), None).map_err(
                |e| {
                    TuskError::connection_with_hint(
                        format!("Failed to parse SSH key: {}", e),
                        "The key may be encrypted - provide the passphrase",
                    )
                },
            )?
        };

        Ok(Arc::new(key))
    }

    /// Authenticate using the SSH agent.
    /// Uses russh's built-in Signer trait implementation for AgentClient.
    #[cfg(unix)]
    async fn authenticate_with_agent(
        handle: &mut client::Handle<SshClientHandler>,
        username: &str,
    ) -> TuskResult<bool> {
        use tokio::net::UnixStream;

        let socket_path = std::env::var("SSH_AUTH_SOCK").map_err(|_| {
            TuskError::connection_with_hint(
                "SSH agent not available (SSH_AUTH_SOCK not set)",
                "Start your SSH agent with: eval $(ssh-agent)",
            )
        })?;

        let stream = UnixStream::connect(&socket_path).await.map_err(|e| {
            TuskError::connection_with_hint(
                format!("Failed to connect to SSH agent: {}", e),
                "Ensure your SSH agent is running",
            )
        })?;

        let mut agent = AgentClient::connect(stream);
        let identities = agent.request_identities().await.map_err(|e| {
            TuskError::connection_with_hint(
                format!("Failed to get identities from SSH agent: {}", e),
                "Ensure your SSH agent is running and has keys loaded",
            )
        })?;

        if identities.is_empty() {
            return Err(TuskError::connection_with_hint(
                "No identities available in SSH agent",
                "Add your SSH key to the agent with: ssh-add",
            ));
        }

        tracing::debug!("Found {} identities in SSH agent", identities.len());

        // Try each identity until one works
        // Use authenticate_future with the AgentClient as the Signer
        for identity in identities {
            tracing::debug!("Trying SSH agent identity: {:?}", identity.name());

            // authenticate_future takes a Signer (AgentClient implements Signer)
            // and the public key to authenticate with
            // It returns a tuple: (agent, Result<bool, AgentAuthError>)
            let (returned_agent, auth_result) = handle
                .authenticate_future(username, identity.clone(), agent)
                .await;

            agent = returned_agent;
            match auth_result {
                Ok(true) => {
                    tracing::info!("SSH agent authentication successful");
                    return Ok(true);
                }
                Ok(false) => {
                    tracing::debug!("SSH agent key rejected, trying next");
                    continue;
                }
                Err(e) => {
                    tracing::debug!("SSH agent auth error: {}, trying next", e);
                    continue;
                }
            }
        }

        // No identity worked
        Ok(false)
    }

    /// Authenticate using the SSH agent (non-Unix platforms).
    /// Windows uses a different mechanism for SSH agent communication.
    #[cfg(windows)]
    async fn authenticate_with_agent(
        handle: &mut client::Handle<SshClientHandler>,
        username: &str,
    ) -> TuskResult<bool> {
        use tokio::net::windows::named_pipe::ClientOptions;

        // Windows OpenSSH agent uses a named pipe
        let pipe_name = r"\\.\pipe\openssh-ssh-agent";

        let pipe = ClientOptions::new()
            .open(pipe_name)
            .map_err(|e| {
                TuskError::connection_with_hint(
                    format!("Failed to connect to SSH agent: {}", e),
                    "Ensure the OpenSSH Authentication Agent service is running",
                )
            })?;

        let mut agent = AgentClient::connect(pipe);
        let identities = agent.request_identities().await.map_err(|e| {
            TuskError::connection_with_hint(
                format!("Failed to get identities from SSH agent: {}", e),
                "Ensure the OpenSSH Authentication Agent service is running and has keys loaded",
            )
        })?;

        if identities.is_empty() {
            return Err(TuskError::connection_with_hint(
                "No identities available in SSH agent",
                "Add your SSH key to the agent with: ssh-add",
            ));
        }

        tracing::debug!("Found {} identities in SSH agent", identities.len());

        // Try each identity until one works
        for identity in identities {
            tracing::debug!("Trying SSH agent identity: {:?}", identity.name());

            // authenticate_future returns a tuple: (agent, Result<bool, AgentAuthError>)
            let (returned_agent, auth_result) = handle
                .authenticate_future(username, identity.clone(), agent)
                .await;

            agent = returned_agent;
            match auth_result {
                Ok(true) => {
                    tracing::info!("SSH agent authentication successful");
                    return Ok(true);
                }
                Ok(false) => {
                    tracing::debug!("SSH agent key rejected, trying next");
                    continue;
                }
                Err(e) => {
                    tracing::debug!("SSH agent auth error: {}, trying next", e);
                    continue;
                }
            }
        }

        Ok(false)
    }

    /// Run the tunnel forwarding loop.
    async fn run_tunnel(
        listener: TcpListener,
        ssh_client: Arc<Mutex<client::Handle<SshClientHandler>>>,
        remote_host: String,
        remote_port: u16,
        connection_id: Uuid,
        cancel_token: CancellationToken,
    ) {
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((local_stream, peer_addr)) => {
                            tracing::debug!(
                                "New tunnel connection from {} for connection {}",
                                peer_addr,
                                connection_id
                            );

                            let ssh_client = ssh_client.clone();
                            let remote_host = remote_host.clone();
                            let cancel = cancel_token.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::forward_connection(
                                    local_stream,
                                    ssh_client,
                                    &remote_host,
                                    remote_port,
                                    cancel,
                                )
                                .await
                                {
                                    tracing::warn!("Tunnel forwarding error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept tunnel connection: {}", e);
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("SSH tunnel cancelled for connection {}", connection_id);
                    break;
                }
            }
        }
    }

    /// Forward a single connection through the SSH tunnel.
    async fn forward_connection(
        mut local_stream: TcpStream,
        ssh_client: Arc<Mutex<client::Handle<SshClientHandler>>>,
        remote_host: &str,
        remote_port: u16,
        cancel_token: CancellationToken,
    ) -> TuskResult<()> {
        // Open a direct-tcpip channel to the remote host
        let channel = {
            let handle = ssh_client.lock().await;
            handle
                .channel_open_direct_tcpip(remote_host, remote_port as u32, "127.0.0.1", 0)
                .await
                .map_err(|e| {
                    TuskError::connection_with_hint(
                        format!("Failed to open SSH channel: {}", e),
                        "The SSH server may not allow TCP forwarding",
                    )
                })?
        };

        let mut channel = channel.into_stream();
        let mut buf_local = [0u8; 8192];
        let mut buf_remote = [0u8; 8192];

        loop {
            tokio::select! {
                // Read from local and write to remote
                n = local_stream.read(&mut buf_local) => {
                    match n {
                        Ok(0) => break, // Connection closed
                        Ok(n) => {
                            if channel.write_all(&buf_local[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                // Read from remote and write to local
                n = channel.read(&mut buf_remote) => {
                    match n {
                        Ok(0) => break, // Connection closed
                        Ok(n) => {
                            if local_stream.write_all(&buf_remote[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                _ = cancel_token.cancelled() => {
                    break;
                }
            }
        }

        Ok(())
    }
}

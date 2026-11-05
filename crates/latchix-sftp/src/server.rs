use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use latchix_core::{FsError, RootId, TenantContext, TenantId};
use russh::Channel;
use russh::ChannelId;
use russh::keys::{HashAlg, PrivateKey, PublicKey};
use russh::server::{Auth, Msg, Server as _, Session, VerifiedPublicKey};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::handler::SftpHandler;
use crate::local_fs::LocalFs;

/// Configuration for a development or test SFTP server.
pub struct SftpServerConfig {
    /// TCP bind address. Use `127.0.0.1:0` in tests.
    pub bind: SocketAddr,
    /// Server host keys. RSA is rejected before this point.
    pub host_keys: Vec<PrivateKey>,
    /// Authorized client public keys.
    pub authorized_keys: Vec<PublicKey>,
    /// Tenant root directory.
    pub root: PathBuf,
    /// Tenant identity for this server.
    pub tenant_id: TenantId,
    /// Root identity for this server.
    pub root_id: RootId,
}

/// Server startup error.
#[derive(Debug, thiserror::Error)]
pub enum SftpServerError {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Filesystem setup error.
    #[error("filesystem error: {0}")]
    Fs(#[from] FsError),
    /// SSH error.
    #[error("ssh error: {0}")]
    Russh(#[from] russh::Error),
    /// Server task join error.
    #[error("server task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Handle to a running SFTP server.
pub struct ServerHandle {
    local_addr: SocketAddr,
    handle: russh::server::RunningServerHandle,
    join: JoinHandle<std::io::Result<()>>,
}

impl ServerHandle {
    /// Returns the bound address.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops the server and waits for the listener task.
    pub async fn shutdown(self) {
        self.handle.shutdown("latchix shutdown".to_string());
        let _ = self.join.await;
    }
}

/// Starts an SFTP server for development and tests.
pub async fn spawn(config: SftpServerConfig) -> Result<ServerHandle, SftpServerError> {
    if config.host_keys.is_empty() {
        return Err(SftpServerError::Io(std::io::Error::other(
            "at least one host key is required",
        )));
    }

    let fs = Arc::new(LocalFs::new(&config.root)?);
    let authorized: HashSet<String> = config.authorized_keys.iter().map(fingerprint).collect();

    let listener = TcpListener::bind(config.bind).await?;
    let local_addr = listener.local_addr()?;
    let ssh_config = Arc::new(russh::server::Config {
        keys: config.host_keys,
        auth_rejection_time: Duration::from_millis(250),
        auth_rejection_time_initial: Some(Duration::ZERO),
        inactivity_timeout: Some(Duration::from_secs(300)),
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    });

    let mut server = SftpServer {
        fs,
        tenant: TenantContext::new(config.tenant_id, config.root_id),
        authorized: Arc::new(authorized),
        next_session: AtomicU64::new(1),
    };
    let (handle_tx, handle_rx) = oneshot::channel();
    let join = tokio::spawn(async move {
        let running = server.run_on_socket(ssh_config, &listener);
        let handle = running.handle();
        let _ = handle_tx.send(handle);
        running.await
    });
    let handle = handle_rx
        .await
        .map_err(|_| SftpServerError::Io(std::io::Error::other("server task ended")))?;

    Ok(ServerHandle {
        local_addr,
        handle,
        join,
    })
}

struct SftpServer {
    fs: Arc<LocalFs>,
    tenant: TenantContext,
    authorized: Arc<HashSet<String>>,
    next_session: AtomicU64,
}

impl russh::server::Server for SftpServer {
    type Handler = SshSession;

    fn new_client(&mut self, peer: Option<SocketAddr>) -> Self::Handler {
        let session_id = self.next_session.fetch_add(1, Ordering::Relaxed);
        info!(session = session_id, ?peer, "ssh client connected");
        SshSession {
            fs: Arc::clone(&self.fs),
            tenant: self.tenant,
            authorized: Arc::clone(&self.authorized),
            peer,
            session_id,
            authenticated_user: None,
            channels: HashMap::new(),
        }
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        error!(?error, "ssh session error");
    }
}

struct SshSession {
    fs: Arc<LocalFs>,
    tenant: TenantContext,
    authorized: Arc<HashSet<String>>,
    peer: Option<SocketAddr>,
    session_id: u64,
    authenticated_user: Option<String>,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

impl russh::server::Handler for SshSession {
    type Error = russh::Error;

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        let fingerprint = fingerprint(key.public_key);
        if self.authorized.contains(&fingerprint) {
            self.authenticated_user = Some(user.to_string());
            info!(
                session = self.session_id,
                user, fingerprint, "public key accepted"
            );
            Ok(Auth::Accept)
        } else {
            info!(
                session = self.session_id,
                user, fingerprint, "public key rejected"
            );
            Ok(Auth::reject())
        }
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::reject())
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != "sftp" || self.authenticated_user.is_none() {
            session.channel_failure(channel_id)?;
            return Ok(());
        }

        let Some(channel) = self.channels.remove(&channel_id) else {
            session.channel_failure(channel_id)?;
            return Ok(());
        };

        session.channel_success(channel_id)?;
        let handler = SftpHandler::new(
            Arc::clone(&self.fs) as Arc<dyn latchix_core::VirtualFs>,
            self.tenant,
            self.session_id,
            self.peer,
        );
        info!(
            session = self.session_id,
            tenant = %self.tenant.tenant_id(),
            "sftp subsystem started"
        );
        russh_sftp::server::run_with_config(
            channel.into_stream(),
            handler,
            russh_sftp::server::Config {
                max_client_packet_len: 256 * 1024,
            },
        )
        .await;
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.close(channel)?;
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.remove(&channel);
        Ok(())
    }
}

fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

#[cfg(test)]
mod tests {
    use super::{SftpServerConfig, SftpServerError, spawn};
    use latchix_core::{RootId, TenantId};

    #[tokio::test]
    async fn spawn_requires_a_host_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let error = match spawn(SftpServerConfig {
            bind: "127.0.0.1:0".parse().expect("bind"),
            host_keys: Vec::new(),
            authorized_keys: Vec::new(),
            root: temp.path().to_path_buf(),
            tenant_id: TenantId::new(),
            root_id: RootId::new(),
        })
        .await
        {
            Ok(_) => panic!("empty host key list must fail"),
            Err(error) => error,
        };

        assert!(matches!(error, SftpServerError::Io(_)));
    }
}

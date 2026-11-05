use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use tokio::net::{TcpListener, UnixListener};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::cli::ServeArgs;
use crate::config::{Config, ConfigError};
use crate::error::AppError;

/// Starts the HTTP health listener.
///
/// The SSH/SFTP listener arrives in Phase 1. The health listener exists now so
/// the binary, systemd unit, and container have a real process to start.
pub async fn run(config_path: &Path, args: ServeArgs) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    config.ensure_directories()?;
    fail_on_bad_paths(&config)?;

    if config.ui.enabled {
        #[cfg(feature = "ui")]
        info!(feature = latchix_ui::FEATURE_NAME, "admin ui enabled");
        #[cfg(not(feature = "ui"))]
        warn!("ui.enabled is true but this build does not include the ui feature");
    }

    warn!(
        bind = %config.server.bind,
        "SSH/SFTP listener is not implemented in Phase 0"
    );

    let app = latchix_api::router(latchix_api::AppState::new(env!("CARGO_PKG_VERSION"), true));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut tasks: Vec<JoinHandle<std::io::Result<()>>> = Vec::new();

    if let Some(bind) = &config.api.tcp {
        let address = bind
            .parse::<SocketAddr>()
            .map_err(|error| AppError::Config(ConfigError::Invalid(format!("api.tcp: {error}"))))?;
        let listener = TcpListener::bind(address).await?;
        let local_address = listener.local_addr()?;
        info!(address = %local_address, "http health listener started");
        let app = app.clone();
        let shutdown_rx = shutdown_rx.clone();
        tasks.push(tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
                .await
        }));
    }

    if let Some(path) = &config.api.unix_socket {
        prepare_unix_socket(path).await?;
        let listener = UnixListener::bind(path)?;
        set_socket_permissions(path)?;
        info!(path = %path.display(), "http health listener started");
        let app = app.clone();
        let shutdown_rx = shutdown_rx.clone();
        tasks.push(tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
                .await
        }));
    }

    if tasks.is_empty() {
        return Err(AppError::Config(ConfigError::Invalid(
            "api.tcp or api.unix_socket must be set".into(),
        )));
    }

    shutdown_signal().await;
    let _ = shutdown_tx.send(true);

    let grace = Duration::from_secs(args.shutdown_grace_seconds.max(1));
    for mut task in tasks {
        match tokio::time::timeout(grace, &mut task).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => return Err(AppError::Serve(error.to_string())),
            Ok(Err(error)) => return Err(AppError::Serve(error.to_string())),
            Err(_) => {
                task.abort();
                return Err(AppError::Serve("graceful shutdown timed out".into()));
            }
        }
    }

    info!("shutdown complete");
    Ok(())
}

fn fail_on_bad_paths(config: &Config) -> Result<(), AppError> {
    let failures: Vec<String> = config
        .path_statuses()
        .into_iter()
        .filter(|status| !status.ok)
        .map(|status| format!("{}: {}", status.label, status.detail))
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Config(ConfigError::Invalid(failures.join(", "))))
    }
}

async fn wait_for_shutdown(mut receiver: watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    let _ = receiver.changed().await;
}

async fn prepare_unix_socket(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        match tokio::net::UnixStream::connect(path).await {
            Ok(_) => {
                return Err(AppError::Serve(format!(
                    "unix socket {} is already in use",
                    path.display()
                )));
            }
            Err(_) => {
                std::fs::remove_file(path)?;
            }
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[cfg(unix)]
fn set_socket_permissions(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_socket_permissions(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            warn!(%error, "failed to listen for ctrl-c");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                warn!(%error, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }

    info!("shutdown signal received");
}

use std::path::Path;

use crate::config::Config;
use crate::error::AppError;

/// Prints build information and checks runtime paths.
pub fn run(config_path: &Path) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let mut ok = true;

    println!("latchix {}", env!("CARGO_PKG_VERSION"));
    println!("sftp protocol {}", latchix_sftp::protocol_version());
    println!("audit schema {}", latchix_audit::schema_version());

    match latchix_store::sqlite_version() {
        Ok(version) => println!("sqlite {version}"),
        Err(error) => {
            println!("sqlite unavailable: {error}");
            ok = false;
        }
    }

    #[cfg(feature = "ui")]
    println!("ui compiled ({})", latchix_ui::FEATURE_NAME);
    #[cfg(not(feature = "ui"))]
    println!("ui not compiled");

    if config.ui.enabled && !cfg!(feature = "ui") {
        println!("ui enabled in config but not compiled");
    }

    println!("ssh bind {}", config.server.bind);
    match &config.api.tcp {
        Some(tcp) => println!("api tcp {tcp}"),
        None => println!("api tcp disabled"),
    }
    if let Some(path) = &config.api.unix_socket {
        println!("api unix {}", path.display());
    }
    match config.database_path() {
        Ok(path) => println!("database {}", path.display()),
        Err(error) => {
            println!("database path invalid: {error}");
            ok = false;
        }
    }

    for status in config.path_statuses() {
        println!(
            "[{}] {}: {}",
            if status.ok { "ok" } else { "fail" },
            status.label,
            status.detail
        );
        ok &= status.ok;
    }

    if ok {
        println!("doctor: ok");
        Ok(())
    } else {
        Err(AppError::Doctor("one or more checks failed".into()))
    }
}

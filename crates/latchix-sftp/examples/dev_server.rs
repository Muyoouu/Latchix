use std::env;
use std::error::Error;
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use latchix_core::{RootId, TenantId};
use latchix_sftp::server::{SftpServerConfig, spawn};
use russh::keys::{Algorithm, PrivateKey, ssh_key::LineEnding};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    fs::create_dir_all(&args.data_dir)?;
    let root = args.data_dir.join("root");
    fs::create_dir_all(&root)?;

    let host_key = load_or_create_key(&args.data_dir.join("host_ed25519_key"))?;
    let client_key = load_or_create_key(&args.data_dir.join("id_ed25519"))?;
    let client_key_path = args.data_dir.join("id_ed25519");

    let server = spawn(SftpServerConfig {
        bind: args.bind,
        host_keys: vec![host_key],
        authorized_keys: vec![client_key.public_key().clone()],
        root: root.clone(),
        tenant_id: TenantId::new(),
        root_id: RootId::new(),
    })
    .await?;

    println!("Latchix dev SFTP server");
    println!("  bind: {}", server.local_addr());
    println!("  root: {}", root.display());
    println!("  client key: {}", client_key_path.display());
    println!();
    println!(
        "sftp -i {} -P {} -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null demo@127.0.0.1",
        client_key_path.display(),
        server.local_addr().port()
    );
    println!();
    println!("Press Ctrl-C to stop.");

    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}

struct Args {
    bind: SocketAddr,
    data_dir: PathBuf,
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut bind = "127.0.0.1:2222".parse::<SocketAddr>()?;
    let mut data_dir = PathBuf::from("/tmp/latchix-sftp-dev");
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => {
                let value = args.next().ok_or("--bind requires a value")?;
                bind = value.parse()?;
            }
            "--data-dir" => {
                data_dir = PathBuf::from(args.next().ok_or("--data-dir requires a value")?);
            }
            "--help" | "-h" => {
                println!("Usage: dev_server [--bind ADDR] [--data-dir PATH]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    Ok(Args { bind, data_dir })
}

fn load_or_create_key(path: &Path) -> Result<PrivateKey, Box<dyn Error>> {
    if path.exists() {
        let text = fs::read_to_string(path)?;
        return Ok(PrivateKey::from_openssh(text.as_bytes())?);
    }

    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)?;
    fs::write(path, key.to_openssh(LineEnding::LF)?.as_bytes())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(key)
}

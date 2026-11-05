use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use latchix_core::{RootId, TenantId};
use latchix_sftp::server::{SftpServerConfig, spawn};
use russh::keys::{Algorithm, PrivateKey, ssh_key::LineEnding};

fn key(path: &std::path::Path) -> PrivateKey {
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).expect("key");
    let encoded = key.to_openssh(LineEnding::LF).expect("encode key");
    fs::write(path, encoded.as_bytes()).expect("write key");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("key mode");
    key
}

#[tokio::test]
async fn rclone_copy_round_trip() {
    if Command::new("rclone").arg("version").output().is_err() {
        eprintln!("rclone not installed, skipping");
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    let local = temp.path().join("local");
    fs::create_dir(&root).expect("root");
    fs::create_dir(&local).expect("local");
    fs::write(local.join("one.txt"), b"one").expect("one");
    fs::write(local.join("two.bin"), [0_u8, 1, 2, 3]).expect("two");

    let host_key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).expect("host key");
    let client_key = key(&temp.path().join("id_ed25519"));
    let client_public = client_key.public_key().clone();

    let server = spawn(SftpServerConfig {
        bind: "127.0.0.1:0".parse().expect("bind"),
        host_keys: vec![host_key],
        authorized_keys: vec![client_public],
        root: root.clone(),
        tenant_id: TenantId::new(),
        root_id: RootId::new(),
    })
    .await
    .expect("server");

    let config = temp.path().join("rclone.conf");
    fs::write(
        &config,
        format!(
            "[latchix]\ntype = sftp\nhost = 127.0.0.1\nport = {}\nuser = demo\nkey_file = {}\nshell_type = none\ndisable_hashcheck = true\nset_modtime = false\n",
            server.local_addr().port(),
            temp.path().join("id_ed25519").display()
        ),
    )
    .expect("config");

    let output = tokio::task::spawn_blocking(move || {
        Command::new("timeout")
            .args([
                "30",
                "rclone",
                "--config",
                config.to_str().expect("config path"),
                "copy",
                local.to_str().expect("local path"),
                "latchix:/",
                "-v",
            ])
            .output()
            .expect("run rclone")
    })
    .await
    .expect("rclone task");

    assert!(
        output.status.success(),
        "rclone failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(root.join("one.txt")).expect("one"), b"one");
    assert_eq!(fs::read(root.join("two.bin")).expect("two"), [0, 1, 2, 3]);

    server.shutdown().await;
}

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
async fn openssh_sftp_round_trip() {
    if Command::new("sftp").arg("-h").output().is_err() {
        eprintln!("sftp client not installed, skipping");
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    fs::create_dir(&root).expect("root");

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

    let local = temp.path().join("local.txt");
    let download = temp.path().join("download.txt");
    fs::write(&local, b"hello from sftp").expect("local file");
    let batch = temp.path().join("batch.txt");
    fs::write(
        &batch,
        format!(
            "put {} /upload.txt\nls -l /\nget /upload.txt {}\nmkdir /dir\nrename /upload.txt /dir/moved.txt\nrm /dir/moved.txt\nrmdir /dir\n",
            local.display(),
            download.display()
        ),
    )
    .expect("batch");

    let port = server.local_addr().port();
    let key_path = temp.path().join("id_ed25519");
    let output = tokio::task::spawn_blocking(move || {
        Command::new("timeout")
            .args([
                "15",
                "sftp",
                "-vv",
                "-b",
                batch.to_str().expect("batch path"),
                "-i",
                key_path.to_str().expect("key path"),
                "-P",
                &port.to_string(),
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "demo@127.0.0.1",
            ])
            .output()
            .expect("run sftp")
    })
    .await
    .expect("sftp task");

    assert!(
        output.status.success(),
        "sftp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&download).expect("download"), b"hello from sftp");
    assert!(!root.join("upload.txt").exists());
    assert!(!root.join("dir").exists());

    server.shutdown().await;
}

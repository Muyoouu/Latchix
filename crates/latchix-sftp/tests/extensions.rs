use std::collections::HashMap;
use std::sync::Arc;

use latchix_core::{RootId, TenantContext, TenantId};
use latchix_sftp::LocalFs;
use latchix_sftp::handler::SftpHandler;
use russh_sftp::protocol::{FileAttributes, OpenFlags, Packet, StatusCode};
use russh_sftp::server::Handler;

fn string(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    encoded.extend_from_slice(bytes);
    encoded
}

fn handler(root: &std::path::Path) -> SftpHandler {
    let fs = LocalFs::new(root).expect("local fs");
    SftpHandler::new(
        Arc::new(fs),
        TenantContext::new(TenantId::new(), RootId::new()),
        1,
        None,
    )
}

fn assert_ok(packet: Packet, id: u32) {
    match packet {
        Packet::Status(status) => {
            assert_eq!(status.id, id);
            assert_eq!(status.status_code, StatusCode::Ok);
        }
        other => panic!("expected status, got {other:?}"),
    }
}

#[tokio::test]
async fn init_advertises_supported_extensions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut handler = handler(temp.path());

    let version = handler.init(3, HashMap::new()).await.expect("init");
    assert_eq!(version.version, 3);
    assert_eq!(
        version.extensions.get("posix-rename@openssh.com"),
        Some(&"1".to_owned())
    );
    assert_eq!(
        version.extensions.get("fsync@openssh.com"),
        Some(&"1".to_owned())
    );
}

#[tokio::test]
async fn posix_rename_moves_a_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("old.txt"), b"rename me").expect("file");
    let mut handler = handler(temp.path());

    let mut data = string("/old.txt");
    data.extend(string("/new.txt"));
    let packet = handler
        .extended(7, "posix-rename@openssh.com".to_owned(), data)
        .await
        .expect("posix rename");

    assert_ok(packet, 7);
    assert!(!temp.path().join("old.txt").exists());
    assert_eq!(
        std::fs::read(temp.path().join("new.txt")).expect("renamed"),
        b"rename me"
    );
}

#[tokio::test]
async fn fsync_flushes_an_open_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("file.txt"), b"data").expect("file");

    let mut handler = handler(temp.path());
    let opened = handler
        .open(
            1,
            "/file.txt".to_owned(),
            OpenFlags::WRITE,
            FileAttributes::default(),
        )
        .await
        .expect("open");
    let handle = opened.handle;

    let packet = handler
        .extended(8, "fsync@openssh.com".to_owned(), string(&handle))
        .await
        .expect("fsync");

    assert_ok(packet, 8);
    handler.close(9, handle).await.expect("close");
}

#[tokio::test]
async fn malformed_and_unknown_extensions_fail_closed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut handler = handler(temp.path());

    let malformed = handler
        .extended(9, "posix-rename@openssh.com".to_owned(), vec![0, 0])
        .await
        .expect_err("malformed");
    assert_eq!(malformed.status_code, StatusCode::BadMessage);

    let mut trailing = string("/a.txt");
    trailing.extend(string("/b.txt"));
    trailing.push(0);
    let trailing = handler
        .extended(10, "posix-rename@openssh.com".to_owned(), trailing)
        .await
        .expect_err("trailing data");
    assert_eq!(trailing.status_code, StatusCode::BadMessage);

    let unknown = handler
        .extended(11, "hardlink@openssh.com".to_owned(), Vec::new())
        .await
        .expect_err("unknown");
    assert_eq!(unknown.status_code, StatusCode::OpUnsupported);
}

use std::sync::Arc;

use latchix_core::{RootId, TenantContext, TenantId};
use latchix_sftp::LocalFs;
use latchix_sftp::handler::SftpHandler;
use russh_sftp::protocol::{FileAttributes, OpenFlags, StatusCode};
use russh_sftp::server::Handler;

fn session(fs: Arc<LocalFs>, id: u64) -> SftpHandler {
    SftpHandler::new(
        fs,
        TenantContext::new(TenantId::new(), RootId::new()),
        id,
        None,
    )
}

fn assert_invalid_handle(error: russh_sftp::server::StatusReply) {
    assert_eq!(error.status_code, StatusCode::Failure);
    assert_eq!(error.error_message.as_deref(), Some("invalid handle"));
}

#[tokio::test]
async fn handles_are_scoped_to_one_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("secret.txt"), b"secret").expect("file");
    let fs = Arc::new(LocalFs::new(temp.path()).expect("local fs"));

    let mut first = session(Arc::clone(&fs), 1);
    let mut second = session(Arc::clone(&fs), 2);

    let opened = first
        .open(
            1,
            "/secret.txt".to_owned(),
            OpenFlags::READ,
            FileAttributes::default(),
        )
        .await
        .expect("open");
    let handle = opened.handle;

    let data = first
        .read(2, handle.clone(), 0, 6)
        .await
        .expect("owner read");
    assert_eq!(data.data, b"secret");

    assert_invalid_handle(
        second
            .read(3, handle.clone(), 0, 6)
            .await
            .expect_err("cross-session read"),
    );
    assert_invalid_handle(
        second
            .fstat(4, handle.clone())
            .await
            .expect_err("cross-session fstat"),
    );
    assert_invalid_handle(
        second
            .write(5, handle.clone(), 0, b"x".to_vec())
            .await
            .expect_err("cross-session write"),
    );
    assert_invalid_handle(
        second
            .fsetstat(
                6,
                handle.clone(),
                FileAttributes {
                    size: Some(0),
                    ..FileAttributes::default()
                },
            )
            .await
            .expect_err("cross-session fsetstat"),
    );
    assert_invalid_handle(
        second
            .close(7, handle.clone())
            .await
            .expect_err("cross-session close"),
    );

    let mut fsync = Vec::new();
    fsync.extend_from_slice(&(handle.len() as u32).to_be_bytes());
    fsync.extend_from_slice(handle.as_bytes());
    assert_invalid_handle(
        second
            .extended(8, "fsync@openssh.com".to_owned(), fsync)
            .await
            .expect_err("cross-session fsync"),
    );

    assert_invalid_handle(
        second
            .readdir(9, handle.clone())
            .await
            .expect_err("cross-session readdir"),
    );

    let error = first
        .readdir(10, handle.clone())
        .await
        .expect_err("file handle is not a directory");
    assert_eq!(error.status_code, StatusCode::Failure);
    assert_eq!(
        error.error_message.as_deref(),
        Some("not a directory handle")
    );

    first.close(11, handle).await.expect("owner close");
}

#[tokio::test]
async fn directory_handles_are_scoped_to_one_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(temp.path().join("dir")).expect("dir");
    let fs = Arc::new(LocalFs::new(temp.path()).expect("local fs"));

    let mut first = session(Arc::clone(&fs), 1);
    let mut second = session(Arc::clone(&fs), 2);

    let opened = first.opendir(1, "/dir".to_owned()).await.expect("opendir");

    assert_invalid_handle(
        second
            .readdir(2, opened.handle.clone())
            .await
            .expect_err("cross-session readdir"),
    );
    assert_invalid_handle(
        second
            .close(3, opened.handle.clone())
            .await
            .expect_err("cross-session close"),
    );

    first.close(4, opened.handle).await.expect("owner close");
}

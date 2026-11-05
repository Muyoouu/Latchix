use std::fs;
use std::os::unix::fs::symlink;

use latchix_core::{FileKind, FsError, OpenOptions, SetAttrs, VirtualFs, VirtualPath};
use latchix_sftp::LocalFs;
use tempfile::TempDir;

fn path(value: &str) -> VirtualPath {
    VirtualPath::parse(value).expect("valid path")
}

fn setup() -> (TempDir, LocalFs) {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    fs::create_dir(&root).expect("root dir");
    let local = LocalFs::new(&root).expect("local fs");
    (temp, local)
}

#[tokio::test]
async fn create_write_read_and_close() {
    let (_temp, local) = setup();
    let handle = local
        .open(
            &path("/file.txt"),
            OpenOptions {
                write: true,
                create: true,
                truncate: true,
                ..OpenOptions::default()
            },
        )
        .await
        .expect("open");

    assert_eq!(local.write(handle, 0, b"hello").await.expect("write"), 5);
    local.fsync(handle).await.expect("fsync");
    let metadata = local.fstat(handle).await.expect("fstat");
    assert_eq!(metadata.kind, FileKind::File);
    assert_eq!(metadata.size, 5);
    local.close(handle).await.expect("close");

    let handle = local
        .open(
            &path("/file.txt"),
            OpenOptions {
                read: true,
                ..OpenOptions::default()
            },
        )
        .await
        .expect("open read");
    assert_eq!(local.read(handle, 0, 32).await.expect("read"), b"hello");
    assert_eq!(local.read(handle, 5, 32).await.expect("read eof"), b"");
    local.close(handle).await.expect("close read");
}

#[tokio::test]
async fn symlink_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    symlink(&outside, temp.path().join("root").join("link")).expect("symlink");

    let error = local
        .open(
            &path("/link"),
            OpenOptions {
                read: true,
                ..OpenOptions::default()
            },
        )
        .await
        .expect_err("symlink open must fail");
    assert!(matches!(error, FsError::SymlinkDenied));

    let error = local
        .stat(&path("/link"))
        .await
        .expect_err("symlink stat must fail");
    assert!(matches!(error, FsError::SymlinkDenied));
}

#[tokio::test]
async fn symlink_parent_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).expect("outside dir");
    symlink(&outside, temp.path().join("root").join("link")).expect("symlink");

    let error = local
        .mkdir(&path("/link/new"), 0o700)
        .await
        .expect_err("mkdir through symlink must fail");
    assert!(matches!(
        error,
        FsError::SymlinkDenied | FsError::NotDirectory
    ));
    assert!(!outside.join("new").exists());
}

#[tokio::test]
async fn hardlink_is_denied_before_truncate() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"do not truncate").expect("outside file");
    fs::hard_link(&outside, temp.path().join("root").join("inside.txt")).expect("hardlink");

    let error = local
        .open(
            &path("/inside.txt"),
            OpenOptions {
                write: true,
                truncate: true,
                ..OpenOptions::default()
            },
        )
        .await
        .expect_err("hardlink open must fail");
    assert!(matches!(error, FsError::HardlinkDenied));
    assert_eq!(
        fs::read(&outside).expect("outside read"),
        b"do not truncate"
    );
}

#[tokio::test]
async fn hardlink_rename_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    fs::hard_link(&outside, temp.path().join("root").join("inside.txt")).expect("hardlink");

    let error = local
        .rename(&path("/inside.txt"), &path("/renamed.txt"))
        .await
        .expect_err("hardlink rename must fail");
    assert!(matches!(error, FsError::HardlinkDenied));
}

#[tokio::test]
async fn rename_symlink_source_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    symlink(&outside, temp.path().join("root").join("link")).expect("symlink");

    let error = local
        .rename(&path("/link"), &path("/renamed.txt"))
        .await
        .expect_err("symlink source rename must fail");
    assert!(matches!(error, FsError::SymlinkDenied));
    assert_eq!(fs::read(&outside).expect("outside read"), b"outside");
    assert!(
        fs::symlink_metadata(temp.path().join("root").join("link"))
            .expect("symlink still exists")
            .file_type()
            .is_symlink()
    );
}

#[tokio::test]
async fn rename_onto_symlink_is_denied() {
    let (temp, local) = setup();
    let root = temp.path().join("root");
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    fs::write(root.join("source.txt"), b"source").expect("source file");
    symlink(&outside, root.join("target-link")).expect("symlink");

    let error = local
        .rename(&path("/source.txt"), &path("/target-link"))
        .await
        .expect_err("symlink target rename must fail");
    assert!(matches!(error, FsError::SymlinkDenied));
    assert_eq!(fs::read(&outside).expect("outside read"), b"outside");
    assert_eq!(
        fs::read(root.join("source.txt")).expect("source"),
        b"source"
    );
    assert!(
        fs::symlink_metadata(root.join("target-link"))
            .expect("symlink still exists")
            .file_type()
            .is_symlink()
    );
}

#[tokio::test]
async fn remove_symlink_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    symlink(&outside, temp.path().join("root").join("link")).expect("symlink");

    let error = local
        .remove(&path("/link"))
        .await
        .expect_err("symlink remove must fail");
    assert!(matches!(error, FsError::SymlinkDenied));
}

#[tokio::test]
async fn rename_within_root() {
    let (_temp, local) = setup();
    fs::write(_temp.path().join("root").join("a.txt"), b"a").expect("file");

    local
        .rename(&path("/a.txt"), &path("/b.txt"))
        .await
        .expect("rename");
    assert!(matches!(
        local.stat(&path("/a.txt")).await,
        Err(FsError::NotFound)
    ));
    assert_eq!(local.stat(&path("/b.txt")).await.expect("stat").size, 1);
}

#[tokio::test]
async fn mkdir_readdir_rmdir_and_remove() {
    let (temp, local) = setup();
    local.mkdir(&path("/dir"), 0o700).await.expect("mkdir");
    fs::write(temp.path().join("root/dir/file.txt"), b"x").expect("file");
    symlink(
        temp.path().join("root/dir/file.txt"),
        temp.path().join("root/dir/link"),
    )
    .expect("symlink");

    let handle = local.opendir(&path("/dir")).await.expect("opendir");
    let mut names: Vec<String> = local
        .readdir(handle)
        .await
        .expect("readdir")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    names.sort();
    assert_eq!(names, vec!["file.txt"]);
    assert!(local.readdir(handle).await.expect("readdir eof").is_empty());
    local.closedir(handle).await.expect("closedir");
    fs::remove_file(temp.path().join("root/dir/link")).expect("remove symlink");

    local
        .remove(&path("/dir/file.txt"))
        .await
        .expect("remove file");
    local.rmdir(&path("/dir")).await.expect("rmdir");
    assert!(matches!(
        local.stat(&path("/dir")).await,
        Err(FsError::NotFound)
    ));
}

#[tokio::test]
async fn setstat_changes_size() {
    let (_temp, local) = setup();
    fs::write(_temp.path().join("root/file.txt"), b"abcdef").expect("file");
    local
        .setstat(
            &path("/file.txt"),
            SetAttrs {
                size: Some(3),
                ..SetAttrs::default()
            },
        )
        .await
        .expect("setstat");
    assert_eq!(local.stat(&path("/file.txt")).await.expect("stat").size, 3);
}

#[tokio::test]
async fn setstat_on_hardlink_is_denied() {
    let (temp, local) = setup();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file");
    fs::hard_link(&outside, temp.path().join("root/inside.txt")).expect("hardlink");

    let error = local
        .setstat(
            &path("/inside.txt"),
            SetAttrs {
                size: Some(0),
                ..SetAttrs::default()
            },
        )
        .await
        .expect_err("hardlink setstat must fail");
    assert!(matches!(error, FsError::HardlinkDenied));
    assert_eq!(fs::read(&outside).expect("outside read"), b"outside");
}

#[tokio::test]
async fn open_directory_and_opendir_file_fail() {
    let (temp, local) = setup();
    fs::write(temp.path().join("root/file.txt"), b"x").expect("file");
    assert!(matches!(
        local
            .open(
                &path("/"),
                OpenOptions {
                    read: true,
                    ..OpenOptions::default()
                }
            )
            .await,
        Err(FsError::IsDirectory)
    ));
    assert!(matches!(
        local.opendir(&path("/file.txt")).await,
        Err(FsError::NotDirectory)
    ));
}

#[tokio::test]
async fn stale_handles_are_rejected() {
    let (_temp, local) = setup();
    fs::write(_temp.path().join("root/file.txt"), b"x").expect("file");
    let handle = local
        .open(
            &path("/file.txt"),
            OpenOptions {
                read: true,
                ..OpenOptions::default()
            },
        )
        .await
        .expect("open");
    local.close(handle).await.expect("close");
    assert!(matches!(
        local.close(handle).await,
        Err(FsError::StaleHandle)
    ));
}

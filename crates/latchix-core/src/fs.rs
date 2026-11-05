use std::time::SystemTime;

use async_trait::async_trait;

use crate::path::VirtualPath;

/// Opaque file handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileHandle(u64);

impl FileHandle {
    /// Wraps a raw handle identifier.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw handle identifier.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Opaque directory handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DirHandle(u64);

impl DirHandle {
    /// Wraps a raw handle identifier.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw handle identifier.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Kind of filesystem object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Other object such as a device, socket, or FIFO.
    Other,
}

/// File or directory metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// Object kind.
    pub kind: FileKind,
    /// Size in bytes.
    pub size: u64,
    /// Unix mode bits, including the file type bits.
    pub permissions: u32,
    /// Hardlink count.
    pub nlink: u64,
    /// Owner user ID.
    pub uid: u32,
    /// Owner group ID.
    pub gid: u32,
    /// Last access time.
    pub accessed: Option<SystemTime>,
    /// Last modification time.
    pub modified: Option<SystemTime>,
}

/// Directory entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirEntry {
    /// Entry name, without a path separator.
    pub name: String,
    /// Entry metadata.
    pub metadata: Metadata,
}

/// Open options for a file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OpenOptions {
    /// Open for reading.
    pub read: bool,
    /// Open for writing.
    pub write: bool,
    /// Create the file if missing.
    pub create: bool,
    /// Truncate the file after opening.
    pub truncate: bool,
    /// Append writes to the end.
    pub append: bool,
    /// Fail if the file already exists.
    pub exclusive: bool,
}

/// Attributes that may be changed.
///
/// Permissions and ownership are intentionally absent. They are rejected in
/// v0.1.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SetAttrs {
    /// New file size, if changing size.
    pub size: Option<u64>,
    /// New access time, if supported.
    pub accessed: Option<SystemTime>,
    /// New modification time, if supported.
    pub modified: Option<SystemTime>,
}

/// Filesystem error.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// Object does not exist.
    #[error("not found")]
    NotFound,
    /// Access is denied.
    #[error("permission denied")]
    PermissionDenied,
    /// Object already exists.
    #[error("already exists")]
    AlreadyExists,
    /// Path component is not a directory.
    #[error("not a directory")]
    NotDirectory,
    /// Object is a directory.
    #[error("is a directory")]
    IsDirectory,
    /// Path is invalid.
    #[error("invalid path")]
    InvalidPath,
    /// Operation is not supported.
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    /// Rename crossed a root boundary.
    #[error("cross-root rename")]
    CrossRootRename,
    /// Hardlinks are denied.
    #[error("hardlinks are denied")]
    HardlinkDenied,
    /// Symlinks are denied.
    #[error("symlinks are denied")]
    SymlinkDenied,
    /// Handle is no longer valid.
    #[error("stale handle")]
    StaleHandle,
    /// Handle limit reached.
    #[error("too many open handles")]
    TooManyHandles,
    /// Quota is exhausted.
    #[error("quota exceeded")]
    QuotaExceeded,
    /// Underlying I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Root-bound virtual filesystem port.
///
/// Adapters must resolve every path beneath one root descriptor. They must not
/// accept host paths or follow symlinks.
#[async_trait]
pub trait VirtualFs: Send + Sync {
    /// Opens a file.
    async fn open(&self, path: &VirtualPath, options: OpenOptions) -> Result<FileHandle, FsError>;

    /// Closes a file handle.
    async fn close(&self, handle: FileHandle) -> Result<(), FsError>;

    /// Reads up to `len` bytes at `offset`.
    async fn read(&self, handle: FileHandle, offset: u64, len: u32) -> Result<Vec<u8>, FsError>;

    /// Writes bytes at `offset`.
    async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> Result<u32, FsError>;

    /// Returns metadata for a path.
    async fn stat(&self, path: &VirtualPath) -> Result<Metadata, FsError>;

    /// Returns metadata without following a final symlink.
    async fn lstat(&self, path: &VirtualPath) -> Result<Metadata, FsError>;

    /// Returns metadata for an open file.
    async fn fstat(&self, handle: FileHandle) -> Result<Metadata, FsError>;

    /// Changes attributes for a path.
    async fn setstat(&self, path: &VirtualPath, attrs: SetAttrs) -> Result<(), FsError>;

    /// Changes attributes for an open file.
    async fn fsetstat(&self, handle: FileHandle, attrs: SetAttrs) -> Result<(), FsError>;

    /// Opens a directory.
    async fn opendir(&self, path: &VirtualPath) -> Result<DirHandle, FsError>;

    /// Returns the next directory entries. An empty vector means end of stream.
    async fn readdir(&self, handle: DirHandle) -> Result<Vec<DirEntry>, FsError>;

    /// Closes a directory handle.
    async fn closedir(&self, handle: DirHandle) -> Result<(), FsError>;

    /// Creates a directory.
    async fn mkdir(&self, path: &VirtualPath, mode: u32) -> Result<(), FsError>;

    /// Removes an empty directory.
    async fn rmdir(&self, path: &VirtualPath) -> Result<(), FsError>;

    /// Removes a file.
    async fn remove(&self, path: &VirtualPath) -> Result<(), FsError>;

    /// Renames a file or directory within one root.
    async fn rename(&self, from: &VirtualPath, to: &VirtualPath) -> Result<(), FsError>;

    /// Returns the canonical virtual path.
    async fn realpath(&self, path: &VirtualPath) -> Result<VirtualPath, FsError>;

    /// Flushes an open file to durable storage.
    async fn fsync(&self, handle: FileHandle) -> Result<(), FsError>;
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    use std::time::SystemTime;

    use async_trait::async_trait;

    use super::{
        DirEntry, DirHandle, FileHandle, FileKind, FsError, Metadata, OpenOptions, SetAttrs,
        VirtualFs, VirtualPath,
    };

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);

        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    fn metadata() -> Metadata {
        Metadata {
            kind: FileKind::File,
            size: 7,
            permissions: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            accessed: Some(SystemTime::UNIX_EPOCH),
            modified: Some(SystemTime::UNIX_EPOCH),
        }
    }

    struct MockFs;

    #[async_trait]
    impl VirtualFs for MockFs {
        async fn open(
            &self,
            _path: &VirtualPath,
            _options: OpenOptions,
        ) -> Result<FileHandle, FsError> {
            Ok(FileHandle::new(1))
        }

        async fn close(&self, _handle: FileHandle) -> Result<(), FsError> {
            Ok(())
        }

        async fn read(
            &self,
            _handle: FileHandle,
            _offset: u64,
            len: u32,
        ) -> Result<Vec<u8>, FsError> {
            let payload = b"latchix";
            let end = (len as usize).min(payload.len());
            Ok(payload[..end].to_vec())
        }

        async fn write(
            &self,
            _handle: FileHandle,
            _offset: u64,
            data: &[u8],
        ) -> Result<u32, FsError> {
            Ok(data.len() as u32)
        }

        async fn stat(&self, _path: &VirtualPath) -> Result<Metadata, FsError> {
            Ok(metadata())
        }

        async fn lstat(&self, _path: &VirtualPath) -> Result<Metadata, FsError> {
            Ok(metadata())
        }

        async fn fstat(&self, _handle: FileHandle) -> Result<Metadata, FsError> {
            Ok(metadata())
        }

        async fn setstat(&self, _path: &VirtualPath, _attrs: SetAttrs) -> Result<(), FsError> {
            Ok(())
        }

        async fn fsetstat(&self, _handle: FileHandle, _attrs: SetAttrs) -> Result<(), FsError> {
            Ok(())
        }

        async fn opendir(&self, _path: &VirtualPath) -> Result<DirHandle, FsError> {
            Ok(DirHandle::new(2))
        }

        async fn readdir(&self, _handle: DirHandle) -> Result<Vec<DirEntry>, FsError> {
            Ok(vec![DirEntry {
                name: "report.txt".to_owned(),
                metadata: metadata(),
            }])
        }

        async fn closedir(&self, _handle: DirHandle) -> Result<(), FsError> {
            Ok(())
        }

        async fn mkdir(&self, _path: &VirtualPath, _mode: u32) -> Result<(), FsError> {
            Ok(())
        }

        async fn rmdir(&self, _path: &VirtualPath) -> Result<(), FsError> {
            Ok(())
        }

        async fn remove(&self, _path: &VirtualPath) -> Result<(), FsError> {
            Ok(())
        }

        async fn rename(&self, _from: &VirtualPath, _to: &VirtualPath) -> Result<(), FsError> {
            Ok(())
        }

        async fn realpath(&self, path: &VirtualPath) -> Result<VirtualPath, FsError> {
            Ok(path.clone())
        }

        async fn fsync(&self, _handle: FileHandle) -> Result<(), FsError> {
            Ok(())
        }
    }

    #[test]
    fn handles_wrap_raw_values() {
        let file = FileHandle::new(9);
        assert_eq!(file.raw(), 9);
        assert_eq!(file, FileHandle::new(9));

        let directory = DirHandle::new(4);
        assert_eq!(directory.raw(), 4);
        assert_eq!(directory, DirHandle::new(4));
    }

    #[test]
    fn file_kinds_are_distinct() {
        assert_ne!(FileKind::File, FileKind::Directory);
        assert_ne!(FileKind::Symlink, FileKind::Other);
        assert_eq!(FileKind::File, FileKind::File);
    }

    #[test]
    fn metadata_and_dir_entry_keep_all_fields() {
        let metadata = metadata();
        assert_eq!(metadata.kind, FileKind::File);
        assert_eq!(metadata.size, 7);
        assert_eq!(metadata.permissions, 0o644);
        assert_eq!(metadata.nlink, 1);
        assert_eq!(metadata.uid, 1000);
        assert_eq!(metadata.gid, 1000);
        assert_eq!(metadata.accessed, Some(SystemTime::UNIX_EPOCH));
        assert_eq!(metadata.modified, Some(SystemTime::UNIX_EPOCH));

        let entry = DirEntry {
            name: "report.txt".to_owned(),
            metadata,
        };
        assert_eq!(entry.name, "report.txt");
        assert_eq!(entry.metadata.size, 7);
    }

    #[test]
    fn open_options_default_to_closed() {
        let options = OpenOptions::default();
        assert!(!options.read);
        assert!(!options.write);
        assert!(!options.create);
        assert!(!options.truncate);
        assert!(!options.append);
        assert!(!options.exclusive);
    }

    #[test]
    fn set_attrs_default_to_no_change() {
        let attrs = SetAttrs::default();
        assert_eq!(attrs.size, None);
        assert_eq!(attrs.accessed, None);
        assert_eq!(attrs.modified, None);
    }

    #[test]
    fn errors_do_not_expose_host_paths() {
        assert_eq!(FsError::NotFound.to_string(), "not found");
        assert_eq!(FsError::PermissionDenied.to_string(), "permission denied");
        assert_eq!(FsError::AlreadyExists.to_string(), "already exists");
        assert_eq!(FsError::NotDirectory.to_string(), "not a directory");
        assert_eq!(FsError::IsDirectory.to_string(), "is a directory");
        assert_eq!(FsError::InvalidPath.to_string(), "invalid path");
        assert_eq!(
            FsError::Unsupported("hardlink".to_owned()).to_string(),
            "unsupported operation: hardlink"
        );
        assert_eq!(FsError::CrossRootRename.to_string(), "cross-root rename");
        assert_eq!(FsError::HardlinkDenied.to_string(), "hardlinks are denied");
        assert_eq!(FsError::SymlinkDenied.to_string(), "symlinks are denied");
        assert_eq!(FsError::StaleHandle.to_string(), "stale handle");
        assert_eq!(FsError::TooManyHandles.to_string(), "too many open handles");
        assert_eq!(FsError::QuotaExceeded.to_string(), "quota exceeded");
    }

    #[test]
    fn io_errors_convert_to_fs_errors() {
        let source = std::io::Error::from(std::io::ErrorKind::NotFound);
        let error: FsError = source.into();
        assert!(matches!(error, FsError::Io(_)));
        assert!(error.to_string().starts_with("io error: "));
    }

    #[test]
    fn mock_backend_round_trips_through_the_trait() {
        let backend = MockFs;
        let path = VirtualPath::parse("/reports/2026.txt").expect("valid path");

        let handle = block_on(backend.open(&path, OpenOptions::default())).expect("open");
        assert_eq!(handle, FileHandle::new(1));

        let data = block_on(backend.read(handle, 0, 4)).expect("read");
        assert_eq!(data, b"latc");
        assert_eq!(
            block_on(backend.write(handle, 4, b"hix")).expect("write"),
            3
        );
        assert_eq!(block_on(backend.fstat(handle)).expect("fstat").size, 7);
        assert_eq!(block_on(backend.realpath(&path)).expect("realpath"), path);

        let directory = block_on(backend.opendir(&path)).expect("opendir");
        let entries = block_on(backend.readdir(directory)).expect("readdir");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "report.txt");
        block_on(backend.closedir(directory)).expect("closedir");
        block_on(backend.close(handle)).expect("close");
    }

    #[test]
    fn trait_is_dyn_compatible() {
        let backend = MockFs;
        let port: &dyn VirtualFs = &backend;
        let path = VirtualPath::root();

        assert_eq!(
            block_on(port.stat(&path)).expect("stat").kind,
            FileKind::File
        );
    }
}

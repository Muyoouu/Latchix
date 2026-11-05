use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use latchix_core::{
    DirEntry, DirHandle, FileHandle, FileKind, FsError, Metadata, OpenOptions, SetAttrs, VirtualFs,
    VirtualPath,
};
use rustix::fs::{self, AtFlags, CWD, Dir, FileType as RustixFileType, Mode, OFlags, ResolveFlags};
use rustix::io::Errno;
use tokio::sync::Mutex;

const MAX_READ_SIZE: u32 = 1024 * 1024;
const MAX_HANDLES: usize = 4096;
const MAX_DIR_ENTRIES: usize = 100_000;
const DIR_BATCH: usize = 64;

/// Root-bound local filesystem.
///
/// Every path is resolved beneath one root descriptor with `openat2`. Symlinks,
/// magic links, mount crossings, and hardlinked regular files are denied.
pub struct LocalFs {
    root: Arc<OwnedFd>,
    files: Arc<Mutex<HashMap<u64, Arc<File>>>>,
    dirs: Arc<Mutex<HashMap<u64, DirState>>>,
    next_handle: AtomicU64,
}

struct DirState {
    entries: Vec<DirEntry>,
    cursor: usize,
}

impl LocalFs {
    /// Opens a root directory and creates a filesystem adapter.
    pub fn new(root: &Path) -> Result<Self, FsError> {
        let root = fs::openat2(
            CWD,
            root,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(map_errno)?;

        Ok(Self {
            root: Arc::new(root),
            files: Arc::new(Mutex::new(HashMap::new())),
            dirs: Arc::new(Mutex::new(HashMap::new())),
            next_handle: AtomicU64::new(1),
        })
    }

    fn next_handle(&self) -> u64 {
        self.next_handle.fetch_add(1, Ordering::Relaxed)
    }

    fn open_dir_fd(&self, path: &VirtualPath) -> Result<OwnedFd, FsError> {
        fs::openat2(
            self.root.as_fd(),
            path.as_relative(),
            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            resolve_flags(),
        )
        .map_err(map_errno)
    }

    async fn file_handle(&self, handle: FileHandle) -> Result<Arc<File>, FsError> {
        self.files
            .lock()
            .await
            .get(&handle.raw())
            .cloned()
            .ok_or(FsError::StaleHandle)
    }

    async fn insert_file_async(&self, file: Arc<File>) -> Result<FileHandle, FsError> {
        let raw = self.next_handle();
        let mut files = self.files.lock().await;
        if files.len() >= MAX_HANDLES {
            return Err(FsError::TooManyHandles);
        }
        files.insert(raw, file);
        Ok(FileHandle::new(raw))
    }
}

#[async_trait]
impl VirtualFs for LocalFs {
    async fn open(&self, path: &VirtualPath, options: OpenOptions) -> Result<FileHandle, FsError> {
        if options.exclusive && !options.create {
            return Err(FsError::InvalidPath);
        }

        let mut flags = OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        if options.read && options.write {
            flags |= OFlags::RDWR;
        } else if options.write {
            flags |= OFlags::WRONLY;
        } else {
            flags |= OFlags::RDONLY;
        }
        if options.create {
            flags |= OFlags::CREATE;
        }
        if options.exclusive {
            flags |= OFlags::EXCL;
        }
        if options.append {
            flags |= OFlags::APPEND;
        }

        let mode = if options.create {
            Mode::from_bits_truncate(0o600)
        } else {
            Mode::empty()
        };
        let fd = fs::openat2(
            self.root.as_fd(),
            path.as_relative(),
            flags,
            mode,
            resolve_flags(),
        )
        .map_err(map_errno)?;

        let file = File::from(fd);
        let metadata = file.metadata().map_err(FsError::from)?;
        if metadata.is_dir() {
            return Err(FsError::IsDirectory);
        }
        if !metadata.is_file() {
            return Err(FsError::Unsupported("not a regular file".into()));
        }
        if metadata.nlink() > 1 {
            return Err(FsError::HardlinkDenied);
        }

        let file = Arc::new(file);
        if options.truncate {
            let file = Arc::clone(&file);
            tokio::task::spawn_blocking(move || file.set_len(0))
                .await
                .map_err(join_error)?
                .map_err(FsError::from)?;
        }

        self.insert_file_async(file).await
    }

    async fn close(&self, handle: FileHandle) -> Result<(), FsError> {
        self.files
            .lock()
            .await
            .remove(&handle.raw())
            .map(|_| ())
            .ok_or(FsError::StaleHandle)
    }

    async fn read(&self, handle: FileHandle, offset: u64, len: u32) -> Result<Vec<u8>, FsError> {
        let file = self.file_handle(handle).await?;
        let len = len.min(MAX_READ_SIZE) as usize;

        tokio::task::spawn_blocking(move || {
            let mut buffer = vec![0_u8; len];
            let read = file.read_at(&mut buffer, offset)?;
            buffer.truncate(read);
            Ok::<_, std::io::Error>(buffer)
        })
        .await
        .map_err(join_error)?
        .map_err(FsError::from)
    }

    async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> Result<u32, FsError> {
        let file = self.file_handle(handle).await?;
        let data = data.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut written = 0_usize;
            while written < data.len() {
                let count = file.write_at(&data[written..], offset + written as u64)?;
                if count == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "write returned zero",
                    ));
                }
                written += count;
            }
            Ok::<_, std::io::Error>(written as u32)
        })
        .await
        .map_err(join_error)?
        .map_err(FsError::from)
    }

    async fn stat(&self, path: &VirtualPath) -> Result<Metadata, FsError> {
        let fd = fs::openat2(
            self.root.as_fd(),
            path.as_relative(),
            OFlags::PATH | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            resolve_flags(),
        )
        .map_err(map_errno)?;

        let metadata = File::from(fd).metadata().map_err(FsError::from)?;
        if metadata.file_type().is_symlink() {
            return Err(FsError::SymlinkDenied);
        }
        Ok(metadata_from_std(&metadata))
    }

    async fn lstat(&self, path: &VirtualPath) -> Result<Metadata, FsError> {
        self.stat(path).await
    }

    async fn fstat(&self, handle: FileHandle) -> Result<Metadata, FsError> {
        let file = self.file_handle(handle).await?;
        tokio::task::spawn_blocking(move || file.metadata())
            .await
            .map_err(join_error)?
            .map(|metadata| metadata_from_std(&metadata))
            .map_err(FsError::from)
    }

    async fn setstat(&self, path: &VirtualPath, attrs: SetAttrs) -> Result<(), FsError> {
        if attrs.accessed.is_some() || attrs.modified.is_some() {
            return Err(FsError::Unsupported("timestamps are not supported".into()));
        }
        if attrs.size.is_none() {
            return Ok(());
        }

        let handle = self
            .open(
                path,
                OpenOptions {
                    write: true,
                    ..OpenOptions::default()
                },
            )
            .await?;
        let result = self.fsetstat(handle, attrs).await;
        let close = self.close(handle).await;
        result.and(close)
    }

    async fn fsetstat(&self, handle: FileHandle, attrs: SetAttrs) -> Result<(), FsError> {
        if attrs.accessed.is_some() || attrs.modified.is_some() {
            return Err(FsError::Unsupported("timestamps are not supported".into()));
        }
        let Some(size) = attrs.size else {
            return Ok(());
        };

        let file = self.file_handle(handle).await?;
        tokio::task::spawn_blocking(move || {
            let metadata = file.metadata()?;
            if metadata.nlink() > 1 {
                return Err(FsError::HardlinkDenied);
            }
            file.set_len(size)?;
            Ok(())
        })
        .await
        .map_err(join_error)?
    }

    async fn opendir(&self, path: &VirtualPath) -> Result<DirHandle, FsError> {
        let fd = fs::openat2(
            self.root.as_fd(),
            path.as_relative(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            resolve_flags(),
        )
        .map_err(map_errno)?;

        let entries = tokio::task::spawn_blocking(move || read_dir_entries(fd))
            .await
            .map_err(join_error)??;

        let raw = self.next_handle();
        let mut dirs = self.dirs.lock().await;
        if dirs.len() >= MAX_HANDLES {
            return Err(FsError::TooManyHandles);
        }
        dirs.insert(raw, DirState { entries, cursor: 0 });
        Ok(DirHandle::new(raw))
    }

    async fn readdir(&self, handle: DirHandle) -> Result<Vec<DirEntry>, FsError> {
        let mut dirs = self.dirs.lock().await;
        let state = dirs.get_mut(&handle.raw()).ok_or(FsError::StaleHandle)?;
        let end = (state.cursor + DIR_BATCH).min(state.entries.len());
        let batch = state.entries[state.cursor..end].to_vec();
        state.cursor = end;
        Ok(batch)
    }

    async fn closedir(&self, handle: DirHandle) -> Result<(), FsError> {
        self.dirs
            .lock()
            .await
            .remove(&handle.raw())
            .map(|_| ())
            .ok_or(FsError::StaleHandle)
    }

    async fn mkdir(&self, path: &VirtualPath, mode: u32) -> Result<(), FsError> {
        let parent = path.parent().ok_or(FsError::InvalidPath)?;
        let name = path.file_name().ok_or(FsError::InvalidPath)?.to_string();
        let parent = self.open_dir_fd(&parent)?;
        tokio::task::spawn_blocking(move || {
            fs::mkdirat(
                &parent,
                name.as_str(),
                Mode::from_bits_truncate(mode & 0o777),
            )
            .map_err(map_errno)
        })
        .await
        .map_err(join_error)?
    }

    async fn rmdir(&self, path: &VirtualPath) -> Result<(), FsError> {
        let parent = path.parent().ok_or(FsError::InvalidPath)?;
        let name = path.file_name().ok_or(FsError::InvalidPath)?.to_string();
        let parent = self.open_dir_fd(&parent)?;
        tokio::task::spawn_blocking(move || {
            fs::unlinkat(&parent, name.as_str(), AtFlags::REMOVEDIR).map_err(map_errno)
        })
        .await
        .map_err(join_error)?
    }

    async fn remove(&self, path: &VirtualPath) -> Result<(), FsError> {
        let parent = path.parent().ok_or(FsError::InvalidPath)?;
        let name = path.file_name().ok_or(FsError::InvalidPath)?.to_string();
        let parent = self.open_dir_fd(&parent)?;

        tokio::task::spawn_blocking(move || {
            let stat =
                fs::statat(&parent, name.as_str(), AtFlags::SYMLINK_NOFOLLOW).map_err(map_errno)?;
            match RustixFileType::from_raw_mode(stat.st_mode) {
                RustixFileType::Symlink => return Err(FsError::SymlinkDenied),
                RustixFileType::Directory => return Err(FsError::IsDirectory),
                _ => {}
            }
            fs::unlinkat(&parent, name.as_str(), AtFlags::empty()).map_err(map_errno)
        })
        .await
        .map_err(join_error)?
    }

    async fn rename(&self, from: &VirtualPath, to: &VirtualPath) -> Result<(), FsError> {
        let from_parent = from.parent().ok_or(FsError::InvalidPath)?;
        let from_name = from.file_name().ok_or(FsError::InvalidPath)?.to_string();
        let to_parent = to.parent().ok_or(FsError::InvalidPath)?;
        let to_name = to.file_name().ok_or(FsError::InvalidPath)?.to_string();
        let from_fd = self.open_dir_fd(&from_parent)?;
        let to_fd = self.open_dir_fd(&to_parent)?;

        tokio::task::spawn_blocking(move || {
            let source = fs::statat(&from_fd, from_name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(map_errno)?;
            let source_type = RustixFileType::from_raw_mode(source.st_mode);
            if source_type == RustixFileType::Symlink {
                return Err(FsError::SymlinkDenied);
            }
            if source_type == RustixFileType::RegularFile && source.st_nlink > 1 {
                return Err(FsError::HardlinkDenied);
            }

            if let Ok(target) = fs::statat(&to_fd, to_name.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
                let target_type = RustixFileType::from_raw_mode(target.st_mode);
                if target_type == RustixFileType::Symlink {
                    return Err(FsError::SymlinkDenied);
                }
                if target_type == RustixFileType::RegularFile && target.st_nlink > 1 {
                    return Err(FsError::HardlinkDenied);
                }
            }

            fs::renameat(&from_fd, from_name.as_str(), &to_fd, to_name.as_str()).map_err(map_errno)
        })
        .await
        .map_err(join_error)?
    }

    async fn realpath(&self, path: &VirtualPath) -> Result<VirtualPath, FsError> {
        Ok(path.clone())
    }

    async fn fsync(&self, handle: FileHandle) -> Result<(), FsError> {
        let file = self.file_handle(handle).await?;
        tokio::task::spawn_blocking(move || file.sync_all())
            .await
            .map_err(join_error)?
            .map_err(FsError::from)
    }
}

fn resolve_flags() -> ResolveFlags {
    ResolveFlags::BENEATH
        | ResolveFlags::NO_SYMLINKS
        | ResolveFlags::NO_MAGICLINKS
        | ResolveFlags::NO_XDEV
}

fn read_dir_entries(fd: OwnedFd) -> Result<Vec<DirEntry>, FsError> {
    let mut dir = Dir::read_from(&fd).map_err(map_errno)?;
    let mut entries = Vec::new();

    while let Some(entry) = dir.read() {
        let entry = entry.map_err(map_errno)?;
        let Some(name) = entry.file_name().to_str().ok() else {
            continue;
        };
        if name == "." || name == ".." {
            continue;
        }
        if entry.file_type() == RustixFileType::Symlink {
            continue;
        }

        let stat = fs::statat(&fd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(map_errno)?;
        let metadata = metadata_from_stat(&stat);
        if metadata.kind == FileKind::Symlink {
            continue;
        }
        entries.push(DirEntry {
            name: name.to_string(),
            metadata,
        });
        if entries.len() >= MAX_DIR_ENTRIES {
            return Err(FsError::Unsupported(
                "directory has too many entries".into(),
            ));
        }
    }

    Ok(entries)
}

fn metadata_from_std(metadata: &std::fs::Metadata) -> Metadata {
    let kind = if metadata.is_file() {
        FileKind::File
    } else if metadata.is_dir() {
        FileKind::Directory
    } else if metadata.file_type().is_symlink() {
        FileKind::Symlink
    } else {
        FileKind::Other
    };

    Metadata {
        kind,
        size: metadata.size(),
        permissions: metadata.mode(),
        nlink: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        accessed: metadata.accessed().ok(),
        modified: metadata.modified().ok(),
    }
}

fn metadata_from_stat(stat: &fs::Stat) -> Metadata {
    let kind = match RustixFileType::from_raw_mode(stat.st_mode) {
        RustixFileType::RegularFile => FileKind::File,
        RustixFileType::Directory => FileKind::Directory,
        RustixFileType::Symlink => FileKind::Symlink,
        _ => FileKind::Other,
    };

    Metadata {
        kind,
        size: u64::try_from(stat.st_size).unwrap_or(0),
        permissions: stat.st_mode,
        nlink: stat.st_nlink,
        uid: stat.st_uid,
        gid: stat.st_gid,
        accessed: system_time(stat.st_atime, stat.st_atime_nsec),
        modified: system_time(stat.st_mtime, stat.st_mtime_nsec),
    }
}

fn system_time(seconds: i64, nanos: u64) -> Option<SystemTime> {
    if seconds < 0 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::new(
        seconds as u64,
        u32::try_from(nanos).unwrap_or(0),
    ))
}

fn map_errno(errno: Errno) -> FsError {
    match errno {
        Errno::NOENT => FsError::NotFound,
        Errno::ACCESS | Errno::PERM => FsError::PermissionDenied,
        Errno::EXIST => FsError::AlreadyExists,
        Errno::NOTDIR => FsError::NotDirectory,
        Errno::ISDIR => FsError::IsDirectory,
        Errno::LOOP => FsError::SymlinkDenied,
        Errno::XDEV => FsError::CrossRootRename,
        Errno::NAMETOOLONG | Errno::INVAL => FsError::InvalidPath,
        Errno::MFILE | Errno::NFILE => FsError::TooManyHandles,
        Errno::DQUOT => FsError::QuotaExceeded,
        Errno::OPNOTSUPP => FsError::Unsupported(errno.to_string()),
        _ => FsError::Io(std::io::Error::from_raw_os_error(errno.raw_os_error())),
    }
}

fn join_error(error: tokio::task::JoinError) -> FsError {
    FsError::Io(std::io::Error::other(format!(
        "blocking filesystem task failed: {error}"
    )))
}

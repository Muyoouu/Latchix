use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use latchix_core::{
    DirEntry, DirHandle, FileHandle, FsError, Metadata, OpenOptions, SetAttrs, TenantContext,
    VirtualFs, VirtualPath,
};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Packet, Status, StatusCode, Version,
};
use russh_sftp::server::{Handler, StatusReply};
use tracing::debug;

/// SFTP v3 handler backed by a root-bound virtual filesystem.
pub struct SftpHandler {
    fs: Arc<dyn VirtualFs>,
    tenant: TenantContext,
    session_id: u64,
    peer: Option<SocketAddr>,
    handles: HashMap<String, HandleKind>,
}

#[derive(Clone, Copy, Debug)]
enum HandleKind {
    File(FileHandle),
    Directory(DirHandle),
}

impl SftpHandler {
    /// Creates a handler for one SFTP session.
    #[must_use]
    pub fn new(
        fs: Arc<dyn VirtualFs>,
        tenant: TenantContext,
        session_id: u64,
        peer: Option<SocketAddr>,
    ) -> Self {
        Self {
            fs,
            tenant,
            session_id,
            peer,
            handles: HashMap::new(),
        }
    }

    fn ok_status(&self, id: u32) -> Status {
        status(id, StatusCode::Ok, "Ok")
    }

    fn file_handle(&self, value: &str) -> Result<FileHandle, StatusReply> {
        match self.handles.get(value) {
            Some(HandleKind::File(handle)) => Ok(*handle),
            Some(HandleKind::Directory(_)) => {
                Err(StatusCode::Failure.with_message("not a file handle"))
            }
            None => Err(StatusCode::Failure.with_message("invalid handle")),
        }
    }

    fn dir_handle(&self, value: &str) -> Result<DirHandle, StatusReply> {
        match self.handles.get(value) {
            Some(HandleKind::Directory(handle)) => Ok(*handle),
            Some(HandleKind::File(_)) => {
                Err(StatusCode::Failure.with_message("not a directory handle"))
            }
            None => Err(StatusCode::Failure.with_message("invalid handle")),
        }
    }
}

impl Handler for SftpHandler {
    type Error = StatusReply;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported.into()
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: std::collections::HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        let mut version = Version::new();
        version
            .extensions
            .insert("posix-rename@openssh.com".into(), "1".into());
        version
            .extensions
            .insert("fsync@openssh.com".into(), "1".into());
        Ok(version)
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let path = parse_path(&filename)?;
        let options = OpenOptions {
            read: pflags.contains(OpenFlags::READ),
            write: pflags.contains(OpenFlags::WRITE),
            create: pflags.contains(OpenFlags::CREATE),
            truncate: pflags.contains(OpenFlags::TRUNCATE),
            append: pflags.contains(OpenFlags::APPEND),
            exclusive: pflags.contains(OpenFlags::EXCLUDE),
        };

        let handle = self.fs.open(&path, options).await.map_err(map_fs_error)?;
        debug!(
            session = self.session_id,
            tenant = %self.tenant.tenant_id(),
            peer = ?self.peer,
            path = %path,
            "sftp open"
        );
        Ok(Handle {
            id,
            handle: self.insert_handle(HandleKind::File(handle)),
        })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        debug!(session = self.session_id, handle = %handle, "sftp close");
        let kind = self
            .handles
            .remove(&handle)
            .ok_or_else(|| StatusCode::Failure.with_message("invalid handle"))?;
        let result = match kind {
            HandleKind::File(handle) => self.fs.close(handle).await,
            HandleKind::Directory(handle) => self.fs.closedir(handle).await,
        };
        if let Err(error) = result {
            debug!(session = self.session_id, ?error, "sftp close failed");
            return Err(map_fs_error(error));
        }
        Ok(self.ok_status(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let handle = self.file_handle(&handle)?;
        let data = self
            .fs
            .read(handle, offset, len)
            .await
            .map_err(map_fs_error)?;
        if data.is_empty() {
            return Err(StatusCode::Eof.into());
        }
        Ok(Data { id, data })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        debug!(session = self.session_id, handle = %handle, offset, len = data.len(), "sftp write");
        let handle = self.file_handle(&handle)?;
        if let Err(error) = self.fs.write(handle, offset, &data).await {
            debug!(session = self.session_id, ?error, "sftp write failed");
            return Err(map_fs_error(error));
        }
        Ok(self.ok_status(id))
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let path = parse_path(&path)?;
        let metadata = self.fs.lstat(&path).await.map_err(map_fs_error)?;
        Ok(Attrs {
            id,
            attrs: file_attributes(&metadata),
        })
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        let handle = self.file_handle(&handle)?;
        let metadata = self.fs.fstat(handle).await.map_err(map_fs_error)?;
        Ok(Attrs {
            id,
            attrs: file_attributes(&metadata),
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let path = parse_path(&path)?;
        let attrs = parse_set_attrs(&attrs)?;
        self.fs.setstat(&path, attrs).await.map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let handle = self.file_handle(&handle)?;
        let attrs = parse_set_attrs(&attrs)?;
        self.fs
            .fsetstat(handle, attrs)
            .await
            .map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let path = parse_path(&path)?;
        let handle = self.fs.opendir(&path).await.map_err(map_fs_error)?;
        Ok(Handle {
            id,
            handle: self.insert_handle(HandleKind::Directory(handle)),
        })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let handle = self.dir_handle(&handle)?;
        let entries = self.fs.readdir(handle).await.map_err(map_fs_error)?;
        if entries.is_empty() {
            return Err(StatusCode::Eof.into());
        }
        Ok(Name {
            id,
            files: entries.iter().map(file_entry).collect(),
        })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        let path = parse_path(&filename)?;
        self.fs.remove(&path).await.map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let path = parse_path(&path)?;
        self.fs.mkdir(&path, 0o700).await.map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        let path = parse_path(&path)?;
        self.fs.rmdir(&path).await.map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let path = parse_path(&path)?;
        let path = self.fs.realpath(&path).await.map_err(map_fs_error)?;
        Ok(Name {
            id,
            files: vec![File::dummy(path.as_str())],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let path = parse_path(&path)?;
        let metadata = self.fs.stat(&path).await.map_err(map_fs_error)?;
        Ok(Attrs {
            id,
            attrs: file_attributes(&metadata),
        })
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        let oldpath = parse_path(&oldpath)?;
        let newpath = parse_path(&newpath)?;
        self.fs
            .rename(&oldpath, &newpath)
            .await
            .map_err(map_fs_error)?;
        Ok(self.ok_status(id))
    }

    async fn readlink(&mut self, _id: u32, _path: String) -> Result<Name, Self::Error> {
        Err(StatusCode::OpUnsupported.with_message("symlinks are disabled"))
    }

    async fn symlink(
        &mut self,
        _id: u32,
        _linkpath: String,
        _targetpath: String,
    ) -> Result<Status, Self::Error> {
        Err(StatusCode::OpUnsupported.with_message("symlinks are disabled"))
    }

    async fn extended(
        &mut self,
        id: u32,
        request: String,
        data: Vec<u8>,
    ) -> Result<Packet, Self::Error> {
        match request.as_str() {
            "posix-rename@openssh.com" => {
                let [oldpath, newpath] = parse_ssh_strings::<2>(&data)?;
                let oldpath = parse_path(&oldpath)?;
                let newpath = parse_path(&newpath)?;
                self.fs
                    .rename(&oldpath, &newpath)
                    .await
                    .map_err(map_fs_error)?;
                Ok(Packet::Status(self.ok_status(id)))
            }
            "fsync@openssh.com" => {
                let [handle] = parse_ssh_strings::<1>(&data)?;
                let handle = self.file_handle(&handle)?;
                self.fs.fsync(handle).await.map_err(map_fs_error)?;
                Ok(Packet::Status(self.ok_status(id)))
            }
            _ => Err(StatusCode::OpUnsupported.with_message("extension is not supported")),
        }
    }
}

impl SftpHandler {
    fn insert_handle(&mut self, kind: HandleKind) -> String {
        let raw = match kind {
            HandleKind::File(handle) => handle.raw(),
            HandleKind::Directory(handle) => handle.raw(),
        };
        let value = raw.to_string();
        self.handles.insert(value.clone(), kind);
        value
    }
}

impl Drop for SftpHandler {
    fn drop(&mut self) {
        if self.handles.is_empty() {
            return;
        }

        let fs = Arc::clone(&self.fs);
        let handles: Vec<HandleKind> = self.handles.drain().map(|(_, kind)| kind).collect();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                for kind in handles {
                    match kind {
                        HandleKind::File(handle) => {
                            let _ = fs.close(handle).await;
                        }
                        HandleKind::Directory(handle) => {
                            let _ = fs.closedir(handle).await;
                        }
                    }
                }
            });
        }
    }
}

fn parse_path(value: &str) -> Result<VirtualPath, StatusReply> {
    VirtualPath::parse(value).map_err(|_| StatusCode::PermissionDenied.with_message("invalid path"))
}

fn parse_ssh_strings<const N: usize>(data: &[u8]) -> Result<[String; N], StatusReply> {
    let mut offset = 0_usize;
    let mut values = Vec::with_capacity(N);

    for _ in 0..N {
        let length_bytes = data
            .get(offset..offset + 4)
            .ok_or_else(|| StatusCode::BadMessage.with_message("truncated extension"))?;
        let length = u32::from_be_bytes(
            length_bytes
                .try_into()
                .map_err(|_| StatusCode::BadMessage.with_message("invalid extension"))?,
        ) as usize;
        offset += 4;
        let value = data
            .get(offset..offset + length)
            .ok_or_else(|| StatusCode::BadMessage.with_message("truncated extension"))?;
        let value = std::str::from_utf8(value)
            .map_err(|_| StatusCode::BadMessage.with_message("invalid extension string"))?;
        values.push(value.to_string());
        offset += length;
    }

    if offset != data.len() {
        return Err(StatusCode::BadMessage.with_message("trailing extension data"));
    }

    values
        .try_into()
        .map_err(|_| StatusCode::BadMessage.with_message("invalid extension"))
}

fn parse_set_attrs(attrs: &FileAttributes) -> Result<SetAttrs, StatusReply> {
    if attrs.permissions.is_some()
        || attrs.uid.is_some()
        || attrs.user.is_some()
        || attrs.gid.is_some()
        || attrs.group.is_some()
        || attrs.atime.is_some()
        || attrs.mtime.is_some()
    {
        return Err(StatusCode::OpUnsupported.with_message("only size changes are supported"));
    }

    Ok(SetAttrs {
        size: attrs.size,
        ..SetAttrs::default()
    })
}

fn file_attributes(metadata: &Metadata) -> FileAttributes {
    FileAttributes {
        size: Some(metadata.size),
        uid: Some(metadata.uid),
        user: None,
        gid: Some(metadata.gid),
        group: None,
        permissions: Some(metadata.permissions),
        atime: metadata.accessed.and_then(system_time_seconds),
        mtime: metadata.modified.and_then(system_time_seconds),
    }
}

fn system_time_seconds(time: SystemTime) -> Option<u32> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u32::try_from(duration.as_secs()).ok())
}

fn file_entry(entry: &DirEntry) -> File {
    File::new(entry.name.clone(), file_attributes(&entry.metadata))
}

fn status(id: u32, status_code: StatusCode, message: &str) -> Status {
    Status {
        id,
        status_code,
        error_message: message.to_string(),
        language_tag: "en-US".to_string(),
    }
}

fn map_fs_error(error: FsError) -> StatusReply {
    match error {
        FsError::NotFound => StatusCode::NoSuchFile.into(),
        FsError::PermissionDenied
        | FsError::SymlinkDenied
        | FsError::HardlinkDenied
        | FsError::InvalidPath
        | FsError::CrossRootRename => {
            StatusCode::PermissionDenied.with_message("permission denied")
        }
        FsError::AlreadyExists => StatusCode::Failure.with_message("already exists"),
        FsError::NotDirectory => StatusCode::Failure.with_message("not a directory"),
        FsError::IsDirectory => StatusCode::Failure.with_message("is a directory"),
        FsError::Unsupported(message) => StatusCode::OpUnsupported.with_message(message),
        FsError::StaleHandle => StatusCode::Failure.with_message("stale handle"),
        FsError::TooManyHandles => StatusCode::Failure.with_message("too many open handles"),
        FsError::QuotaExceeded => StatusCode::Failure.with_message("quota exceeded"),
        FsError::Io(_) => StatusCode::Failure.with_message("filesystem error"),
    }
}

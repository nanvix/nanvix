// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host filesystem operation handlers.
//!
//! Each handler receives a decoded request, performs the corresponding operation on the host
//! filesystem (constrained by the sandbox), and returns an encoded response payload.

use crate::{
    fd_table::{
        FdEntry,
        FdTable,
    },
    sandbox::Sandbox,
};
use ::hostfs_api::*;
use ::sys::ipc::Message;
use ::sysapi::{
    fcntl::{
        file_access_mode::{
            O_ACCMODE,
            O_RDONLY,
            O_RDWR,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_DIRECTORY,
            O_TRUNC,
        },
        file_status_flags::O_APPEND,
    },
    unistd::file_seek::{
        SEEK_CUR,
        SEEK_END,
        SEEK_SET,
    },
};
use ::syscall::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use std::{
    fs::{
        self,
        File,
        OpenOptions,
    },
    io::{
        self,
        Read,
        Seek,
        SeekFrom,
        Write,
    },
    path::PathBuf,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// The host filesystem request handler.
///
/// Holds the sandbox (root directory constraint) and the remote FD table.
pub struct HostFsHandler {
    /// Path sandbox for security.
    sandbox: Sandbox,
    /// Remote file descriptor table.
    fd_table: FdTable,
}

impl HostFsHandler {
    /// Creates a new handler with the given root directory.
    ///
    /// Returns an error if the root directory does not exist or is not a directory.
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            sandbox: Sandbox::new(root)?,
            fd_table: FdTable::new(),
        })
    }

    /// Dispatches a hostfs request message and returns the response payload.
    ///
    /// The caller is responsible for wrapping the response in an IKC `Message`.
    /// The operation identifier (`op_id`) from the request is echoed into the
    /// response so that the caller can match responses to pending operations.
    pub fn handle_request(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
    ) -> [u8; Message::PAYLOAD_SIZE] {
        let mut response: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];

        let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(*payload) {
            Ok(msg) => msg,
            Err(_) => {
                log::error!("hostfsd: invalid message header in payload");
                return response;
            },
        };

        match syscall_msg.header {
            SystemCallMessageHeader::HostFsOpenRequest => self.handle_open(payload, &mut response),
            SystemCallMessageHeader::HostFsCloseRequest => {
                self.handle_close(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsReadRequest => self.handle_read(payload, &mut response),
            SystemCallMessageHeader::HostFsWriteRequest => {
                self.handle_write(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsStatRequest => self.handle_stat(payload, &mut response),
            SystemCallMessageHeader::HostFsReadDirRequest => {
                self.handle_readdir(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsMkdirRequest => {
                self.handle_mkdir(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsRmdirRequest => {
                self.handle_rmdir(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsUnlinkRequest => {
                self.handle_unlink(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsRenameRequest => {
                self.handle_rename(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsLseekRequest => {
                self.handle_lseek(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsTruncateRequest => {
                self.handle_truncate(payload, &mut response)
            },
            SystemCallMessageHeader::HostFsFlushRequest => {
                self.handle_flush(payload, &mut response)
            },
            other => {
                log::error!("hostfsd: unexpected message header: {:?}", other);
            },
        }

        // Echo the operation identifier from the request into the response so
        // that the caller can correlate this response with the pending operation.
        set_op_id(&mut response, get_op_id(payload));

        response
    }

    fn handle_open(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: OpenRequest = OpenRequest::decode(payload);
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };

        let host_path: PathBuf = match self.sandbox.resolve(path_str) {
            Some(p) => p,
            None => {
                log::warn!("hostfsd: path traversal rejected: {:?}", path_str);
                set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        // Translate POSIX flags to Rust OpenOptions.
        let flags: i32 = req.flags;
        let mut opts: OpenOptions = OpenOptions::new();

        let rdonly: bool = (flags & O_ACCMODE) == O_RDONLY;
        let wronly: bool = (flags & O_ACCMODE) == O_WRONLY;
        let rdwr: bool = (flags & O_ACCMODE) == O_RDWR;
        let o_creat: bool = (flags & O_CREAT) != 0;
        let o_trunc: bool = (flags & O_TRUNC) != 0;
        let o_append: bool = (flags & O_APPEND) != 0;

        if rdonly || rdwr {
            opts.read(true);
        }
        if wronly || rdwr {
            opts.write(true);
        }
        if o_creat {
            opts.create(true);
        }
        if o_trunc {
            opts.truncate(true);
        }
        // Only set append when the access mode includes write. Rust's
        // `OpenOptions::append(true)` implicitly enables write access, so setting it
        // on a read-only open would incorrectly grant write permissions.
        if o_append && (wronly || rdwr) {
            opts.append(true);
        }

        // Check if this is a directory open.
        let o_directory: bool = (flags & O_DIRECTORY) != 0;

        // POSIX: open(path, O_DIRECTORY) requires distinguishing between
        // "path does not exist" (ENOENT) and "exists but not a directory" (ENOTDIR).
        if o_directory && !host_path.is_dir() {
            let code: i32 = if host_path.exists() {
                HOSTFS_ERR_NOT_DIR
            } else {
                HOSTFS_ERR_NOT_FOUND
            };
            set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
            set_payload_data(response, &code.to_le_bytes());
            return;
        }

        let is_dir: bool = o_directory || host_path.is_dir();

        // Reject write-mode opens on directories without O_DIRECTORY. Without this
        // check, opening a directory with O_RDWR silently succeeds as read-only,
        // which violates POSIX semantics.
        if is_dir && !o_directory && (wronly || rdwr) {
            set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
            set_payload_data(response, &HOSTFS_ERR_IS_DIR.to_le_bytes());
            return;
        }

        let file: File = if is_dir {
            // For directories, open as read-only. Readdir uses the stored path.
            match File::open(&host_path) {
                Ok(f) => f,
                Err(e) => {
                    log::debug!(
                        "hostfsd: directory open failed (path={:?}, kind={:?}): {}",
                        host_path,
                        e.kind(),
                        e
                    );
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
                    return;
                },
            }
        } else {
            match opts.open(&host_path) {
                Ok(f) => f,
                Err(e) => {
                    log::debug!(
                        "hostfsd: open failed (path={:?}, flags={:#x}, kind={:?}): {}",
                        host_path,
                        flags,
                        e.kind(),
                        e
                    );
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
                    return;
                },
            }
        };

        let path_string: String = host_path.to_string_lossy().into_owned();
        match self.fd_table.alloc(file, is_dir, path_string) {
            Ok(fd) => {
                let resp: OpenResponse = OpenResponse {
                    fd,
                    is_dir: if is_dir { 1 } else { 0 },
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
            },
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_IO.to_le_bytes());
            },
        }
    }

    fn handle_close(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: CloseRequest = CloseRequest::decode(payload);
        let success: bool = self.fd_table.close(req.fd);
        let status: i32 = if success { 0 } else { HOSTFS_ERR_IO };
        set_header(response, SystemCallMessageHeader::HostFsCloseResponse as u16);
        set_payload_data(response, &status.to_le_bytes());
    }

    fn handle_read(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: ReadRequest = ReadRequest::decode(payload);
        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                let resp: ReadResponse = ReadResponse {
                    bytes_read: -1,
                    data: [0u8; MAX_INLINE_READ_DATA],
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsReadResponse as u16);
                return;
            },
        };

        // Reject reads on directory file descriptors.
        if entry.is_dir {
            let resp: ReadResponse = ReadResponse {
                bytes_read: -1,
                data: [0u8; MAX_INLINE_READ_DATA],
            };
            resp.encode(response);
            set_header(response, SystemCallMessageHeader::HostFsReadResponse as u16);
            return;
        }

        // For positional reads (offset >= 0), save and restore the current position
        // to provide pread() semantics: the file position is not affected.
        let saved_pos: Option<u64> = if req.offset >= 0 {
            let pos: Option<u64> = entry.file.stream_position().ok();
            if entry.file.seek(SeekFrom::Start(req.offset as u64)).is_err() {
                let resp: ReadResponse = ReadResponse {
                    bytes_read: -1,
                    data: [0u8; MAX_INLINE_READ_DATA],
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsReadResponse as u16);
                return;
            }
            pos
        } else {
            None
        };

        let read_count: usize = (req.count as usize).min(MAX_INLINE_READ_DATA);
        let mut data: [u8; MAX_INLINE_READ_DATA] = [0u8; MAX_INLINE_READ_DATA];

        let result = entry.file.read(&mut data[..read_count]);

        // Restore position after positional read.
        if let Some(pos) = saved_pos {
            let _ = entry.file.seek(SeekFrom::Start(pos));
        }

        match result {
            Ok(n) => {
                let resp: ReadResponse = ReadResponse {
                    bytes_read: n as i32,
                    data,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsReadResponse as u16);
            },
            Err(e) => {
                log::debug!("hostfsd: read failed (fd={}, kind={:?}): {}", req.fd, e.kind(), e);
                let resp: ReadResponse = ReadResponse {
                    bytes_read: -1,
                    data: [0u8; MAX_INLINE_READ_DATA],
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsReadResponse as u16);
            },
        }
    }

    fn handle_write(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: WriteRequest = WriteRequest::decode(payload);
        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                let resp: WriteResponse = WriteResponse { bytes_written: -1 };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
                return;
            },
        };

        // Reject writes on directory file descriptors.
        if entry.is_dir {
            let resp: WriteResponse = WriteResponse { bytes_written: -1 };
            resp.encode(response);
            set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
            return;
        }

        // For positional writes (offset >= 0), save and restore the current position
        // to provide pwrite() semantics: the file position is not affected.
        let saved_pos: Option<u64> = if req.offset >= 0 {
            let pos: Option<u64> = entry.file.stream_position().ok();
            if entry.file.seek(SeekFrom::Start(req.offset as u64)).is_err() {
                let resp: WriteResponse = WriteResponse { bytes_written: -1 };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
                return;
            }
            pos
        } else {
            None
        };

        let write_count: usize = (req.data_len as usize).min(MAX_INLINE_WRITE_DATA);
        let result = entry.file.write(&req.data[..write_count]);

        // Restore position after positional write.
        if let Some(pos) = saved_pos {
            let _ = entry.file.seek(SeekFrom::Start(pos));
        }

        match result {
            Ok(n) => {
                let resp: WriteResponse = WriteResponse {
                    bytes_written: n as i32,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
            },
            Err(e) => {
                log::debug!("hostfsd: write failed (fd={}, kind={:?}): {}", req.fd, e.kind(), e);
                let resp: WriteResponse = WriteResponse { bytes_written: -1 };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
            },
        }
    }

    fn handle_stat(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: StatRequest = StatRequest::decode(payload);
        let entry: &FdEntry = match self.fd_table.get(req.fd) {
            Some(e) => e,
            None => {
                let resp: StatResponse = StatResponse {
                    status: HOSTFS_ERR_IO,
                    size: 0,
                    mode: 0,
                    is_dir: 0,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsStatResponse as u16);
                return;
            },
        };

        let path: PathBuf = PathBuf::from(&entry.path);
        match fs::metadata(&path) {
            Ok(meta) => {
                #[cfg(unix)]
                let mode: u32 = meta.permissions().mode();
                #[cfg(not(unix))]
                let mode: u32 = if meta.permissions().readonly() {
                    0o444
                } else {
                    0o644
                };
                let resp: StatResponse = StatResponse {
                    status: 0,
                    size: meta.len(),
                    mode,
                    is_dir: if meta.is_dir() { 1 } else { 0 },
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsStatResponse as u16);
            },
            Err(e) => {
                log::debug!("hostfsd: stat failed (path={:?}, kind={:?}): {}", path, e.kind(), e);
                let resp: StatResponse = StatResponse {
                    status: io_error_to_code(&e),
                    size: 0,
                    mode: 0,
                    is_dir: 0,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsStatResponse as u16);
            },
        }
    }

    fn handle_readdir(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: ReadDirRequest = ReadDirRequest::decode(payload);
        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                // Return empty readdir (name_len=0 signals end).
                set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
                set_payload_data(response, &[0u8; 2]);
                return;
            },
        };

        // Use the cached directory listing (populated on first access).
        if let Some(cached) = entry.readdir_at(req.offset as usize) {
            let name: String = cached.name.to_string_lossy().into_owned();
            let name_bytes: &[u8] = name.as_bytes();
            let name_len: usize = name_bytes.len().min(MAX_DIR_ENTRY_NAME_LEN);

            if name_bytes.len() > MAX_DIR_ENTRY_NAME_LEN {
                log::warn!("hostfsd: filename truncated in readdir: {:?}", name);
            }

            let mut entry_name: [u8; MAX_DIR_ENTRY_NAME_LEN] = [0u8; MAX_DIR_ENTRY_NAME_LEN];
            entry_name[..name_len].copy_from_slice(&name_bytes[..name_len]);

            let resp: ReadDirEntry = ReadDirEntry {
                name_len: name_len as u16,
                is_dir: if cached.is_dir { 1 } else { 0 },
                size: cached.size,
                name: entry_name,
            };
            resp.encode(response);
            set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
        } else {
            // No more entries at this offset.
            set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
            set_payload_data(response, &[0u8; 2]);
        }
    }

    fn handle_mkdir(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: MkdirRequest = MkdirRequest::decode(payload);
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };

        let host_path: PathBuf = match self.sandbox.resolve(path_str) {
            Some(p) => p,
            None => {
                log::warn!("hostfsd: mkdir path traversal rejected: {:?}", path_str);
                set_header(response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match fs::create_dir(&host_path) {
            Ok(()) => {
                self.fd_table.invalidate_dir_caches();
                set_header(response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_header(response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    fn handle_rmdir(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: RmdirRequest = RmdirRequest::decode(payload);
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsRmdirResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };

        let host_path: PathBuf = match self.sandbox.resolve(path_str) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsRmdirResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match fs::remove_dir(&host_path) {
            Ok(()) => {
                self.fd_table.invalidate_dir_caches();
                set_header(response, SystemCallMessageHeader::HostFsRmdirResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_header(response, SystemCallMessageHeader::HostFsRmdirResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    fn handle_unlink(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: UnlinkRequest = UnlinkRequest::decode(payload);
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsUnlinkResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };

        let host_path: PathBuf = match self.sandbox.resolve(path_str) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsUnlinkResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match fs::remove_file(&host_path) {
            Ok(()) => {
                self.fd_table.invalidate_dir_caches();
                set_header(response, SystemCallMessageHeader::HostFsUnlinkResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_header(response, SystemCallMessageHeader::HostFsUnlinkResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    fn handle_rename(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: RenameRequest = RenameRequest::decode(payload);
        let old_len: usize = req.old_path_len as usize;
        let new_len: usize = req.new_path_len as usize;

        // Reject if paths exceed the inline buffer capacity.
        if old_len > MAX_INLINE_PATH_LEN || old_len + new_len > MAX_INLINE_PATH_LEN {
            log::warn!("hostfsd: rename paths too long (old={old_len}, new={new_len})");
            set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
            set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
            return;
        }

        let old_str: &str = match core::str::from_utf8(&req.paths[..old_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };
        let new_str: &str = match core::str::from_utf8(&req.paths[old_len..old_len + new_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return;
            },
        };

        let old_path: PathBuf = match self.sandbox.resolve(old_str) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };
        let new_path: PathBuf = match self.sandbox.resolve(new_str) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match fs::rename(&old_path, &new_path) {
            Ok(()) => {
                self.fd_table.invalidate_dir_caches();
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                log::debug!(
                    "hostfsd: rename failed ({:?} -> {:?}, kind={:?}): {}",
                    old_path,
                    new_path,
                    e.kind(),
                    e
                );
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    fn handle_lseek(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: LseekRequest = LseekRequest::decode(payload);
        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                // Invalid FD → return EBADF-equivalent via structured error code in data.
                set_header(response, SystemCallMessageHeader::HostFsLseekResponse as u16);
                set_payload_data(response, &(HOSTFS_ERR_IO as i64).to_le_bytes());
                return;
            },
        };

        let seek_from: SeekFrom = match req.whence {
            SEEK_SET => {
                if req.offset < 0 {
                    // POSIX: SEEK_SET with negative offset → EINVAL.
                    set_header(response, SystemCallMessageHeader::HostFsLseekResponse as u16);
                    set_payload_data(response, &(HOSTFS_ERR_INVALID as i64).to_le_bytes());
                    return;
                }
                SeekFrom::Start(req.offset as u64)
            },
            SEEK_CUR => SeekFrom::Current(req.offset),
            SEEK_END => SeekFrom::End(req.offset),
            _ => {
                // Unknown whence → EINVAL.
                set_header(response, SystemCallMessageHeader::HostFsLseekResponse as u16);
                set_payload_data(response, &(HOSTFS_ERR_INVALID as i64).to_le_bytes());
                return;
            },
        };

        match entry.file.seek(seek_from) {
            Ok(pos) => {
                let resp: LseekResponse = LseekResponse { offset: pos as i64 };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsLseekResponse as u16);
            },
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsLseekResponse as u16);
                set_payload_data(response, &(HOSTFS_ERR_IO as i64).to_le_bytes());
            },
        }
    }

    fn handle_truncate(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: TruncateRequest = TruncateRequest::decode(payload);

        // Reject negative lengths (POSIX: ftruncate with negative length → EINVAL).
        if req.length < 0 {
            set_header(response, SystemCallMessageHeader::HostFsTruncateResponse as u16);
            set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
            return;
        }

        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsTruncateResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_IO.to_le_bytes());
                return;
            },
        };

        // Reject truncate on directory file descriptors.
        if entry.is_dir {
            set_header(response, SystemCallMessageHeader::HostFsTruncateResponse as u16);
            set_payload_data(response, &HOSTFS_ERR_IS_DIR.to_le_bytes());
            return;
        }

        match entry.file.set_len(req.length as u64) {
            Ok(()) => {
                set_header(response, SystemCallMessageHeader::HostFsTruncateResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_header(response, SystemCallMessageHeader::HostFsTruncateResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    fn handle_flush(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: FlushRequest = FlushRequest::decode(payload);
        let entry: &mut FdEntry = match self.fd_table.get_mut(req.fd) {
            Some(e) => e,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsFlushResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_IO.to_le_bytes());
                return;
            },
        };

        match entry.file.flush() {
            Ok(()) => {
                set_header(response, SystemCallMessageHeader::HostFsFlushResponse as u16);
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_header(response, SystemCallMessageHeader::HostFsFlushResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }
}

/// Converts an `io::Error` to a simple integer error code.
///
/// These codes are defined in the `hostfs-api` crate as `HOSTFS_ERR_*` constants.
fn io_error_to_code(e: &io::Error) -> i32 {
    match e.kind() {
        io::ErrorKind::NotFound => HOSTFS_ERR_NOT_FOUND,
        io::ErrorKind::PermissionDenied => HOSTFS_ERR_PERMISSION,
        io::ErrorKind::AlreadyExists => HOSTFS_ERR_EXISTS,
        io::ErrorKind::InvalidInput => HOSTFS_ERR_INVALID,
        io::ErrorKind::NotADirectory => HOSTFS_ERR_NOT_DIR,
        io::ErrorKind::IsADirectory => HOSTFS_ERR_IS_DIR,
        io::ErrorKind::DirectoryNotEmpty => HOSTFS_ERR_NOT_EMPTY,
        _ => HOSTFS_ERR_IO,
    }
}

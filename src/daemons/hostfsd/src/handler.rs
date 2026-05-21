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
use ::hostfs_api::{
    long_msg,
    *,
};
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
    message::SystemCallMessagePart,
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
    /// Assembler for multi-part request messages.
    assembler: HostFsAssembler,
}

impl HostFsHandler {
    /// Creates a new handler with the given root directory.
    ///
    /// Returns an error if the root directory does not exist or is not a directory.
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            sandbox: Sandbox::new(root)?,
            fd_table: FdTable::new(),
            assembler: HostFsAssembler::new(),
        })
    }

    /// Dispatches a hostfs request message and returns the response payload.
    ///
    /// Returns `Some(response)` for complete single-message requests and for multi-part
    /// requests whose final part has arrived. Returns `None` for intermediate parts that
    /// are still being accumulated.
    ///
    /// The caller is responsible for wrapping the response in an IKC `Message`.
    /// The operation identifier (`op_id`) from the request is echoed into the
    /// response so that the caller can match responses to pending operations.
    pub fn handle_request(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
    ) -> Option<[u8; Message::PAYLOAD_SIZE]> {
        let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(*payload) {
            Ok(msg) => msg,
            Err(_) => {
                log::error!("hostfsd: invalid message header in payload");
                let mut response = [0u8; Message::PAYLOAD_SIZE];
                set_op_id(&mut response, get_op_id(payload));
                // The raw u16 header didn't parse into a known variant, so we cannot
                // derive the matching response header. Set a generic hostfs response
                // header (HostFsOpenResponse) so vfsd recognizes the frame as a
                // hostfs response and safely drops/logs it, instead of ignoring a
                // frame whose zeroed header decodes as `OpenAtRequestPart` and
                // potentially leaving a pending op stuck.
                set_header(&mut response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return Some(response);
            },
        };

        // Helper: run a single-message handler, echo op_id, and wrap the result.
        let run =
            |this: &mut Self,
             f: fn(&mut Self, &[u8; Message::PAYLOAD_SIZE], &mut [u8; Message::PAYLOAD_SIZE])|
             -> Option<[u8; Message::PAYLOAD_SIZE]> {
                let mut response = [0u8; Message::PAYLOAD_SIZE];
                f(this, payload, &mut response);
                set_op_id(&mut response, get_op_id(payload));
                Some(response)
            };

        match syscall_msg.header {
            // Multi-part request messages: accumulate parts.
            SystemCallMessageHeader::HostFsOpenRequestPart
            | SystemCallMessageHeader::HostFsRenameRequestPart
            | SystemCallMessageHeader::HostFsUnlinkRequestPart
            | SystemCallMessageHeader::HostFsMkdirRequestPart
            | SystemCallMessageHeader::HostFsRmdirRequestPart => {
                self.handle_long_part(syscall_msg.header, &syscall_msg.payload)
            },
            // Single-message requests: dispatch inline.
            //
            // NOTE: `HostFsOpenRequest`, `HostFsMkdirRequest`, `HostFsRmdirRequest`,
            // `HostFsUnlinkRequest`, and `HostFsRenameRequest` are legacy single-message
            // request variants for path-bearing operations. In production `vfsd` always
            // sends these as multi-part `*RequestPart` messages (above) to lift the
            // 36-byte inline path limit. The single-message arms are retained because
            // `handler_test` still exercises them directly via in-process payloads.
            SystemCallMessageHeader::HostFsOpenRequest => run(self, Self::handle_open),
            SystemCallMessageHeader::HostFsCloseRequest => run(self, Self::handle_close),
            SystemCallMessageHeader::HostFsReadRequest => run(self, Self::handle_read),
            SystemCallMessageHeader::HostFsWriteRequest => run(self, Self::handle_write),
            SystemCallMessageHeader::HostFsStatRequest => run(self, Self::handle_stat),
            SystemCallMessageHeader::HostFsReadDirRequest => run(self, Self::handle_readdir),
            SystemCallMessageHeader::HostFsMkdirRequest => run(self, Self::handle_mkdir),
            SystemCallMessageHeader::HostFsRmdirRequest => run(self, Self::handle_rmdir),
            SystemCallMessageHeader::HostFsUnlinkRequest => run(self, Self::handle_unlink),
            SystemCallMessageHeader::HostFsRenameRequest => run(self, Self::handle_rename),
            SystemCallMessageHeader::HostFsLseekRequest => run(self, Self::handle_lseek),
            SystemCallMessageHeader::HostFsTruncateRequest => run(self, Self::handle_truncate),
            SystemCallMessageHeader::HostFsFlushRequest => run(self, Self::handle_flush),
            other => {
                log::error!("hostfsd: unexpected message header: {:?}", other);
                let mut response = [0u8; Message::PAYLOAD_SIZE];
                set_op_id(&mut response, get_op_id(payload));
                if let Some(resp_header) = other.hostfs_response_header() {
                    set_header(&mut response, resp_header as u16);
                }
                set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                Some(response)
            },
        }
    }

    /// Handles a multi-part request message part.
    ///
    /// Accumulates the part in the assembler. When all parts have arrived,
    /// deserializes the complete request and dispatches to the appropriate handler.
    /// Returns `None` for intermediate parts and `Some(response)` for the final part
    /// or when a fatal assembly error is detected.
    fn handle_long_part(
        &mut self,
        header: SystemCallMessageHeader,
        syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
    ) -> Option<[u8; Message::PAYLOAD_SIZE]> {
        let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(*syscall_payload);

        match self.assembler.add_part(header, part) {
            AssemblyStatus::NeedMore => return None,
            AssemblyStatus::Error => {
                // Fatal assembly error — return an error response so vfsd can drain
                // its pending queue. The assembler preserves any buffered bytes and
                // the recorded header from the in-flight stream so that we can echo
                // the original op_id (read from the first 4 bytes of part 0, when
                // available) and the matching response header. This keeps vfsd's
                // pending-op tracking consistent in the face of malformed streams.
                let recorded_header: SystemCallMessageHeader = self.assembler.recorded_header();
                let buffered: Vec<u8> = self.assembler.take_assembled();
                let op_id: OperationId = if buffered.len() >= 4 {
                    OperationId::from_le_bytes([buffered[0], buffered[1], buffered[2], buffered[3]])
                } else {
                    OperationId::INVALID
                };
                let mut response = [0u8; Message::PAYLOAD_SIZE];
                set_op_id(&mut response, op_id);
                // Prefer the response header for the recorded (first-part) request
                // type; fall back to the current part's header if no part has been
                // accepted yet (e.g., total_parts==0 on the very first part).
                let resp_header = recorded_header
                    .hostfs_response_header()
                    .or_else(|| header.hostfs_response_header());
                if let Some(resp_header) = resp_header {
                    set_header(&mut response, resp_header as u16);
                }
                set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                return Some(response);
            },
            AssemblyStatus::Complete => { /* fall through to dispatch */ },
        }

        // All parts received — assemble and dispatch.
        // Use the header recorded on the first part for dispatch (not the current part's header)
        // to prevent a malformed stream from causing a wrong-format deserialization.
        let dispatch_header: SystemCallMessageHeader = self.assembler.recorded_header();
        let assembled: Vec<u8> = self.assembler.take_assembled();
        let mut response = [0u8; Message::PAYLOAD_SIZE];

        // All wire formats start with op_id (4 bytes LE). Extract it unconditionally
        // so that error responses can echo it back to vfsd and avoid orphaning pending ops.
        let op_id: OperationId = if assembled.len() >= 4 {
            OperationId::from_le_bytes([assembled[0], assembled[1], assembled[2], assembled[3]])
        } else {
            OperationId::INVALID
        };
        set_op_id(&mut response, op_id);

        match dispatch_header {
            SystemCallMessageHeader::HostFsOpenRequestPart => {
                if let Some(req) = long_msg::deserialize_long_open(&assembled) {
                    self.handle_long_open(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long open request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsRenameRequestPart => {
                if let Some(req) = long_msg::deserialize_long_rename(&assembled) {
                    self.handle_long_rename(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long rename request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsUnlinkRequestPart => {
                if let Some(req) = long_msg::deserialize_long_unlink(&assembled) {
                    self.handle_long_unlink(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long unlink request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsUnlinkResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsMkdirRequestPart => {
                if let Some(req) = long_msg::deserialize_long_mkdir(&assembled) {
                    self.handle_long_mkdir(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long mkdir request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsRmdirRequestPart => {
                if let Some(req) = long_msg::deserialize_long_rmdir(&assembled) {
                    self.handle_long_rmdir(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long rmdir request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsRmdirResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            _ => {
                log::error!("hostfsd: unexpected assembled header: {:?}", dispatch_header);
                if let Some(resp_header) = dispatch_header.hostfs_response_header() {
                    set_header(&mut response, resp_header as u16);
                }
                set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
            },
        }

        Some(response)
    }

    /// Handles a fully assembled long OPEN request.
    fn handle_long_open(
        &mut self,
        req: long_msg::LongOpenRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let host_path: PathBuf = match self.sandbox.resolve(&req.path) {
            Some(p) => p,
            None => {
                log::warn!("hostfsd: path traversal rejected: {:?}", req.path);
                set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

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
        if o_append && (wronly || rdwr) {
            opts.append(true);
        }

        let o_directory: bool = (flags & O_DIRECTORY) != 0;

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

        if is_dir && !o_directory && (wronly || rdwr) {
            set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
            set_payload_data(response, &HOSTFS_ERR_IS_DIR.to_le_bytes());
            return;
        }

        let file: File = if is_dir {
            match open_dir_handle(&host_path) {
                Ok(f) => f,
                Err(e) => {
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
                    return;
                },
            }
        } else {
            match opts.open(&host_path) {
                Ok(f) => f,
                Err(e) => {
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

    /// Handles a fully assembled long RENAME request.
    fn handle_long_rename(
        &mut self,
        req: long_msg::LongRenameRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let old_path: PathBuf = match self.sandbox.resolve(&req.old_path) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };
        let new_path: PathBuf = match self.sandbox.resolve(&req.new_path) {
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
                    "hostfsd: long rename failed ({:?} -> {:?}): {}",
                    old_path,
                    new_path,
                    e
                );
                set_header(response, SystemCallMessageHeader::HostFsRenameResponse as u16);
                set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
            },
        }
    }

    /// Handles a fully assembled long UNLINK request.
    fn handle_long_unlink(
        &mut self,
        req: long_msg::LongUnlinkRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let host_path: PathBuf = match self.sandbox.resolve(&req.path) {
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

    /// Handles a fully assembled long MKDIR request.
    fn handle_long_mkdir(
        &mut self,
        req: long_msg::LongMkdirRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let host_path: PathBuf = match self.sandbox.resolve(&req.path) {
            Some(p) => p,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsMkdirResponse as u16);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match fs::create_dir(&host_path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    let perms = std::fs::Permissions::from_mode(req.mode);
                    let _ = fs::set_permissions(&host_path, perms);
                }
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

    /// Handles a fully assembled long RMDIR request.
    fn handle_long_rmdir(
        &mut self,
        req: long_msg::LongRmdirRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let host_path: PathBuf = match self.sandbox.resolve(&req.path) {
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
            match open_dir_handle(&host_path) {
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

/// Opens a directory as a [`File`] in a cross-platform way.
///
/// On Unix, `File::open()` works for directories. On Windows, `File::open()`
/// fails with `PermissionDenied` for directories because the underlying
/// `CreateFileW` call requires `FILE_FLAG_BACKUP_SEMANTICS` to open a
/// directory handle. This helper sets that flag on Windows so that directory
/// handles (e.g., dirfds opened with `O_DIRECTORY`) can be obtained.
fn open_dir_handle(path: &std::path::Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_BACKUP_SEMANTICS is required to obtain a handle on a
        // directory via CreateFileW.
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
    }
    #[cfg(not(windows))]
    {
        File::open(path)
    }
}

//==================================================================================================
// Multi-part Message Assembler
//==================================================================================================

/// Maximum buffer size for any valid long request.
///
/// The largest wire format is RENAME: `RENAME_HEADER_SIZE + 2 * MAX_PATH_LEN`.
const MAX_LONG_BUFFER_SIZE: usize = long_msg::RENAME_HEADER_SIZE + 2 * long_msg::MAX_PATH_LEN;

/// Maximum allowed value of `total_parts` for a long request.
///
/// Senders compute `total_parts` with `div_ceil(buffer.len(), PAYLOAD_SIZE)`, so the
/// theoretical bound is `ceil(MAX_LONG_BUFFER_SIZE / PAYLOAD_SIZE)`. Rejecting on
/// `total_parts * PAYLOAD_SIZE > MAX_LONG_BUFFER_SIZE` would spuriously refuse valid
/// maximum-sized requests whose final part is only partially filled. Per-part overflow
/// is enforced separately while appending payload bytes (see [`HostFsAssembler::add_part`]).
const MAX_TOTAL_PARTS: u16 =
    MAX_LONG_BUFFER_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE) as u16;

/// Result of attempting to add a part to the assembler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssemblyStatus {
    /// More parts are needed to complete the request.
    NeedMore,
    /// All parts received — the request is ready for dispatch.
    Complete,
    /// A fatal assembly error occurred (e.g., out-of-order parts, buffer overflow).
    ///
    /// On error, the assembler state (recorded header and buffered bytes) is
    /// *preserved*, not reset. This lets the caller recover the in-flight
    /// `op_id` (from the first 4 bytes of part 0, when available) and the
    /// recorded header to build a well-formed error response. The caller must
    /// explicitly drain the assembler afterwards via
    /// [`HostFsAssembler::take_assembled`] (or [`HostFsAssembler::reset`]).
    Error,
}

/// Assembles multi-part hostfs IKC request messages.
///
/// Since vfsd sends all parts of a request sequentially before starting another
/// multi-part operation (single-threaded event loop on guest side), we only need
/// to track a single in-flight multi-part request at a time.
struct HostFsAssembler {
    /// Header of the request currently being assembled (recorded from the first part).
    header: Option<SystemCallMessageHeader>,
    /// Total number of parts expected.
    total_parts: u16,
    /// Number of parts received so far.
    parts_received: u16,
    /// Expected part_number for the next part (sequential ordering).
    next_part_number: u16,
    /// Accumulated payload bytes.
    buffer: Vec<u8>,
}

impl HostFsAssembler {
    fn new() -> Self {
        Self {
            header: None,
            total_parts: 0,
            parts_received: 0,
            next_part_number: 0,
            buffer: Vec::new(),
        }
    }

    /// Returns the header recorded from the first part of the current request.
    ///
    /// This is used for dispatch after assembly is complete so that a malformed
    /// stream with inconsistent headers does not cause a wrong-format deserialization.
    fn recorded_header(&self) -> SystemCallMessageHeader {
        self.header
            .unwrap_or(SystemCallMessageHeader::HostFsOpenRequestPart)
    }

    /// Adds a part to the assembler.
    ///
    /// Returns [`AssemblyStatus::Complete`] if all parts have been received.
    /// Returns [`AssemblyStatus::NeedMore`] if more parts are needed.
    /// Returns [`AssemblyStatus::Error`] on a fatal assembly error. On error, the
    /// assembler's recorded header and buffered bytes are preserved so the caller
    /// can recover the in-flight `op_id` (from the first 4 bytes of part 0) and the
    /// recorded header in order to build a well-formed error response. The caller
    /// is responsible for clearing the assembler state afterwards via
    /// [`HostFsAssembler::take_assembled`].
    ///
    /// Validates that:
    /// - `total_parts` is non-zero.
    /// - The resulting buffer would not exceed `MAX_LONG_BUFFER_SIZE`.
    /// - The header matches the in-flight request.
    /// - `part_number` matches the expected sequential order.
    /// - `total_parts` is consistent across all parts in a stream.
    fn add_part(
        &mut self,
        header: SystemCallMessageHeader,
        part: SystemCallMessagePart,
    ) -> AssemblyStatus {
        let total: u16 = part.total_parts;
        let number: u16 = part.part_number;
        let size: u8 = part.payload_size;

        // Reject requests with zero total_parts.
        if total == 0 {
            log::error!("hostfsd: assembler received part with total_parts=0, dropping");
            return AssemblyStatus::Error;
        }

        // Cap total_parts to prevent excessive allocations. Use the ceiling-based bound
        // so that valid maximum-sized requests (whose last part may be partially filled)
        // are not spuriously rejected. Per-part overflow is enforced below while
        // appending payload bytes.
        if total > MAX_TOTAL_PARTS {
            log::error!(
                "hostfsd: assembler total_parts={} exceeds maximum {}, dropping",
                total,
                MAX_TOTAL_PARTS
            );
            return AssemblyStatus::Error;
        }

        // A stray `part_number == 0` arriving mid-stream means a new request has
        // started before the in-flight one completed. vfsd's single-threaded,
        // sequential send loop makes this unexpected, but if it ever happens we
        // must not silently discard the in-flight bytes: doing so would orphan the
        // previous op_id on vfsd's pending-op table. Instead, surface an error so
        // the caller can recover the in-flight op_id from the buffered bytes and
        // emit a well-formed error response. The new part is dropped; the caller
        // is expected to drain the assembler via `take_assembled()` afterwards.
        if self.header.is_some() && number == 0 && self.parts_received < self.total_parts {
            log::error!(
                "hostfsd: assembler received new part_number=0 ({:?}) while {}/{} parts of {:?} \
                 were still in flight; failing in-flight request",
                header,
                self.parts_received,
                self.total_parts,
                self.header,
            );
            return AssemblyStatus::Error;
        }

        // Check if this is the first part of a new request.
        if self.header.is_none() {
            self.header = Some(header);
            self.total_parts = total;
            self.parts_received = 0;
            self.next_part_number = 0;
            self.buffer.clear();
            self.buffer.reserve(
                MAX_LONG_BUFFER_SIZE.min(total as usize * SystemCallMessagePart::PAYLOAD_SIZE),
            );
        } else if self.header != Some(header) {
            // Header mismatch: the stream is malformed. Preserve buffered bytes and
            // the recorded header so the caller can echo the original op_id.
            log::error!(
                "hostfsd: assembler header mismatch (expected {:?}, got {:?})",
                self.header,
                header
            );
            return AssemblyStatus::Error;
        }

        // Validate that total_parts is consistent with the first part's value.
        if total != self.total_parts {
            log::error!(
                "hostfsd: assembler total_parts mismatch (expected {}, got {})",
                self.total_parts,
                total
            );
            return AssemblyStatus::Error;
        }

        // Validate sequential part_number ordering.
        if number != self.next_part_number {
            log::error!(
                "hostfsd: assembler part_number out of order (expected {}, got {})",
                self.next_part_number,
                number
            );
            return AssemblyStatus::Error;
        }

        // Append the payload bytes, enforcing the cumulative buffer cap so a stream
        // whose advertised `total_parts` is within bounds but whose declared payload
        // sizes overflow `MAX_LONG_BUFFER_SIZE` is rejected.
        let payload_len: usize = (size as usize).min(SystemCallMessagePart::PAYLOAD_SIZE);
        if self.buffer.len().saturating_add(payload_len) > MAX_LONG_BUFFER_SIZE {
            log::error!(
                "hostfsd: assembler buffer would exceed max size {} (have {}, +{}), dropping",
                MAX_LONG_BUFFER_SIZE,
                self.buffer.len(),
                payload_len,
            );
            return AssemblyStatus::Error;
        }
        self.buffer.extend_from_slice(&part.payload[..payload_len]);
        self.parts_received += 1;
        self.next_part_number += 1;

        if self.parts_received >= self.total_parts {
            AssemblyStatus::Complete
        } else {
            AssemblyStatus::NeedMore
        }
    }

    /// Takes the assembled bytes, resetting the assembler for the next request.
    ///
    /// The caller should read `recorded_header()` before calling this method,
    /// as it resets the recorded header.
    fn take_assembled(&mut self) -> Vec<u8> {
        let buf = std::mem::take(&mut self.buffer);
        self.reset();
        buf
    }

    /// Resets the assembler state.
    fn reset(&mut self) {
        self.header = None;
        self.total_parts = 0;
        self.parts_received = 0;
        self.next_part_number = 0;
        self.buffer.clear();
    }
}

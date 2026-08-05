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
            O_EXCL,
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

fn open_regular_file(
    options: &OpenOptions,
    host_path: &PathBuf,
    create: bool,
    exclusive: bool,
) -> io::Result<(File, bool)> {
    if !create {
        return options.open(host_path).map(|file| (file, false));
    }

    let mut create_options: OpenOptions = options.clone();
    create_options.create_new(true);

    let mut existing_options: OpenOptions = options.clone();
    existing_options.create(false);

    loop {
        match create_options.open(host_path) {
            Ok(file) => return Ok((file, true)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && !exclusive => {
                match existing_options.open(host_path) {
                    Ok(file) => return Ok((file, false)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {},
                    Err(error) => return Err(error),
                }
            },
            Err(error) => return Err(error),
        }
    }
}

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
    /// Queue of additional response payloads that a single request may produce.
    ///
    /// Most hostfs operations reply with a single message, but a few (currently the
    /// long-target variant of `readlink`) emit a multi-part response. The first part
    /// is returned directly from [`Self::handle_request`]; the remaining parts are
    /// queued here and must be drained by the caller via
    /// [`Self::take_next_response_part`] before submitting the next request.
    extra_responses: std::collections::VecDeque<[u8; Message::PAYLOAD_SIZE]>,
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
            extra_responses: std::collections::VecDeque::new(),
        })
    }

    /// Pops and returns the next queued response part, if any.
    ///
    /// After a call to [`Self::handle_request`] returns `Some(payload)`, callers must
    /// repeatedly invoke this method (sending each returned payload as its own IKC
    /// message) until it returns `None`. Only then is the next request safe to feed
    /// in. This contract preserves the strict in-order arrival of response parts on
    /// the vfsd side, which is required for the multi-part response assembler to
    /// keep a single in-flight stream.
    pub fn take_next_response_part(&mut self) -> Option<[u8; Message::PAYLOAD_SIZE]> {
        self.extra_responses.pop_front()
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
        //
        // Multi-part response builders stamp the op_id on every outer frame and also retain it in
        // the assembled response body. Single-message handlers are stamped here.
        let run =
            |this: &mut Self,
             f: fn(&mut Self, &[u8; Message::PAYLOAD_SIZE], &mut [u8; Message::PAYLOAD_SIZE])|
             -> Option<[u8; Message::PAYLOAD_SIZE]> {
                let mut response = [0u8; Message::PAYLOAD_SIZE];
                f(this, payload, &mut response);
                let resp_header_raw: u16 = u16::from_ne_bytes([response[0], response[1]]);
                let is_multipart: bool = SystemCallMessageHeader::try_from(resp_header_raw)
                    .map(|h| h.is_hostfs_multipart_response())
                    .unwrap_or(false);
                if !is_multipart {
                    set_op_id(&mut response, get_op_id(payload));
                }
                Some(response)
            };

        match syscall_msg.header {
            // Multi-part request messages: accumulate parts.
            SystemCallMessageHeader::HostFsOpenRequestPart
            | SystemCallMessageHeader::HostFsRenameRequestPart
            | SystemCallMessageHeader::HostFsUnlinkRequestPart
            | SystemCallMessageHeader::HostFsMkdirRequestPart
            | SystemCallMessageHeader::HostFsRmdirRequestPart
            | SystemCallMessageHeader::HostFsSymlinkRequestPart
            | SystemCallMessageHeader::HostFsReadlinkRequestPart
            | SystemCallMessageHeader::HostFsLstatRequestPart
            | SystemCallMessageHeader::HostFsPathStatRequestPart => self.handle_long_part(
                syscall_msg.header,
                OperationId::from_le_bytes(syscall_msg.request_id.to_le_bytes()),
                &syscall_msg.payload,
            ),
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
            SystemCallMessageHeader::HostFsReadlinkRequest => run(self, Self::handle_readlink),
            SystemCallMessageHeader::HostFsLstatRequest => run(self, Self::handle_lstat),
            SystemCallMessageHeader::HostFsPathStatRequest => run(self, Self::handle_pathstat),
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
        request_id: OperationId,
        syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
    ) -> Option<[u8; Message::PAYLOAD_SIZE]> {
        let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(*syscall_payload);

        let assembly_status: AssemblyStatus = self.assembler.add_part(header, request_id, part);
        match assembly_status {
            AssemblyStatus::NeedMore => return None,
            AssemblyStatus::Interrupted | AssemblyStatus::Error => {
                // Fatal assembly error — return an error response so vfsd can drain
                // its pending queue. The assembler preserves any buffered bytes and
                // the recorded header from the in-flight stream so that we can echo
                // the original op_id (read from the first 4 bytes of part 0, when
                // available) and the matching response header. This keeps vfsd's
                // pending-op tracking consistent in the face of malformed streams.
                let recorded_header: SystemCallMessageHeader = self.assembler.recorded_header();
                let recorded_request_id: OperationId = self.assembler.recorded_request_id();
                let op_id: OperationId = if recorded_request_id == OperationId::INVALID {
                    request_id
                } else {
                    recorded_request_id
                };
                let _buffered: Vec<u8> = self.assembler.take_assembled();
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
                if assembly_status == AssemblyStatus::Interrupted {
                    // The current part starts a new request. Replay it against the reset
                    // assembler and queue any immediate response after this in-flight error.
                    // `push_front()` keeps a multipart response head ahead of any tail parts that
                    // the recursive dispatch queued in `extra_responses`.
                    if let Some(next_response) =
                        self.handle_long_part(header, request_id, syscall_payload)
                    {
                        self.extra_responses.push_front(next_response);
                    }
                }
                return Some(response);
            },
            AssemblyStatus::Complete => { /* fall through to dispatch */ },
        }

        // All parts received — assemble and dispatch.
        // Use the header recorded on the first part for dispatch (not the current part's header)
        // to prevent a malformed stream from causing a wrong-format deserialization.
        let dispatch_header: SystemCallMessageHeader = self.assembler.recorded_header();
        let request_id: OperationId = self.assembler.recorded_request_id();
        let assembled: Vec<u8> = self.assembler.take_assembled();
        let mut response = [0u8; Message::PAYLOAD_SIZE];

        // All wire formats start with op_id (4 bytes LE). Extract it unconditionally
        // so that error responses can echo it back to vfsd and avoid orphaning pending ops.
        let op_id: OperationId = if assembled.len() >= 4 {
            OperationId::from_le_bytes([assembled[0], assembled[1], assembled[2], assembled[3]])
        } else {
            OperationId::INVALID
        };
        if op_id != request_id {
            log::error!(
                "hostfsd: assembled request identifier mismatch (outer={}, body={})",
                request_id,
                op_id
            );
            set_op_id(&mut response, request_id);
            if let Some(resp_header) = dispatch_header.hostfs_response_header() {
                set_header(&mut response, resp_header as u16);
            }
            set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
            return Some(response);
        }
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
            SystemCallMessageHeader::HostFsSymlinkRequestPart => {
                if let Some(req) = long_msg::deserialize_long_symlink(&assembled) {
                    self.handle_long_symlink(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long symlink request");
                    set_header(
                        &mut response,
                        SystemCallMessageHeader::HostFsSymlinkResponse as u16,
                    );
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsReadlinkRequestPart => {
                if let Some(req) = long_msg::deserialize_long_readlink(&assembled) {
                    self.handle_long_readlink(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long readlink request");
                    set_header(
                        &mut response,
                        SystemCallMessageHeader::HostFsReadlinkResponse as u16,
                    );
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsLstatRequestPart => {
                if let Some(req) = long_msg::deserialize_long_lstat(&assembled) {
                    self.handle_long_lstat(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long lstat request");
                    set_header(&mut response, SystemCallMessageHeader::HostFsLstatResponse as u16);
                    set_payload_data(&mut response, &HOSTFS_ERR_INVALID.to_le_bytes());
                }
            },
            SystemCallMessageHeader::HostFsPathStatRequestPart => {
                if let Some(req) = long_msg::deserialize_long_lstat(&assembled) {
                    self.handle_long_pathstat(req, &mut response);
                } else {
                    log::error!("hostfsd: failed to deserialize long pathstat request");
                    set_header(
                        &mut response,
                        SystemCallMessageHeader::HostFsPathStatResponse as u16,
                    );
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
        let o_excl: bool = (flags & O_EXCL) != 0;
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

        let (file, created): (File, bool) = if is_dir {
            match open_dir_handle(&host_path) {
                Ok(file) => (file, false),
                Err(e) => {
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
                    return;
                },
            }
        } else {
            match open_regular_file(&opts, &host_path, o_creat, o_excl) {
                Ok(result) => result,
                Err(e) => {
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&e).to_le_bytes());
                    return;
                },
            }
        };

        if created {
            #[cfg(unix)]
            {
                let permissions: std::fs::Permissions = std::fs::Permissions::from_mode(req.mode);
                if let Err(error) = file.set_permissions(permissions) {
                    drop(file);
                    let _ = fs::remove_file(&host_path);
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&error).to_le_bytes());
                    return;
                }
            }
        }

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
        let host_path: PathBuf = match self.sandbox.resolve_nofollow(&req.path) {
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

    /// Handles a fully assembled long SYMLINK request.
    ///
    /// Creates a symbolic link at `linkpath` pointing to the verbatim `target` string.
    /// The `target` is not sandbox-validated at creation time (POSIX semantics: a link's
    /// target is stored as-is and only resolved on dereference). However, when the link
    /// is later opened/stat'd through hostfsd, [`Sandbox::resolve`] will reject any
    /// access that escapes the mount root, so an out-of-sandbox target simply produces
    /// an unusable link rather than a security hole.
    ///
    /// The `linkpath` is resolved with [`Sandbox::resolve_nofollow`] so an existing
    /// link at that name is not followed.
    fn handle_long_symlink(
        &mut self,
        req: long_msg::LongSymlinkRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        set_header(response, SystemCallMessageHeader::HostFsSymlinkResponse as u16);

        // Reject empty target and embedded NUL bytes — these are invalid as POSIX
        // pathnames and would silently corrupt the link.
        if req.target.is_empty() || req.target.as_bytes().contains(&0) {
            set_payload_data(response, &HOSTFS_ERR_INVALID.to_le_bytes());
            return;
        }

        let link_path: PathBuf = match self.sandbox.resolve_nofollow(&req.linkpath) {
            Some(p) => p,
            None => {
                log::warn!("hostfsd: symlink linkpath escapes sandbox: {:?}", req.linkpath);
                set_payload_data(response, &HOSTFS_ERR_PERMISSION.to_le_bytes());
                return;
            },
        };

        match create_symlink(&req.target, &link_path) {
            Ok(()) => {
                self.fd_table.invalidate_dir_caches();
                set_payload_data(response, &0i32.to_le_bytes());
            },
            Err(e) => {
                set_payload_data(response, &symlink_error_to_code(&e).to_le_bytes());
            },
        }
    }

    /// Handles a fully assembled long READLINK request.
    fn handle_long_readlink(
        &mut self,
        req: long_msg::LongReadlinkRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        self.do_readlink(req.op_id, &req.path, response);
    }

    /// Handles a fully assembled long LSTAT request.
    fn handle_long_lstat(
        &mut self,
        req: long_msg::LongLstatRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        self.do_lstat(&req.path, response);
    }

    /// Handles a fully assembled long path-based following STAT request.
    ///
    /// Reuses the lstat wire format ([`long_msg::LongLstatRequest`]) because the request
    /// shape is identical (a single path). The distinguishing behavior is in
    /// [`Self::do_pathstat`], which follows symbolic links.
    fn handle_long_pathstat(
        &mut self,
        req: long_msg::LongLstatRequest,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        self.do_pathstat(&req.path, response);
    }

    /// Handles an inline single-message READLINK request.
    fn handle_readlink(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: ReadlinkRequest = match ReadlinkRequest::decode(payload) {
            Some(r) => r,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsReadlinkResponse as u16);
                let err = ReadlinkResponse {
                    status: HOSTFS_ERR_INVALID,
                    target_len: 0,
                    target: [0u8; MAX_INLINE_READLINK_TARGET],
                };
                err.encode(response);
                return;
            },
        };
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsReadlinkResponse as u16);
                let err = ReadlinkResponse {
                    status: HOSTFS_ERR_INVALID,
                    target_len: 0,
                    target: [0u8; MAX_INLINE_READLINK_TARGET],
                };
                err.encode(response);
                return;
            },
        };
        self.do_readlink(get_op_id(payload), path_str, response);
    }

    /// Handles an inline single-message LSTAT request.
    fn handle_lstat(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: LstatRequest = match LstatRequest::decode(payload) {
            Some(r) => r,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsLstatResponse as u16);
                let err = LstatResponse {
                    status: HOSTFS_ERR_INVALID,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsLstatResponse as u16);
                let err = LstatResponse {
                    status: HOSTFS_ERR_INVALID,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };
        self.do_lstat(path_str, response);
    }

    /// Handles an inline single-message path-based following STAT request.
    ///
    /// Reuses the [`LstatRequest`] wire format (a single inline path). Unlike
    /// [`Self::handle_lstat`], the resolution follows symbolic links (see
    /// [`Self::do_pathstat`]).
    fn handle_pathstat(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: LstatRequest = match LstatRequest::decode(payload) {
            Some(r) => r,
            None => {
                set_header(response, SystemCallMessageHeader::HostFsPathStatResponse as u16);
                let err = LstatResponse {
                    status: HOSTFS_ERR_INVALID,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };
        let path_len: usize = (req.path_len as usize).min(MAX_INLINE_PATH_LEN);
        let path_str: &str = match core::str::from_utf8(&req.path[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                set_header(response, SystemCallMessageHeader::HostFsPathStatResponse as u16);
                let err = LstatResponse {
                    status: HOSTFS_ERR_INVALID,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };
        self.do_pathstat(path_str, response);
    }

    /// Shared `readlink` implementation used by both the inline and multi-part
    /// request handlers.
    ///
    /// `op_id` is the request's operation identifier. It is required because a
    /// successful read of a long target produces a multi-part response whose body
    /// embeds the op_id at a fixed offset for vfsd-side routing — callers (inline or
    /// long path) cannot rely on `set_op_id` having been applied to `response` yet.
    fn do_readlink(
        &mut self,
        op_id: OperationId,
        path_str: &str,
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        set_header(response, SystemCallMessageHeader::HostFsReadlinkResponse as u16);

        let host_path: PathBuf = match self.sandbox.resolve_nofollow(path_str) {
            Some(p) => p,
            None => {
                let err = ReadlinkResponse {
                    status: HOSTFS_ERR_PERMISSION,
                    target_len: 0,
                    target: [0u8; MAX_INLINE_READLINK_TARGET],
                };
                err.encode(response);
                return;
            },
        };

        // Refuse to follow any symlink before reading: `read_link` itself never follows
        // the final component, but `symlink_metadata` lets us produce ENOENT for missing
        // paths and EINVAL for non-symlinks (matching POSIX readlink semantics).
        let meta = match fs::symlink_metadata(&host_path) {
            Ok(m) => m,
            Err(e) => {
                let err = ReadlinkResponse {
                    status: io_error_to_code(&e),
                    target_len: 0,
                    target: [0u8; MAX_INLINE_READLINK_TARGET],
                };
                err.encode(response);
                return;
            },
        };
        if !meta.file_type().is_symlink() {
            let err = ReadlinkResponse {
                status: HOSTFS_ERR_INVALID,
                target_len: 0,
                target: [0u8; MAX_INLINE_READLINK_TARGET],
            };
            err.encode(response);
            return;
        }

        match fs::read_link(&host_path) {
            Ok(target_path) => {
                // Preserve the verbatim target bytes on Unix: `to_string_lossy` would
                // replace invalid UTF-8 sequences with U+FFFD, silently corrupting
                // non-UTF-8 link targets. On Windows, paths are natively UTF-16 and the
                // hostfs wire format is byte-oriented, so a lossy UTF-8 conversion is
                // the closest faithful representation available.
                #[cfg(unix)]
                let target_owned: Vec<u8> = {
                    use std::os::unix::ffi::OsStrExt;
                    target_path.as_os_str().as_bytes().to_vec()
                };
                #[cfg(not(unix))]
                let target_owned: Vec<u8> = target_path.to_string_lossy().into_owned().into_bytes();
                let target_bytes: &[u8] = &target_owned;
                // Targets exceeding the per-response cap are rejected outright rather
                // than truncated so callers get a deterministic, clear error.
                if target_bytes.len() > ::sysapi::limits::PATH_MAX {
                    log::warn!(
                        "hostfsd: readlink target exceeds PATH_MAX ({} > {})",
                        target_bytes.len(),
                        ::sysapi::limits::PATH_MAX
                    );
                    let err = ReadlinkResponse {
                        status: HOSTFS_ERR_INVALID,
                        target_len: 0,
                        target: [0u8; MAX_INLINE_READLINK_TARGET],
                    };
                    err.encode(response);
                    return;
                }
                if target_bytes.len() <= MAX_INLINE_READLINK_TARGET {
                    // Inline fast path: target fits in a single response message.
                    let mut target_buf: [u8; MAX_INLINE_READLINK_TARGET] =
                        [0u8; MAX_INLINE_READLINK_TARGET];
                    target_buf[..target_bytes.len()].copy_from_slice(target_bytes);
                    let resp = ReadlinkResponse {
                        status: 0,
                        target_len: target_bytes.len() as u16,
                        target: target_buf,
                    };
                    resp.encode(response);
                    return;
                }
                // Multi-part response: emit a stream of `HostFsReadlinkResponsePart`
                // messages. The first part is written into `response` (and returned
                // by `handle_request`); the rest are queued in `extra_responses` for
                // the caller to drain.
                self.emit_long_readlink_response(response, op_id, 0, target_bytes);
            },
            Err(e) => {
                let err = ReadlinkResponse {
                    status: io_error_to_code(&e),
                    target_len: 0,
                    target: [0u8; MAX_INLINE_READLINK_TARGET],
                };
                err.encode(response);
            },
        }
    }

    /// Builds a multi-part `HostFsReadlinkResponsePart` stream for a successful
    /// `readlink` whose target exceeds the inline capacity.
    ///
    /// Wire-format details (body layout, per-part framing, chunk size) live in
    /// [`long_msg::serialize_long_readlink_response`] and
    /// [`long_msg::chunk_long_response`]. This method only encodes the policy that
    /// is local to the daemon: the first chunk is written into `first_response`,
    /// and the remaining chunks are queued in `extra_responses` for the caller to
    /// drain via [`Self::take_next_response_part`].
    ///
    /// The `op_id` is recorded in each outer frame's request-ID field and in the first four bytes
    /// of the assembled body, where vfsd's hostfs response assembler reads it.
    fn emit_long_readlink_response(
        &mut self,
        first_response: &mut [u8; Message::PAYLOAD_SIZE],
        op_id: OperationId,
        status: i32,
        target_bytes: &[u8],
    ) {
        // `target_bytes.len()` is bounded above by `PATH_MAX` at the call site, which
        // is well below `MAX_PATH_LEN` (u16::MAX), so the serializer cannot return
        // `None` here. Use a debug-only assert and fall through with an empty body
        // in release builds rather than panic on a malformed input.
        let body: Vec<u8> = long_msg::serialize_long_readlink_response(op_id, status, target_bytes)
            .unwrap_or_default();
        debug_assert!(
            !body.is_empty(),
            "serialize_long_readlink_response failed: target len {} exceeds wire limit",
            target_bytes.len()
        );

        // `target_bytes.len()` is bounded above by `PATH_MAX` at the call site,
        // which keeps the chunked stream well below `MAX_LONG_RESPONSE_PARTS`. The
        // chunker can therefore not return `None` in practice, but guard with a
        // debug assert and drop the response in release builds rather than emit a
        // wrapped/malformed stream.
        let parts_vec: Vec<[u8; Message::PAYLOAD_SIZE]> = match long_msg::chunk_long_response(
            SystemCallMessageHeader::HostFsReadlinkResponsePart as u16,
            op_id,
            &body,
        ) {
            Some(v) => v,
            None => {
                debug_assert!(
                    false,
                    "chunk_long_response rejected body of {} bytes (exceeds wire-format limit)",
                    body.len()
                );
                log::error!(
                    "hostfsd: dropping readlink response body ({} bytes) that exceeds the \
                     long-response part limit",
                    body.len()
                );
                return;
            },
        };
        let mut parts: std::vec::IntoIter<[u8; Message::PAYLOAD_SIZE]> = parts_vec.into_iter();
        if let Some(first) = parts.next() {
            *first_response = first;
        }
        self.extra_responses.extend(parts);
    }

    /// Shared `lstat` implementation used by both the inline and multi-part
    /// request handlers.
    fn do_lstat(&mut self, path_str: &str, response: &mut [u8; Message::PAYLOAD_SIZE]) {
        set_header(response, SystemCallMessageHeader::HostFsLstatResponse as u16);

        let host_path: PathBuf = match self.sandbox.resolve_nofollow(path_str) {
            Some(p) => p,
            None => {
                let err = LstatResponse {
                    status: HOSTFS_ERR_PERMISSION,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };

        match fs::symlink_metadata(&host_path) {
            Ok(meta) => {
                let kind: u8 = metadata_kind(&meta);
                let mode: u32 = metadata_mode(&meta, kind);
                let resp = LstatResponse {
                    status: 0,
                    size: meta.len(),
                    mode,
                    kind,
                };
                resp.encode(response);
            },
            Err(e) => {
                let err = LstatResponse {
                    status: io_error_to_code(&e),
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
            },
        }
    }

    /// Shared path-based *following* stat implementation used by both the inline and
    /// multi-part request handlers.
    ///
    /// This is the following counterpart to [`Self::do_lstat`]: it resolves the path
    /// with [`Sandbox::resolve`] (which follows symbolic links within the sandbox) and
    /// queries [`fs::metadata`] (which follows the final component). It is the host-side
    /// implementation of a following `fstatat`/`stat(2)` over hostfs.
    ///
    /// The response reuses [`LstatResponse`] (identical wire shape: status, size, mode,
    /// kind) but is tagged with the [`SystemCallMessageHeader::HostFsPathStatResponse`]
    /// header. Because the final component is followed, [`metadata_kind`] never reports
    /// [`file_kind::SYMLINK`]: a dangling final link surfaces as the host `ENOENT`, and a
    /// resolved link reports the target's kind.
    fn do_pathstat(&mut self, path_str: &str, response: &mut [u8; Message::PAYLOAD_SIZE]) {
        set_header(response, SystemCallMessageHeader::HostFsPathStatResponse as u16);

        let host_path: PathBuf = match self.sandbox.resolve(path_str) {
            Some(p) => p,
            None => {
                let err = LstatResponse {
                    status: HOSTFS_ERR_PERMISSION,
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
                return;
            },
        };

        match fs::metadata(&host_path) {
            Ok(meta) => {
                let kind: u8 = metadata_kind(&meta);
                let mode: u32 = metadata_mode(&meta, kind);
                let resp = LstatResponse {
                    status: 0,
                    size: meta.len(),
                    mode,
                    kind,
                };
                resp.encode(response);
            },
            Err(e) => {
                let err = LstatResponse {
                    status: io_error_to_code(&e),
                    size: 0,
                    mode: 0,
                    kind: file_kind::OTHER,
                };
                err.encode(response);
            },
        }
    }

    fn handle_open(
        &mut self,
        payload: &[u8; Message::PAYLOAD_SIZE],
        response: &mut [u8; Message::PAYLOAD_SIZE],
    ) {
        let req: OpenRequest = OpenRequest::decode(payload);
        let path_len: usize = (req.path_len as usize).min(req.path.len());
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
        let o_excl: bool = (flags & O_EXCL) != 0;
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

        let (file, created): (File, bool) = if is_dir {
            // For directories, open as read-only. Readdir uses the stored path.
            match open_dir_handle(&host_path) {
                Ok(file) => (file, false),
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
            match open_regular_file(&opts, &host_path, o_creat, o_excl) {
                Ok(result) => result,
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

        if created {
            #[cfg(unix)]
            {
                let permissions: std::fs::Permissions = std::fs::Permissions::from_mode(req.mode);
                if let Err(error) = file.set_permissions(permissions) {
                    drop(file);
                    let _ = fs::remove_file(&host_path);
                    set_header(response, SystemCallMessageHeader::HostFsOpenResponse as u16);
                    set_payload_data(response, &io_error_to_code(&error).to_le_bytes());
                    return;
                }
            }
        }

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
                let resp: WriteResponse = WriteResponse {
                    bytes_written: -1,
                    offset: -1,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
                return;
            },
        };

        // Reject writes on directory file descriptors.
        if entry.is_dir {
            let resp: WriteResponse = WriteResponse {
                bytes_written: -1,
                offset: -1,
            };
            resp.encode(response);
            set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
            return;
        }

        // For positional writes (offset >= 0), save and restore the current position
        // to provide pwrite() semantics: the file position is not affected.
        let saved_pos: Option<u64> = if req.offset >= 0 {
            let pos: Option<u64> = entry.file.stream_position().ok();
            if entry.file.seek(SeekFrom::Start(req.offset as u64)).is_err() {
                let resp: WriteResponse = WriteResponse {
                    bytes_written: -1,
                    offset: -1,
                };
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
        let resulting_offset: i64 = entry
            .file
            .stream_position()
            .ok()
            .and_then(|offset| i64::try_from(offset).ok())
            .unwrap_or(-1);

        // Restore position after positional write.
        if let Some(pos) = saved_pos {
            let _ = entry.file.seek(SeekFrom::Start(pos));
        }

        match result {
            Ok(n) => {
                let resp: WriteResponse = WriteResponse {
                    bytes_written: n as i32,
                    offset: resulting_offset,
                };
                resp.encode(response);
                set_header(response, SystemCallMessageHeader::HostFsWriteResponse as u16);
            },
            Err(e) => {
                log::debug!("hostfsd: write failed (fd={}, kind={:?}): {}", req.fd, e.kind(), e);
                let resp: WriteResponse = WriteResponse {
                    bytes_written: -1,
                    offset: -1,
                };
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

        // Copy the entry out of the cache so the mutable borrow of `self.fd_table` is
        // released before we may need `&mut self` to emit a multi-part response.
        let cached: Option<(String, bool, u64)> = match self.fd_table.get_mut(req.fd) {
            Some(entry) => entry
                .readdir_at(req.offset as usize)
                .map(|c| (c.name.to_string_lossy().into_owned(), c.is_dir, c.size)),
            None => {
                log::warn!("hostfsd: readdir on unknown fd {} (offset={})", req.fd, req.offset);
                ReadDirEntry {
                    status: HOSTFS_ERR_IO,
                    name_len: 0,
                    is_dir: 0,
                    size: 0,
                    name: [0u8; MAX_DIR_ENTRY_NAME_LEN],
                }
                .encode(response);
                set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
                return;
            },
        };

        let (name, is_dir, size) = match cached {
            Some(t) => t,
            None => {
                // Past end of directory: name_len=0 with a successful status signals end.
                set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
                ReadDirEntry {
                    status: 0,
                    name_len: 0,
                    is_dir: 0,
                    size: 0,
                    name: [0u8; MAX_DIR_ENTRY_NAME_LEN],
                }
                .encode(response);
                return;
            },
        };

        let name_bytes: &[u8] = name.as_bytes();
        if name_bytes.len() <= MAX_DIR_ENTRY_NAME_LEN {
            // Inline fast path: the name fits in a single response message.
            let mut entry_name: [u8; MAX_DIR_ENTRY_NAME_LEN] = [0u8; MAX_DIR_ENTRY_NAME_LEN];
            entry_name[..name_bytes.len()].copy_from_slice(name_bytes);

            let resp: ReadDirEntry = ReadDirEntry {
                status: 0,
                name_len: name_bytes.len() as u16,
                is_dir: if is_dir { 1 } else { 0 },
                size,
                name: entry_name,
            };
            resp.encode(response);
            set_header(response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
        } else {
            // Multi-part response: the name exceeds the inline capacity, so emit a
            // `HostFsReadDirResponsePart` stream carrying the full name.
            let op_id: OperationId = get_op_id(payload);
            self.emit_long_readdir_response(response, op_id, is_dir, size, name_bytes);
        }
    }

    /// Builds a multi-part `HostFsReadDirResponsePart` stream for a directory entry
    /// whose name exceeds the inline `ReadDirEntry` capacity.
    ///
    /// Wire-format details (body layout, per-part framing, chunk size) live in
    /// [`long_msg::serialize_long_readdir_response`] and
    /// [`long_msg::chunk_long_response`]. The first chunk is written into
    /// `first_response`, and the remaining chunks are queued in `extra_responses` for
    /// the caller to drain via [`Self::take_next_response_part`].
    ///
    /// As with the readlink multi-part response, the `op_id` is carried by both each outer frame's
    /// request-ID field and the first four bytes of the assembled response body.
    fn emit_long_readdir_response(
        &mut self,
        first_response: &mut [u8; Message::PAYLOAD_SIZE],
        op_id: OperationId,
        is_dir: bool,
        size: u64,
        name_bytes: &[u8],
    ) {
        let body: Vec<u8> =
            match long_msg::serialize_long_readdir_response(op_id, is_dir, size, name_bytes) {
                Some(body) => body,
                None => {
                    // Unreachable in practice: host filenames are bounded far below the u16
                    // name-length limit. Guard against emitting a zeroed (and therefore
                    // malformed) frame by falling back to a single-message end-of-directory
                    // marker, so the getdents sweep terminates cleanly instead of leaving the
                    // caller hung on a frame it cannot parse. The op_id is stamped by the
                    // caller (`run` in `handle_request`) because this is not a multi-part
                    // response header.
                    debug_assert!(
                        false,
                        "serialize_long_readdir_response failed: name len {} exceeds wire limit",
                        name_bytes.len()
                    );
                    log::error!(
                        "hostfsd: readdir entry name ({} bytes) exceeds the wire-format limit; \
                         ending directory listing early",
                        name_bytes.len()
                    );
                    set_header(
                        first_response,
                        SystemCallMessageHeader::HostFsReadDirResponse as u16,
                    );
                    set_payload_data(first_response, &[0u8; 2]);
                    return;
                },
            };

        let parts_vec: Vec<[u8; Message::PAYLOAD_SIZE]> = match long_msg::chunk_long_response(
            SystemCallMessageHeader::HostFsReadDirResponsePart as u16,
            op_id,
            &body,
        ) {
            Some(v) => v,
            None => {
                debug_assert!(
                    false,
                    "chunk_long_response rejected body of {} bytes (exceeds wire-format limit)",
                    body.len()
                );
                log::error!(
                    "hostfsd: dropping readdir response body ({} bytes) that exceeds the \
                     long-response part limit",
                    body.len()
                );
                // Fall back to a well-formed single-message end-of-directory marker so the
                // caller's getdents sweep terminates cleanly.
                set_header(first_response, SystemCallMessageHeader::HostFsReadDirResponse as u16);
                set_payload_data(first_response, &[0u8; 2]);
                return;
            },
        };
        let mut parts: std::vec::IntoIter<[u8; Message::PAYLOAD_SIZE]> = parts_vec.into_iter();
        if let Some(first) = parts.next() {
            *first_response = first;
        }
        self.extra_responses.extend(parts);
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

        let host_path: PathBuf = match self.sandbox.resolve_nofollow(path_str) {
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
///
/// `ELOOP` (too many levels of symbolic links) is detected via the raw OS error code
/// because `std::io::ErrorKind::FilesystemLoop` is unstable. The Linux/macOS code is
/// `40`/`62`; on Windows the closest equivalent reported by `CreateFileW` when a
/// reparse-point chain cannot be resolved is `ERROR_CANT_RESOLVE_FILENAME` (1921).
fn io_error_to_code(e: &io::Error) -> i32 {
    // ELOOP-style errors do not have a stable `ErrorKind`, so probe the raw OS code.
    if let Some(raw) = e.raw_os_error() {
        #[cfg(target_os = "linux")]
        const ELOOP_RAW: i32 = 40;
        #[cfg(target_os = "macos")]
        const ELOOP_RAW: i32 = 62;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        const ELOOP_RAW: i32 = i32::MIN; // sentinel: no match on this platform
        #[cfg(windows)]
        const ERROR_CANT_RESOLVE_FILENAME: i32 = 1921;
        if raw == ELOOP_RAW {
            return HOSTFS_ERR_LOOP;
        }
        #[cfg(windows)]
        if raw == ERROR_CANT_RESOLVE_FILENAME {
            return HOSTFS_ERR_LOOP;
        }
    }
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

/// Creates a symbolic link at `linkpath` pointing to the verbatim `target` string.
///
/// On Unix, [`std::os::unix::fs::symlink`] is used directly: the resulting link is
/// type-agnostic, matching POSIX `symlink(2)` semantics.
///
/// On Windows, symbolic links are typed at creation time (file vs. directory). The
/// best-effort policy is:
/// 1. If the target resolves (relative to the link's parent) to an existing directory,
///    use [`std::os::windows::fs::symlink_dir`].
/// 2. Otherwise — including the common case of a target that does not yet exist or is a
///    file — use [`std::os::windows::fs::symlink_file`].
///
/// On Windows, creating a symbolic link requires either administrative privileges or
/// Developer Mode (which grants `SeCreateSymbolicLinkPrivilege` to standard users).
/// Without those, the call fails with `ERROR_PRIVILEGE_NOT_HELD`, which is mapped to
/// [`HOSTFS_ERR_NOT_SUPPORTED`] by [`symlink_error_to_code`].
///
/// Note: the Windows file-vs-directory probe uses `fs::metadata`, which *follows*
/// symbolic links. If `target` itself resolves through another link to a directory,
/// the directory variant is selected; if it resolves to a file or does not exist,
/// the file variant is used. This is best-effort classification at creation time and
/// does not affect the verbatim storage of the link target.
fn create_symlink(target: &str, linkpath: &std::path::Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, linkpath)
    }
    #[cfg(windows)]
    {
        // Resolve `target` relative to the link's parent to decide which variant of the
        // Windows symlink API to use. If the target's type cannot be determined (e.g.,
        // it does not exist yet), default to the file variant — this matches the most
        // common case and lets the link become valid once the target is created.
        let parent: &std::path::Path = linkpath
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let absolute_target: std::path::PathBuf = parent.join(target);
        let is_dir: bool = fs::metadata(&absolute_target)
            .map(|m| m.is_dir())
            .unwrap_or(false);
        if is_dir {
            std::os::windows::fs::symlink_dir(target, linkpath)
        } else {
            std::os::windows::fs::symlink_file(target, linkpath)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, linkpath);
        Err(io::Error::new(io::ErrorKind::Unsupported, "symlink not supported on this platform"))
    }
}

/// Translates an `io::Error` from a symlink creation attempt into a hostfs error code.
///
/// Has the same behaviour as [`io_error_to_code`] for the cases it can handle, plus a
/// Windows-specific mapping of `ERROR_PRIVILEGE_NOT_HELD` (raw OS code 1314) to
/// [`HOSTFS_ERR_NOT_SUPPORTED`]. That error appears when the daemon is run without
/// Developer Mode and without the symlink privilege, and surfacing it as a distinct
/// code lets the guest report a clearer diagnostic.
fn symlink_error_to_code(e: &io::Error) -> i32 {
    #[cfg(windows)]
    {
        // ERROR_PRIVILEGE_NOT_HELD: returned by CreateSymbolicLinkW when the caller
        // lacks SeCreateSymbolicLinkPrivilege (e.g., no Developer Mode).
        const ERROR_PRIVILEGE_NOT_HELD: i32 = 1314;
        if e.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD) {
            return HOSTFS_ERR_NOT_SUPPORTED;
        }
    }
    io_error_to_code(e)
}

/// Derives a hostfs [`file_kind`] discriminant from host [`fs::Metadata`].
///
/// Symbolic links take precedence over the regular/dir classification because the
/// metadata is expected to come from [`fs::symlink_metadata`] (an `lstat`), and the
/// caller wants to know that the path resolves to a link rather than what the link
/// points to.
fn metadata_kind(meta: &fs::Metadata) -> u8 {
    let ft = meta.file_type();
    if ft.is_symlink() {
        file_kind::SYMLINK
    } else if ft.is_dir() {
        file_kind::DIRECTORY
    } else if ft.is_file() {
        file_kind::REGULAR
    } else {
        file_kind::OTHER
    }
}

/// Reports a `st_mode`-style value for an `lstat` response.
///
/// On Unix, the host kernel already supplies a `st_mode` that includes both the
/// permission bits and the type bits. On Windows, the Rust standard library exposes a
/// permission *boolean* (read-only) and no file-type bits, so a synthetic value is
/// built from POSIX-style constants. The guest must treat the value as informational
/// — the authoritative file kind is the separate [`LstatResponse::kind`] field.
fn metadata_mode(meta: &fs::Metadata, kind: u8) -> u32 {
    #[cfg(unix)]
    {
        let _ = kind;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        // POSIX file-type constants. Defined locally to avoid pulling sysapi into this
        // host-side crate just for two integer literals.
        const S_IFREG: u32 = 0o100_000;
        const S_IFDIR: u32 = 0o040_000;
        const S_IFLNK: u32 = 0o120_000;
        const PERM_RW: u32 = 0o666;
        const PERM_RO: u32 = 0o444;
        const PERM_DIR: u32 = 0o755;
        let type_bits: u32 = match kind {
            file_kind::SYMLINK => S_IFLNK,
            file_kind::DIRECTORY => S_IFDIR,
            file_kind::REGULAR => S_IFREG,
            _ => 0,
        };
        let perm_bits: u32 = if kind == file_kind::DIRECTORY {
            PERM_DIR
        } else if meta.permissions().readonly() {
            PERM_RO
        } else {
            PERM_RW
        };
        type_bits | perm_bits
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
    /// A new request started before the current request was complete.
    Interrupted,
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
    /// Request identifier recorded from the first part.
    request_id: Option<OperationId>,
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
            request_id: None,
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

    /// Returns the request identifier recorded from the first part.
    fn recorded_request_id(&self) -> OperationId {
        self.request_id.unwrap_or(OperationId::INVALID)
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
        request_id: OperationId,
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
        // previous op_id on vfsd's pending-op table. Instead, surface an interruption so
        // the caller can recover the in-flight op_id from the buffered bytes, emit a
        // well-formed error response, and replay the new part after resetting the assembler.
        if self.header.is_some() && number == 0 && self.parts_received < self.total_parts {
            log::error!(
                "hostfsd: assembler received new part_number=0 ({:?}) while {}/{} parts of {:?} \
                 were still in flight; failing in-flight request",
                header,
                self.parts_received,
                self.total_parts,
                self.header,
            );
            return AssemblyStatus::Interrupted;
        }

        // Check if this is the first part of a new request.
        if self.header.is_none() {
            self.header = Some(header);
            self.request_id = Some(request_id);
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

        if self.request_id != Some(request_id) {
            log::error!(
                "hostfsd: assembler request identifier mismatch (expected {:?}, got {:?})",
                self.request_id,
                request_id
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
        self.request_id = None;
        self.total_parts = 0;
        self.parts_received = 0;
        self.next_part_number = 0;
        self.buffer.clear();
    }
}

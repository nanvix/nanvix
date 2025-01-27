// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod fcntl;
mod message;
mod socket;
mod time;
mod times;
mod unistd;
mod venv;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

extern crate alloc;

use self::{
    args::Args,
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
        RequestAssemblerType,
    },
    venv::VirtualEnviromentDirectory,
};
use ::anyhow::Result;
use ::flexi_logger::{
    FileSpec,
    Logger,
};
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::posix::{
    fcntl::message::{
        FileAdvisoryInformationRequest,
        FileChmodAtRequest,
        FileChownAtRequest,
        FileControlRequest,
        FileSpaceControlRequest,
        MakeDirectoryAtRequest,
        OpenAtRequest,
        ReadLinkAtRequest,
        RenameAtRequest,
        SymbolicLinkAtRequest,
        UnlinkAtRequest,
    },
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
    },
    sys::{
        socket::message::{
            AcceptSocketRequest,
            BindSocketRequest,
            ConnectSocketRequest,
            CreateSocketPairRequest,
            CreateSocketRequest,
            GetPeerNameRequest,
            GetSockNameRequest,
            ListenSocketRequest,
            ReceiveSocketRequest,
            SendSocketRequest,
            ShutdownSocketRequest,
        },
        stat::message::{
            FileStatAtRequest,
            FileStatRequest,
            UpdateFileAccessTimeAtRequest,
            UpdateFileAccessTimeRequest,
        },
        times::message::TimesRequest,
        types::ssize_t,
    },
    time::message::{
        ClockResolutionRequest,
        GetClockTimeRequest,
    },
    unistd::message::{
        CloseRequest,
        CloseResponse,
        FileChmodRequest,
        FileChownRequest,
        FileDataSyncRequest,
        FileSyncRequest,
        FileTruncateRequest,
        LinkAtRequest,
        PartialReadRequest,
        PartialWriteRequest,
        ReadRequest,
        ReadResponse,
        SeekRequest,
        WriteRequest,
        WriteResponse,
    },
    venv::message::{
        JoinEnvRequest,
        LeaveEnvRequest,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::signal_hook::{
    consts::SIGINT,
    iterator::{
        Signals,
        SignalsInfo,
    },
};
use ::std::{
    env,
    fs,
    io::{
        ErrorKind,
        Read,
        Write,
    },
    os::unix::net::{
        UnixListener,
        UnixStream,
    },
    sync::Once,
    thread,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon<'a> {
    pid: ProcessIdentifier,
    assembler: RequestAssembler,
    stream: UnixStream,
    gateway_conn: &'a mut Option<UnixStream>,
    venv: VirtualEnviromentDirectory,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> LinuxDaemon<'a> {
    pub fn init(
        stream: UnixStream,
        gateway_conn: &'a mut Option<UnixStream>,
    ) -> Result<Self, Error> {
        Ok(Self {
            pid: ProcessIdentifier::from(0),
            assembler: RequestAssembler::default(),
            stream,
            gateway_conn,
            venv: VirtualEnviromentDirectory::new(),
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            let message: Message = match self.recv() {
                Ok(Some(message)) => message,
                Ok(None) => {
                    info!("connection closed");
                    break Ok(());
                },

                Err(e) => {
                    error!("failed to receive message (error={:?})", e);
                    continue;
                },
            };

            trace!(
                "message.source={:?}, message.destination={:?}, message.type={:?}",
                { message.source },
                { message.destination },
                message.message_type,
            );

            let source = message.source;

            match message.message_type {
                nvx::ipc::MessageType::Empty => panic!("received empty message"),
                nvx::ipc::MessageType::Interrupt => panic!("received interrupt message"),
                nvx::ipc::MessageType::Exception => panic!("received exception message"),
                nvx::ipc::MessageType::Ipc => panic!("received IPC message"),
                nvx::ipc::MessageType::SchedulingEvent => {
                    panic!("received scheduling event message")
                },
                nvx::ipc::MessageType::Ikc => {
                    match LinuxDaemonMessage::try_from_bytes(message.payload) {
                        Ok(message) => {
                            let message: Message = match message.header {
                                LinuxDaemonMessageHeader::JoinEnvRequest => {
                                    let request: JoinEnvRequest =
                                        JoinEnvRequest::from_bytes(message.payload);
                                    self.venv.join(source, request)
                                },
                                LinuxDaemonMessageHeader::LeaveEnvRequest => {
                                    let request: LeaveEnvRequest =
                                        LeaveEnvRequest::from_bytes(message.payload);
                                    self.venv.leave(source, request)
                                },
                                LinuxDaemonMessageHeader::GetClockResolutionRequest => {
                                    let request: ClockResolutionRequest =
                                        ClockResolutionRequest::from_bytes(message.payload);
                                    time::do_clock_getres(source, request)
                                },
                                LinuxDaemonMessageHeader::GetClockTimeRequest => {
                                    let request: GetClockTimeRequest =
                                        GetClockTimeRequest::from_bytes(message.payload);
                                    time::do_clock_gettime(source, request)
                                },
                                LinuxDaemonMessageHeader::OpenAtRequest => {
                                    let request: OpenAtRequest =
                                        OpenAtRequest::from_bytes(message.payload);
                                    fcntl::do_open_at(source, request)
                                },
                                LinuxDaemonMessageHeader::UnlinkAtRequest => {
                                    let request: UnlinkAtRequest =
                                        UnlinkAtRequest::from_bytes(message.payload);
                                    fcntl::do_unlink_at(source, request)
                                },
                                LinuxDaemonMessageHeader::CloseRequest => {
                                    let request: CloseRequest =
                                        CloseRequest::from_bytes(message.payload);

                                    // Inspect file descriptor that is being closed, as we need to
                                    // handle standard file descriptors specially.
                                    match request.fd {
                                        // Closing standard file descriptors.
                                        ::posix::unistd::STDIN_FILENO
                                        | ::posix::unistd::STDOUT_FILENO
                                        | ::posix::unistd::STDERR_FILENO => {
                                            // Perform a fake close, as standard file descriptors
                                            // are shared with the current process.
                                            CloseResponse::build(source, 0)
                                        },
                                        // Closing other file descriptors.
                                        _ => unistd::do_close(source, request),
                                    }
                                },
                                LinuxDaemonMessageHeader::RenameAtRequest => {
                                    let request: RenameAtRequest =
                                        RenameAtRequest::from_bytes(message.payload);
                                    fcntl::do_rename_at(source, request)
                                },
                                LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                                    self.handle_fstatat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::FileDataSyncRequest => {
                                    let request: FileDataSyncRequest =
                                        FileDataSyncRequest::from_bytes(message.payload);
                                    unistd::do_fdatasync(source, request)
                                },
                                LinuxDaemonMessageHeader::FileSyncRequest => {
                                    let request: FileSyncRequest =
                                        FileSyncRequest::from_bytes(message.payload);
                                    unistd::do_fsync(source, request)
                                },
                                LinuxDaemonMessageHeader::SeekRequest => {
                                    let request: SeekRequest =
                                        SeekRequest::from_bytes(message.payload);
                                    unistd::do_lseek(source, request)
                                },
                                LinuxDaemonMessageHeader::FileSpaceControlRequest => {
                                    let request: FileSpaceControlRequest =
                                        FileSpaceControlRequest::from_bytes(message.payload);
                                    fcntl::do_posix_fallocate(source, request)
                                },
                                LinuxDaemonMessageHeader::FileTruncateRequest => {
                                    let request: FileTruncateRequest =
                                        FileTruncateRequest::from_bytes(message.payload);
                                    unistd::do_ftruncate(source, request)
                                },
                                LinuxDaemonMessageHeader::FileAdvisoryInformationRequest => {
                                    let request: FileAdvisoryInformationRequest =
                                        FileAdvisoryInformationRequest::from_bytes(message.payload);
                                    fcntl::do_posix_fadvise(source, request)
                                },
                                LinuxDaemonMessageHeader::FileStatRequest => {
                                    self.handle_fstat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::WriteRequest => {
                                    let request: WriteRequest =
                                        WriteRequest::from_bytes(message.payload);

                                    // Check if writing to gateway.
                                    if request.fd == ::posix::unistd::STDOUT_FILENO {
                                        if let Some(ref mut conn) = self.gateway_conn {
                                            let count: usize = request.count as usize;
                                            let mut buffer: Vec<u8> = vec![0u8; count + 1];
                                            // TODO: make count 32-bit long.
                                            buffer[0] = count as u8;
                                            buffer[1..].copy_from_slice(&request.buffer[..count]);
                                            match conn.write_all(&buffer) {
                                                Ok(_) => {
                                                    debug!("wrote {} bytes to the gateway", count);
                                                    WriteResponse::build(source, count as i32)
                                                },
                                                Err(e) => {
                                                    debug!(
                                                        "failed to write to the gateway \
                                                         (error={:?})",
                                                        e
                                                    );
                                                    // TODO: Check error conversion.
                                                    build_error(source, ErrorCode::ConnectionReset)
                                                },
                                            }
                                        } else {
                                            // Not connected to the gateway, print to stdout.
                                            let count: usize = request.count as usize;
                                            let buffer: &[u8] = &request.buffer[..count];
                                            let string: String =
                                                String::from_utf8_lossy(buffer).to_string();
                                            print!("{}", string);
                                            WriteResponse::build(source, count as ssize_t)
                                        }
                                    } else {
                                        // Write to other file descriptor.
                                        unistd::do_write(source, request)
                                    }
                                },
                                LinuxDaemonMessageHeader::ReadRequest => {
                                    let request: ReadRequest =
                                        ReadRequest::from_bytes(message.payload);

                                    // Check if reading from gateway.
                                    if request.fd == ::posix::unistd::STDIN_FILENO {
                                        if let Some(ref mut conn) = self.gateway_conn {
                                            // TODO: make count 32-bit long.
                                            let mut len_buf: [u8; 1] = [0u8; 1];
                                            match conn.read_exact(&mut len_buf) {
                                                Ok(_) => {
                                                    if len_buf[0] == 0 {
                                                        debug!("read 0 bytes from the gateway");
                                                        ReadResponse::build(
                                                            source,
                                                            0,
                                                            [0u8; ReadResponse::BUFFER_SIZE],
                                                        )
                                                    } else {
                                                        let count: usize = len_buf[0] as usize;
                                                        let mut buf: Vec<u8> = vec![0u8; count];
                                                        match conn.read_exact(&mut buf) {
                                                            Ok(_) => {
                                                                debug!(
                                                                    "read {} bytes from the \
                                                                     gateway",
                                                                    count
                                                                );
                                                                let mut response_buf = [0u8;
                                                                    ReadResponse::BUFFER_SIZE];
                                                                response_buf[..count]
                                                                    .copy_from_slice(&buf);
                                                                ReadResponse::build(
                                                                    source,
                                                                    count as i32,
                                                                    response_buf,
                                                                )
                                                            },
                                                            Err(e) => {
                                                                debug!(
                                                                    "failed to read from the \
                                                                     gateway (error={:?})",
                                                                    e
                                                                );
                                                                // TODO: Check error conversion.
                                                                build_error(
                                                                    source,
                                                                    ErrorCode::ConnectionReset,
                                                                )
                                                            },
                                                        }
                                                    }
                                                },
                                                Err(e) => {
                                                    debug!(
                                                        "failed to read length from the gateway \
                                                         (error={:?})",
                                                        e
                                                    );
                                                    // TODO: Check error conversion.
                                                    build_error(source, ErrorCode::ConnectionReset)
                                                },
                                            }
                                        } else {
                                            // Not connected to the gateway, read from stdin.
                                            let mut buffer: [u8; ReadResponse::BUFFER_SIZE] =
                                                [0u8; ReadResponse::BUFFER_SIZE];
                                            let count: usize = match ::std::io::stdin()
                                                .read(&mut buffer)
                                            {
                                                Ok(count) => count,
                                                Err(e) => {
                                                    debug!(
                                                        "failed to read from stdin (error={:?})",
                                                        e
                                                    );
                                                    0
                                                },
                                            };
                                            ReadResponse::build(source, count as ssize_t, buffer)
                                        }
                                    } else {
                                        // Read from other file descriptor.
                                        unistd::do_read(source, request)
                                    }
                                },
                                LinuxDaemonMessageHeader::PartialWriteRequest => {
                                    let request: PartialWriteRequest =
                                        PartialWriteRequest::from_bytes(message.payload);
                                    unistd::do_pwrite(source, request)
                                },
                                LinuxDaemonMessageHeader::PartialReadRequest => {
                                    let request: PartialReadRequest =
                                        PartialReadRequest::from_bytes(message.payload);
                                    unistd::do_pread(source, request)
                                },
                                LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                                    self.handle_symlinkat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::LinkAtRequestPart => {
                                    debug!("received linkat request");
                                    self.handle_linkat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                                    self.handle_readlinkat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                                    self.handle_mkdirat_request(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                                    self.handle_utimensat(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                                    let request: UpdateFileAccessTimeRequest =
                                        UpdateFileAccessTimeRequest::from_bytes(message.payload);
                                    fcntl::do_futimens(source, request)
                                },
                                LinuxDaemonMessageHeader::FileControlRequest => {
                                    let request: FileControlRequest =
                                        FileControlRequest::from_bytes(message.payload);
                                    fcntl::do_fcntl(source, request)
                                },
                                LinuxDaemonMessageHeader::CreateSocketRequest => {
                                    let request: CreateSocketRequest =
                                        CreateSocketRequest::from_bytes(message.payload);
                                    socket::do_socket(source, request)
                                },
                                LinuxDaemonMessageHeader::BindSocketRequest => {
                                    let request: BindSocketRequest =
                                        BindSocketRequest::from_bytes(message.payload);
                                    socket::do_bind(source, request)
                                },
                                LinuxDaemonMessageHeader::ListenSocketRequest => {
                                    let request: ListenSocketRequest =
                                        ListenSocketRequest::from_bytes(message.payload);
                                    socket::do_listen(source, request)
                                },
                                LinuxDaemonMessageHeader::AcceptSocketRequest => {
                                    let request: AcceptSocketRequest =
                                        AcceptSocketRequest::from_bytes(message.payload);
                                    socket::do_accept(source, request)
                                },
                                LinuxDaemonMessageHeader::ShutdownSocketRequest => {
                                    let request: ShutdownSocketRequest =
                                        ShutdownSocketRequest::from_bytes(message.payload);
                                    socket::do_shutdown(source, request)
                                },
                                LinuxDaemonMessageHeader::ReceiveSocketRequest => {
                                    let request: ReceiveSocketRequest =
                                        ReceiveSocketRequest::from_bytes(message.payload);
                                    socket::do_recv(source, request)
                                },
                                LinuxDaemonMessageHeader::SendSocketRequest => {
                                    let request: SendSocketRequest =
                                        SendSocketRequest::from_bytes(message.payload);
                                    socket::do_send(source, request)
                                },
                                LinuxDaemonMessageHeader::TimesRequest => {
                                    let request: TimesRequest =
                                        TimesRequest::from_bytes(message.payload);
                                    times::do_times(source, request)
                                },
                                LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                                    self.handle_chownat(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::FileChownRequest => {
                                    let request: FileChownRequest =
                                        FileChownRequest::from_bytes(message.payload);
                                    unistd::do_fchown(source, request)
                                },
                                LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                                    self.handle_chmodat(source, message);
                                    continue;
                                },
                                LinuxDaemonMessageHeader::FileChmodRequest => {
                                    let request: FileChmodRequest =
                                        FileChmodRequest::from_bytes(message.payload);
                                    unistd::do_fchmod(source, request)
                                },
                                LinuxDaemonMessageHeader::ConnectSocketRequest => {
                                    let request: ConnectSocketRequest =
                                        ConnectSocketRequest::from_bytes(message.payload);
                                    socket::do_connect(source, request)
                                },
                                LinuxDaemonMessageHeader::CreateSocketPairRequest => {
                                    let request: CreateSocketPairRequest =
                                        CreateSocketPairRequest::from_bytes(message.payload);
                                    socket::do_socketpair(source, request)
                                },
                                LinuxDaemonMessageHeader::GetPeerNameRequest => {
                                    let request: GetPeerNameRequest =
                                        GetPeerNameRequest::from_bytes(message.payload);
                                    socket::do_getpeername(source, request)
                                },
                                LinuxDaemonMessageHeader::GetSockNameRequest => {
                                    let request: GetSockNameRequest =
                                        GetSockNameRequest::from_bytes(message.payload);
                                    socket::do_getsockname(source, request)
                                },

                                _ => self.do_error(source, ErrorCode::InvalidMessage),
                            };
                            self.send(message).unwrap();
                        },
                        Err(e) => {
                            error!("failed to parse Linux daemon message (error={:?})", e);
                            continue;
                        },
                    }
                },
            }
        }
    }

    // Read a message from the TCP stream.
    fn recv(&mut self) -> Result<Option<Message>> {
        let mut buf = [0u8; config::kernel::IPC_MESSAGE_SIZE];
        let mut buf_reader = std::io::BufReader::new(&self.stream);
        if let Err(e) = buf_reader.read_exact(&mut buf) {
            match e.kind() {
                ErrorKind::UnexpectedEof => return Ok(None),
                _ => {
                    let reason: String = format!("failed to read message (error={:?})", e);
                    unimplemented!("handle: {}", reason);
                },
            }
        };

        let message = match Message::try_from_bytes(buf) {
            Ok(message) => message,
            Err(e) => {
                let reason: String = format!("failed to parse message (error={:?})", e);
                unimplemented!("handle: {}", reason);
            },
        };

        Ok(Some(message))
    }

    // Send a message to the TCP stream.
    fn send(&mut self, message: Message) -> Result<()> {
        let bytes = message.to_bytes();
        match self.stream.write_all(&bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                let reason: String = format!("failed to write message (error={:?})", e);
                unimplemented!("handle: {}", reason);
            },
        }
    }

    fn do_error(&self, source: ProcessIdentifier, code: ErrorCode) -> Message {
        Message::new(self.pid, source, MessageType::Ikc, Some(code), [0u8; Message::PAYLOAD_SIZE])
    }

    fn handle_fstatat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<FileStatAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_fstat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let request: FileStatRequest = FileStatRequest::from_bytes(message.payload);

        let messages = fcntl::do_fstat(source, request);
        for message in messages {
            if let Err(e) = self.send(message) {
                error!("failed to send message (error={:?})", e);
            }
        }
    }

    fn handle_symlinkat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<SymbolicLinkAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }
    fn handle_linkat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<LinkAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process linkat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_readlinkat_request(
        &mut self,
        source: ProcessIdentifier,
        message: LinuxDaemonMessage,
    ) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<ReadLinkAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process readlinkat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_mkdirat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<MakeDirectoryAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process mkdirat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_utimensat(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<UpdateFileAccessTimeAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process mkdirat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_chownat(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<FileChownAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process fchownat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }

    fn handle_chmodat(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self
            .assembler
            .process_message::<FileChmodAtRequest>(source, part)
        {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process fchmodat request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }
}

pub fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    let sockaddr: String = args.bind_sockaddr();
    initialize(args.log_to_file());

    let listener: UnixListener = match UnixListener::bind(sockaddr.clone()) {
        Ok(listener) => listener,
        Err(e) => {
            error!("failed to bind to socket address (error={:?})", e);
            anyhow::bail!("failed to bind to socket address");
        },
    };

    // Install signal handler.
    let path: String = sockaddr.clone();
    let mut signals: SignalsInfo = Signals::new([SIGINT])?;
    thread::spawn(move || {
        #[allow(clippy::never_loop)]
        for sig in signals.forever() {
            println!("Received signal {:?}", sig);
            if let Err(e) = fs::remove_file(path.clone()) {
                error!("failed to remove socket file (error={:?})", e);
            }
            // Exit process.
            std::process::exit(0);
        }
    });

    // Connect to gateway after binding to socket address, as a connection to the gateway will
    // signal we are ready to accept commands.
    let mut gateway_conn: Option<UnixStream> = match args.gateway_sockaddr() {
        Some(sockaddr) => match UnixStream::connect(sockaddr) {
            Ok(stream) => Some(stream),
            Err(e) => {
                error!("failed to connect to gateway (error={:?})", e);
                anyhow::bail!("failed to connect to gateway");
            },
        },
        None => None,
    };

    loop {
        let stream: UnixStream = match listener.accept() {
            Ok((stream, sockaddr)) => {
                info!("Connected to: {:?}", sockaddr);
                stream
            },
            Err(e) => {
                anyhow::bail!("Failed to connect: {}", e);
            },
        };

        let mut procd: LinuxDaemon = match LinuxDaemon::init(stream, &mut gateway_conn) {
            Ok(procd) => procd,
            Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
        };

        if procd.run().is_err() {
            break;
        }
    }

    fs::remove_file(sockaddr)?;

    Ok(())
}

///
/// # Description
///
/// Initializes the logger.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn initialize(logfile: bool) {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        let logger = Logger::try_with_env().expect("malformed RUST_LOG environment variable");
        if logfile {
            logger
                .log_to_file(FileSpec::default())
                .start()
                .expect("failed to initialize logger");
        } else {
            logger.start().expect("failed to initialize logger");
        }
    });
}

///
/// # Description
///
/// Builds an error response message.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `error`: Error code.
///
/// # Returns
///
/// A message with the error response.
///
pub fn build_error(pid: ProcessIdentifier, error: ErrorCode) -> Message {
    Message::new(::posix::LINUXD, pid, MessageType::Ikc, Some(error), [0u8; Message::PAYLOAD_SIZE])
}

impl RequestAssemblerTrait for FileStatAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileStatAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fstat_at(source, request)
    }
}

impl RequestAssemblerTrait for SymbolicLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::SymbolicLinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_symlinkat(source, request)
    }
}

impl RequestAssemblerTrait for LinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        debug!("creating linkat request assembler");
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::LinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        debug!("adding part to linkat request");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        debug!("checking if linkat request is complete");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        debug!("taking parts from linkat request");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        unistd::do_linkat(source, request)
    }
}

impl RequestAssemblerTrait for ReadLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::ReadLinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_readlinkat(source, request)
    }
}

impl RequestAssemblerTrait for MakeDirectoryAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::MakeDirectoryAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_mkdirat(source, request)
    }
}

impl RequestAssemblerTrait for UpdateFileAccessTimeAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::UpdateFileAccessTimeAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                assembler.add_part(part)
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                Ok(assembler.is_complete())
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                assembler.take_parts()
            },
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_utimensat(source, request)
    }
}

impl RequestAssemblerTrait for FileChownAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChownAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fchownat(source, request)
    }
}

impl RequestAssemblerTrait for FileChmodAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChmodAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fchmodat(source, request)
    }
}

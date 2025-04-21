// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod assemble;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    build_error,
    dirent,
    fcntl,
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    socket,
    time,
    times,
    unistd,
    venv::{
        VirtualEnviromentDirectory,
        VirtualEnvironment,
    },
};
use ::anyhow::Result;
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
    dirent::message::GetDirectoryEntriesRequest,
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
    message::LinuxDaemonMessagePart,
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
        FileChdirRequest,
        FileChmodRequest,
        FileChownRequest,
        FileDataSyncRequest,
        FileSyncRequest,
        FileTruncateRequest,
        LinkAtRequest,
        PartialReadRequest,
        PartialWriteRequest,
        PipeRequest,
        ReadRequest,
        ReadResponse,
        SeekRequest,
        WriteRequest,
        WriteResponse,
    },
    venv::VirtualEnvironmentIdentifier,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::std::{
    io,
    io::{
        ErrorKind,
        Read,
        Write,
    },
    mem,
    os::unix::net::UnixStream,
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
                    break;
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

            let source: ProcessIdentifier = message.source;

            // Check if process is associated with a virtual environment.
            if self.venv.get(source).is_none() {
                // Join a new virtual environment.
                if let Err(error) = self.venv.join(source, VirtualEnvironmentIdentifier::NEW) {
                    warn!("failed to join new virtual environment (error={:?})", error);
                    let message: Message = crate::build_error(source, error.code);
                    self.send(message).unwrap();
                }
                // TODO: leave environment on process exit.
            }

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
                                // The system calls are interposed before being forwarded to the
                                // backend provider.
                                LinuxDaemonMessageHeader::CloseRequest
                                | LinuxDaemonMessageHeader::ReadRequest
                                | LinuxDaemonMessageHeader::WriteRequest => {
                                    self.handle_special_messages(source, message)
                                },

                                // The following system calls have their request and response
                                // data fit in a single message. Thus, they can be immediately
                                // forwarded to the backend provider.
                                LinuxDaemonMessageHeader::AcceptSocketRequest
                                | LinuxDaemonMessageHeader::BindSocketRequest
                                | LinuxDaemonMessageHeader::ConnectSocketRequest
                                | LinuxDaemonMessageHeader::CreateSocketPairRequest
                                | LinuxDaemonMessageHeader::CreateSocketRequest
                                | LinuxDaemonMessageHeader::FileAdvisoryInformationRequest
                                | LinuxDaemonMessageHeader::FileChdirRequest
                                | LinuxDaemonMessageHeader::FileChmodRequest
                                | LinuxDaemonMessageHeader::FileChownRequest
                                | LinuxDaemonMessageHeader::FileControlRequest
                                | LinuxDaemonMessageHeader::FileDataSyncRequest
                                | LinuxDaemonMessageHeader::FileSpaceControlRequest
                                | LinuxDaemonMessageHeader::FileSyncRequest
                                | LinuxDaemonMessageHeader::FileTruncateRequest
                                | LinuxDaemonMessageHeader::GetClockResolutionRequest
                                | LinuxDaemonMessageHeader::GetClockTimeRequest
                                | LinuxDaemonMessageHeader::GetPeerNameRequest
                                | LinuxDaemonMessageHeader::GetSockNameRequest
                                | LinuxDaemonMessageHeader::ListenSocketRequest
                                | LinuxDaemonMessageHeader::OpenAtRequest
                                | LinuxDaemonMessageHeader::PartialReadRequest
                                | LinuxDaemonMessageHeader::PartialWriteRequest
                                | LinuxDaemonMessageHeader::ReceiveSocketRequest
                                | LinuxDaemonMessageHeader::RenameAtRequest
                                | LinuxDaemonMessageHeader::SeekRequest
                                | LinuxDaemonMessageHeader::SendSocketRequest
                                | LinuxDaemonMessageHeader::ShutdownSocketRequest
                                | LinuxDaemonMessageHeader::TimesRequest
                                | LinuxDaemonMessageHeader::UnlinkAtRequest
                                | LinuxDaemonMessageHeader::PipeRequest
                                | LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                                    self.handle_short_request_messages(source, message)
                                },

                                // The following system calls have their request data fit in a
                                // single message, but their response data is too large to fit in a
                                // single message. Thus, their response is split into multiple
                                // messages.
                                LinuxDaemonMessageHeader::FileStatRequest
                                | LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest
                                | LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                                    self.handle_long_response_messages(source, message);
                                    continue;
                                },

                                // The following system calls have request data that is too large to
                                // fit in a single message. Thus, their request is split into multiple
                                // messages.
                                LinuxDaemonMessageHeader::FileStatAtRequestPart
                                | LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart
                                | LinuxDaemonMessageHeader::LinkAtRequestPart
                                | LinuxDaemonMessageHeader::ReadLinkAtRequestPart
                                | LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart
                                | LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart
                                | LinuxDaemonMessageHeader::FileChownAtRequestPart
                                | LinuxDaemonMessageHeader::FileChmodAtRequestPart
                                | LinuxDaemonMessageHeader::OpenAtRequestPart => {
                                    self.handle_long_request_messages(source, message);
                                    continue;
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

        self.send_eof()
    }

    ///
    /// # Description
    ///
    /// Sends an EOF message to the gateway, to indicate that the sandbox hung up the connection.
    ///
    /// # Returns
    ///
    /// The function returns `Ok(())` if the EOF message was sent successfully. Otherwise, it
    /// returns an error.
    ///
    fn send_eof(&mut self) -> Result<(), Error> {
        trace!("send_eof()");
        if let Some(ref mut conn) = self.gateway_conn {
            let eof: u32 = 0;
            let length_buffer: [u8; mem::size_of::<u32>()] = eof.to_le_bytes();
            if let Err(e) = conn.write_all(&length_buffer) {
                let reason: &str = "failed to write EOF to the gateway";
                error!("send_eof(): {:?} (error={:?}", reason, e);
                return Err(Error::new(ErrorCode::ConnectionReset, reason));
            }
        }

        Ok(())
    }

    fn handle_special_messages(
        &mut self,
        source: ProcessIdentifier,
        message: LinuxDaemonMessage,
    ) -> Message {
        match message.header {
            LinuxDaemonMessageHeader::CloseRequest => {
                let request: CloseRequest = CloseRequest::from_bytes(message.payload);
                self.handle_close_request(source, request)
            },
            LinuxDaemonMessageHeader::ReadRequest => {
                let request: ReadRequest = ReadRequest::from_bytes(message.payload);
                self.handle_read_request(source, request)
            },
            LinuxDaemonMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                self.handle_write_request(source, request)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected special message {:?}", header)
            },
        }
    }

    fn handle_short_request_messages(
        &mut self,
        source: ProcessIdentifier,
        message: LinuxDaemonMessage,
    ) -> Message {
        match message.header {
            LinuxDaemonMessageHeader::AcceptSocketRequest => {
                let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(message.payload);
                socket::do_accept(source, request)
            },
            LinuxDaemonMessageHeader::BindSocketRequest => {
                let request: BindSocketRequest = BindSocketRequest::from_bytes(message.payload);
                socket::do_bind(source, request)
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
            LinuxDaemonMessageHeader::CreateSocketRequest => {
                let request: CreateSocketRequest = CreateSocketRequest::from_bytes(message.payload);
                socket::do_socket(source, request)
            },
            LinuxDaemonMessageHeader::FileAdvisoryInformationRequest => {
                let request: FileAdvisoryInformationRequest =
                    FileAdvisoryInformationRequest::from_bytes(message.payload);
                fcntl::do_posix_fadvise(source, request)
            },
            LinuxDaemonMessageHeader::FileChdirRequest => {
                let request: FileChdirRequest = FileChdirRequest::from_bytes(message.payload);
                unistd::do_fchdir(source, request)
            },
            LinuxDaemonMessageHeader::FileChmodRequest => {
                let request: FileChmodRequest = FileChmodRequest::from_bytes(message.payload);
                unistd::do_fchmod(source, request)
            },
            LinuxDaemonMessageHeader::FileChownRequest => {
                let request: FileChownRequest = FileChownRequest::from_bytes(message.payload);
                unistd::do_fchown(source, request)
            },
            LinuxDaemonMessageHeader::FileControlRequest => {
                let request: FileControlRequest = FileControlRequest::from_bytes(message.payload);
                fcntl::do_fcntl(source, request)
            },
            LinuxDaemonMessageHeader::FileDataSyncRequest => {
                let request: FileDataSyncRequest = FileDataSyncRequest::from_bytes(message.payload);
                unistd::do_fdatasync(source, request)
            },
            LinuxDaemonMessageHeader::FileSpaceControlRequest => {
                let request: FileSpaceControlRequest =
                    FileSpaceControlRequest::from_bytes(message.payload);
                fcntl::do_posix_fallocate(source, request)
            },
            LinuxDaemonMessageHeader::FileSyncRequest => {
                let request: FileSyncRequest = FileSyncRequest::from_bytes(message.payload);
                unistd::do_fsync(source, request)
            },
            LinuxDaemonMessageHeader::FileTruncateRequest => {
                let request: FileTruncateRequest = FileTruncateRequest::from_bytes(message.payload);
                unistd::do_ftruncate(source, request)
            },
            LinuxDaemonMessageHeader::GetClockResolutionRequest => {
                let request: ClockResolutionRequest =
                    ClockResolutionRequest::from_bytes(message.payload);
                time::do_clock_getres(source, request)
            },
            LinuxDaemonMessageHeader::GetClockTimeRequest => {
                let request: GetClockTimeRequest = GetClockTimeRequest::from_bytes(message.payload);
                time::do_clock_gettime(source, request)
            },
            LinuxDaemonMessageHeader::GetPeerNameRequest => {
                let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(message.payload);
                socket::do_getpeername(source, request)
            },
            LinuxDaemonMessageHeader::GetSockNameRequest => {
                let request: GetSockNameRequest = GetSockNameRequest::from_bytes(message.payload);
                socket::do_getsockname(source, request)
            },
            LinuxDaemonMessageHeader::ListenSocketRequest => {
                let request: ListenSocketRequest = ListenSocketRequest::from_bytes(message.payload);
                socket::do_listen(source, request)
            },
            LinuxDaemonMessageHeader::PartialReadRequest => {
                let request: PartialReadRequest = PartialReadRequest::from_bytes(message.payload);
                unistd::do_pread(source, request)
            },
            LinuxDaemonMessageHeader::PartialWriteRequest => {
                let request: PartialWriteRequest = PartialWriteRequest::from_bytes(message.payload);
                unistd::do_pwrite(source, request)
            },
            LinuxDaemonMessageHeader::ReceiveSocketRequest => {
                let request: ReceiveSocketRequest =
                    ReceiveSocketRequest::from_bytes(message.payload);
                socket::do_recv(source, request)
            },
            LinuxDaemonMessageHeader::RenameAtRequest => {
                let request: RenameAtRequest = RenameAtRequest::from_bytes(message.payload);
                fcntl::do_rename_at(source, request)
            },
            LinuxDaemonMessageHeader::SeekRequest => {
                let request: SeekRequest = SeekRequest::from_bytes(message.payload);
                unistd::do_lseek(source, request)
            },
            LinuxDaemonMessageHeader::SendSocketRequest => {
                let request: SendSocketRequest = SendSocketRequest::from_bytes(message.payload);
                socket::do_send(source, request)
            },
            LinuxDaemonMessageHeader::ShutdownSocketRequest => {
                let request: ShutdownSocketRequest =
                    ShutdownSocketRequest::from_bytes(message.payload);
                socket::do_shutdown(source, request)
            },
            LinuxDaemonMessageHeader::TimesRequest => {
                let request: TimesRequest = TimesRequest::from_bytes(message.payload);
                times::do_times(source, request)
            },
            LinuxDaemonMessageHeader::UnlinkAtRequest => {
                let request: UnlinkAtRequest = UnlinkAtRequest::from_bytes(message.payload);
                fcntl::do_unlink_at(source, request)
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                let request: UpdateFileAccessTimeRequest =
                    UpdateFileAccessTimeRequest::from_bytes(message.payload);
                fcntl::do_futimens(source, request)
            },
            LinuxDaemonMessageHeader::PipeRequest => {
                let _request = PipeRequest::from_bytes(message.payload);
                unistd::do_pipe(source)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected short message {:?}", header)
            },
        }
    }

    fn handle_long_request_messages(
        &mut self,
        source: ProcessIdentifier,
        message: LinuxDaemonMessage,
    ) {
        match message.header {
            LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                self.handle_long_request::<FileStatAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                self.handle_long_request::<SymbolicLinkAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::LinkAtRequestPart => {
                self.handle_long_request::<LinkAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                self.handle_long_request::<ReadLinkAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                self.handle_long_request::<MakeDirectoryAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                self.handle_long_request::<UpdateFileAccessTimeAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                self.handle_long_request::<FileChownAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                self.handle_long_request::<FileChmodAtRequest>(source, &message);
            },
            LinuxDaemonMessageHeader::OpenAtRequestPart => {
                self.handle_long_request::<OpenAtRequest>(source, &message);
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long request message {:?}", header)
            },
        }
    }

    fn handle_long_response_messages(
        &mut self,
        source: ProcessIdentifier,
        message: LinuxDaemonMessage,
    ) {
        match message.header {
            LinuxDaemonMessageHeader::FileStatRequest => {
                self.handle_fstat_request(source, message);
            },
            LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest => {
                self.handle_getcwd_request(source);
            },
            LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                self.handle_getdents_request(source, message);
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long response message {:?}", header)
            },
        }
    }

    // Read a message from the TCP stream.
    fn recv(&mut self) -> Result<Option<Message>> {
        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];
        let mut buf_reader: &UnixStream = &self.stream;
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

    fn handle_close_request(
        &mut self,
        source: ProcessIdentifier,
        request: CloseRequest,
    ) -> Message {
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
    }

    fn handle_write_request(
        &mut self,
        source: ProcessIdentifier,
        request: WriteRequest,
    ) -> Message {
        trace!("handle_write_request(): source={:?}, request={:?}", source, request);
        // Check if writing to gateway.
        if request.fd == ::posix::unistd::STDOUT_FILENO {
            if let Some(ref mut conn) = self.gateway_conn {
                // Check if write size is invalid.
                if request.count == 0 {
                    // Writing zero-bytes to STDOUT is not allowed, as we used this to signal EOF.
                    error!("handle_write_request(): trying to write zero bytes to STDOUT");
                    build_error(source, ErrorCode::InvalidArgument)
                } else {
                    // NOTE: we don't check if the write operation is too big, because its size is
                    // already bound by the maximum payload size of the message.
                    let count: usize = request.count as usize;
                    let length_buffer: [u8; mem::size_of::<u32>()] = (count as u32).to_le_bytes();
                    match conn.write_all(&length_buffer) {
                        Ok(_) => {
                            match conn.write_all(&request.buffer[..count]) {
                                Ok(_) => {
                                    debug!("wrote {} bytes to the gateway", count);
                                    WriteResponse::build(source, count as i32)
                                },
                                Err(e) => {
                                    debug!("failed to write buffer to the gateway (error={:?})", e);
                                    // TODO: Check error conversion.
                                    build_error(source, ErrorCode::ConnectionReset)
                                },
                            }
                        },
                        Err(e) => {
                            debug!("failed to write length to the gateway (error={:?})", e);
                            // TODO: Check error conversion.
                            build_error(source, ErrorCode::ConnectionReset)
                        },
                    }
                }
            } else {
                // Not connected to the gateway, print to stdout.
                let count: usize = request.count as usize;
                let buffer: &[u8] = &request.buffer[..count];
                let string: String = String::from_utf8_lossy(buffer).to_string();
                print!("{}", string);
                let _ = io::stdout().lock().flush();
                WriteResponse::build(source, count as ssize_t)
            }
        } else {
            // Write to other file descriptor.
            unistd::do_write(source, request)
        }
    }

    fn handle_read_request(&mut self, source: ProcessIdentifier, request: ReadRequest) -> Message {
        trace!("handle_read_request(): source={:?}, request={:?}", source, request);
        // Check if reading from gateway.
        if request.fd == ::posix::unistd::STDIN_FILENO {
            if let Some(ref mut conn) = self.gateway_conn {
                // Check if the process is associated with a virtual environment.
                let env: &mut VirtualEnvironment = if let Some(env) = self.venv.get_mut(source) {
                    env
                } else {
                    warn!(
                        "handle_read_request(): process is not associated with a virtual \
                         environment, returning EOF"
                    );
                    return ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]);
                };

                // Check if there are any outstanding messages ready to be read.
                if let Some(message) = env.pop_stdin_message() {
                    trace!("handle_read_request(): reading outstanding message");
                    return message;
                }

                let mut length_buffer: [u8; mem::size_of::<u32>()] = [0u8; mem::size_of::<u32>()];
                match conn.read_exact(&mut length_buffer) {
                    Ok(_) => {
                        let length: u32 = u32::from_le_bytes(length_buffer);
                        if length == 0 {
                            debug!("read 0 bytes from the gateway");
                            ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE])
                        } else {
                            let count: usize = length as usize;
                            let mut buf: Vec<u8> = vec![0u8; count];
                            match conn.read_exact(&mut buf) {
                                Ok(_) => {
                                    debug!("read {} bytes from the gateway", count);

                                    // Truncate read request to fit in the response buffer.
                                    let read_count: usize = if count > request.count as usize {
                                        warn!(
                                            "handle_read_request(): truncating payload \
                                             (requested={}, actual={})",
                                            { request.count },
                                            count
                                        );
                                        request.count as usize
                                    } else {
                                        count
                                    };

                                    let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] =
                                        [0u8; ReadResponse::BUFFER_SIZE];
                                    response_buf[..read_count].copy_from_slice(&buf[..read_count]);

                                    // Check if there are any outstanding bytes to be read.
                                    if count > read_count {
                                        // Break outstanding bytes into multiple read responses.
                                        for i in
                                            (read_count..count).step_by(ReadResponse::BUFFER_SIZE)
                                        {
                                            let end: usize = i + ReadResponse::BUFFER_SIZE;
                                            let end: usize = if end > count { count } else { end };
                                            let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] =
                                                [0u8; ReadResponse::BUFFER_SIZE];
                                            response_buf[..end - i].copy_from_slice(&buf[i..end]);
                                            env.push_stdin_message(ReadResponse::build(
                                                source,
                                                (end - i) as ssize_t,
                                                response_buf,
                                            ));
                                        }
                                    }
                                    // Push EoF message.
                                    env.push_stdin_message(ReadResponse::build(
                                        source,
                                        0,
                                        [0u8; ReadResponse::BUFFER_SIZE],
                                    ));

                                    ReadResponse::build(source, read_count as ssize_t, response_buf)
                                },
                                Err(e) => {
                                    debug!("failed to read from the gateway (error={:?})", e);
                                    // TODO: Check error conversion.
                                    build_error(source, ErrorCode::ConnectionReset)
                                },
                            }
                        }
                    },
                    Err(e) => {
                        debug!("failed to read length from the gateway (error={:?})", e);
                        // TODO: Check error conversion.
                        build_error(source, ErrorCode::ConnectionReset)
                    },
                }
            } else {
                // Not connected to the gateway, read from stdin.
                let mut buffer: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
                let count: usize = match ::std::io::stdin().read(&mut buffer) {
                    Ok(count) => count,
                    Err(e) => {
                        debug!("failed to read from stdin (error={:?})", e);
                        0
                    },
                };
                ReadResponse::build(source, count as ssize_t, buffer)
            }
        } else {
            // Read from other file descriptor.
            unistd::do_read(source, request)
        }
    }

    fn handle_fstat_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let request: FileStatRequest = FileStatRequest::from_bytes(message.payload);

        let messages: Vec<Message> = fcntl::do_fstat(source, request);
        for message in messages {
            if let Err(e) = self.send(message) {
                error!("failed to send message (error={:?})", e);
            }
        }
    }

    fn handle_getcwd_request(&mut self, source: ProcessIdentifier) {
        let messages: Vec<Message> = unistd::do_getcwd(source);
        for message in messages {
            if let Err(e) = self.send(message) {
                error!("failed to send message (error={:?})", e);
            }
        }
    }

    fn handle_getdents_request(&mut self, source: ProcessIdentifier, message: LinuxDaemonMessage) {
        let request: GetDirectoryEntriesRequest =
            GetDirectoryEntriesRequest::from_bytes(message.payload);

        let messages: Vec<Message> = dirent::do_getdents(source, request);
        for message in messages {
            if let Err(e) = self.send(message) {
                error!("failed to send message (error={:?})", e);
            }
        }
    }

    fn handle_long_request<T>(&mut self, source: ProcessIdentifier, message: &LinuxDaemonMessage)
    where
        T: RequestAssemblerTrait,
    {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        match self.assembler.process_message::<T>(source, part) {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = self.send(message) {
                        error!("failed to send message (error={:?})", e);
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process request (error={:?})", e);
                if let Err(e) = self.send(self.do_error(source, e.code)) {
                    error!("failed to send error message (error={:?})", e);
                }
            },
        }
    }
}

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
mod time;
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
use ::flexi_logger::Logger;
use ::linuxd::{
    fcntl::message::{
        FileAdvisoryInformationRequest,
        FileSpaceControlRequest,
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
    },
    sys::stat::message::FileStatAtRequest,
    time::message::{
        ClockResolutionRequest,
        GetClockTimeRequest,
    },
    unistd::message::{
        CloseRequest,
        FileDataSyncRequest,
        FileSyncRequest,
        FileTruncateRequest,
        SeekRequest,
    },
    venv::message::{
        JoinEnvRequest,
        LeaveEnvRequest,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::{
        config,
        error::{
            Error,
            ErrorCode,
        },
    },
};
use ::std::{
    env,
    io::{
        ErrorKind,
        Read,
        Write,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    sync::Once,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct ProcessDaemon {
    pid: ProcessIdentifier,
    assembler: RequestAssembler,
    stream: TcpStream,
    venv: VirtualEnviromentDirectory,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessDaemon {
    pub fn init(stream: TcpStream) -> Result<Self, Error> {
        Ok(Self {
            pid: ProcessIdentifier::from(0),
            assembler: RequestAssembler::default(),
            stream,
            venv: VirtualEnviromentDirectory::new(),
        })
    }

    pub fn run(&mut self) {
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
                                    unistd::do_close(source, request)
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
}

pub fn main() -> Result<()> {
    initialize();

    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    let sockaddr: String = args.server_sockaddr();

    let listener = match TcpListener::bind(sockaddr.clone()) {
        Ok(l) => l,
        Err(e) => {
            anyhow::bail!("Failed to bind: {}", e);
        },
    };
    let stream: TcpStream = match listener.accept() {
        Ok((s, sockaddr)) => {
            info!("Connected to: {}", sockaddr);
            s
        },
        Err(e) => {
            anyhow::bail!("Failed to connect: {}", e);
        },
    };

    let mut procd: ProcessDaemon = match ProcessDaemon::init(stream) {
        Ok(procd) => procd,
        Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
    };

    procd.run();

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
pub fn initialize() {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        Logger::try_with_env()
            .expect("malformed RUST_LOG environment variable")
            .start()
            .expect("failed to initialize logger");
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
    Message::new(linuxd::LINUXD, pid, MessageType::Ikc, Some(error), [0u8; Message::PAYLOAD_SIZE])
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
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => Ok(assembler.is_complete()),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.take_parts(),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fstat_at(source, request)
    }
}

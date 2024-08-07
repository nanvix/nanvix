// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use core::{
    ffi::CStr,
    str,
};

use ::error::Error;
use ::nvx::{
    event::{
        Event,
        EventCtrlRequest,
        SchedulingEvent,
    },
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        Capability,
        ProcessIdentifier,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
    },
};
use error::ErrorCode;
use nvx::pm::{
    LookupMessage,
    SignupMessage,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

struct ProcessDaemon {
    pid: ProcessIdentifier,
    ndaemons: usize,
    ndaemons_expected: usize,
    testd: ProcessIdentifier,
    memd: Option<(ProcessIdentifier, [u8; SignupMessage::NAME_SIZE])>,
}

impl ProcessDaemon {
    fn init(testd: ProcessIdentifier) -> Self {
        ::nvx::log!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::nvx::pm::getpid().expect("failed to get pid");
        assert_eq!(mypid, ProcessIdentifier::PROCD, "procd has unexpected pid");

        // Acquire process management capabilities.
        ::nvx::log!("acquiring process managemnet capabilities...");
        if let Err(e) = ::nvx::pm::capctl(Capability::ProcessManagement, true) {
            panic!("failed to acquire process management capabilities (error={:?})", e);
        }

        // Subscribe to process termination.
        ::nvx::log!("subscribing to process termination...");
        if let Err(e) = ::nvx::event::evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Register,
        ) {
            panic!("failed to subscribe to process termination (error={:?})", e);
        }

        Self {
            pid: mypid,
            ndaemons: 0,
            ndaemons_expected: 1,
            testd,
            memd: None,
        }
    }

    fn run(&mut self) {
        loop {
            match ::nvx::ipc::recv() {
                Ok(message) => {
                    ::nvx::log!("received message from={:?}", message.source,);
                    match message.message_type {
                        MessageType::Exception => unreachable!("should not receive exceptions"),
                        MessageType::Ipc => {
                            if let Err(e) = self.handle_ipc_message(message) {
                                ::nvx::log!("failed to handle IPC message (error={:?})", e);
                            }
                        },
                        MessageType::Interrupt => unreachable!("should not receive interrupts"),
                        MessageType::SchedulingEvent => {
                            match self.handle_scheduling_event(message) {
                                Ok(true) => break,
                                Ok(false) => continue,
                                Err(e) => {
                                    ::nvx::log!("failed to handle scheduling event (error={:?})", e)
                                },
                            }
                        },
                    }
                },
                Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
            }
        }
    }

    fn handle_scheduling_event(&mut self, message: Message) -> Result<bool, Error> {
        // Deserialize process identifier.
        let pid: ProcessIdentifier =
            ProcessIdentifier::from(u32::from_le_bytes(message.payload[0..4].try_into().unwrap()));

        ::nvx::log!("received scheduling event (pid={:?})", pid);

        // Deserialize process status.
        let status: i32 = i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
        ::nvx::log!("process terminated (pid={:?}, status={:?})", pid, status);

        // Check if process is the test daemon.
        if pid == self.testd {
            return Ok(true);
        }

        // Check if process is the memory daemon.
        if let Some((p, _)) = self.memd {
            if p == pid {
                self.memd = None;
            }
        }

        Ok(false)
    }

    fn handle_ipc_message(&mut self, message: Message) -> Result<(), Error> {
        let destionation: ProcessIdentifier = message.source;
        let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

        ::nvx::log!("received system message (header={:?})", message.header);

        // Parse message.
        match message.header {
            // Parse process management message.
            SystemMessageHeader::ProcessManagement => {
                let message: ProcessManagementMessage =
                    ProcessManagementMessage::from_bytes(message.payload)?;

                // Parse operation.
                match message.header {
                    ProcessManagementMessageHeader::Signup => {
                        let message: SignupMessage = SignupMessage::from_bytes(message.payload);
                        self.handle_signup(destionation, message)?;
                    },
                    ProcessManagementMessageHeader::SignupResponse => {
                        ::nvx::log!("received signup response message, ignoring...");
                    },
                    ProcessManagementMessageHeader::Lookup => {
                        let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                        self.handle_lookup(destionation, message)?;
                    },
                    ProcessManagementMessageHeader::LookupResponse => {
                        ::nvx::log!("received lookup response message, ignoring...");
                    },
                    ProcessManagementMessageHeader::Shutdown => {
                        ::nvx::log!("received shutdown message, ignoring...");
                    },
                }
            },
            // Parse memory management message.
            SystemMessageHeader::MemoryManagement => {
                ::nvx::log!("received memory management message, ignoring...");
            },
            // Parse filesystem management message.
            SystemMessageHeader::FilesystemManagement => {
                ::nvx::log!("received filesystem management message, ignoring...");
            },
        }

        Ok(())
    }

    fn handle_signup(
        &mut self,
        destination: ProcessIdentifier,
        message: SignupMessage,
    ) -> Result<(), Error> {
        let pid: ProcessIdentifier = message.pid;
        match CStr::from_bytes_until_nul(&message.name) {
            Ok(name) => {
                ::nvx::log!("signing up process (pid={:?}, name={:?})", pid, name);

                // Register memory daemon.
                if name.to_bytes() == b"memd" {
                    self.memd = Some((pid, message.name));

                    self.ndaemons += 1;
                }

                ::nvx::pm::signup_response(destination, pid, 0)?;
            },
            Err(_) => ::nvx::pm::signup_response(
                destination,
                pid,
                ErrorCode::InvalidArgument.into_errno(),
            )?,
        }

        // Check if system initialization is complete.
        if self.ndaemons == self.ndaemons_expected {
            // Unblock test daemon.
            ::nvx::log!("unblocking test daemon...");
            let message: Message =
                Message::new(self.pid, self.testd, [0; Message::SIZE], MessageType::Ipc);
            if let Err(e) = ::nvx::ipc::send(&message) {
                panic!("failed to unblock test daemon(error={:?})", e);
            }
        }

        Ok(())
    }

    pub fn handle_lookup(
        &mut self,
        destination: ProcessIdentifier,
        message: LookupMessage,
    ) -> Result<(), Error> {
        let name: &CStr = CStr::from_bytes_until_nul(&message.name)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid process name"))?;

        ::nvx::log!("looking up process (name={:?})", name);

        // Check if process is the memory daemon.
        if name.to_bytes() == b"memd" {
            if let Some((pid, _)) = self.memd {
                ::nvx::pm::lookup_response(destination, pid, 0)?;
            } else {
                ::nvx::pm::lookup_response(
                    destination,
                    ProcessIdentifier::from(u32::MAX),
                    ErrorCode::NoSuchEntry.into_errno(),
                )?;
            }
        } else {
            ::nvx::pm::lookup_response(
                destination,
                ProcessIdentifier::from(u32::MAX),
                ErrorCode::NoSuchEntry.into_errno(),
            )?;
        }

        Ok(())
    }

    fn shutdown(&mut self) {
        ::nvx::log!("shutting down process manager daemon...");
        if let Some((pid, _)) = self.memd {
            ::nvx::pm::shutdown(pid, 0).expect("failed to broadcast shutdown message");
        }

        // Wait for memory daemon to terminate.
        while self.ndaemons > 0 {
            match ::nvx::ipc::recv() {
                Ok(message) => match message.message_type {
                    MessageType::Exception => unreachable!("should not receive exceptions"),
                    MessageType::Ipc => unreachable!("should not receive IPC messages"),
                    MessageType::Interrupt => unreachable!("should not receive interrupts"),
                    MessageType::SchedulingEvent => {
                        // Deserialize process identifier.
                        let pid: ProcessIdentifier = ProcessIdentifier::from(u32::from_le_bytes(
                            message.payload[0..4].try_into().unwrap(),
                        ));

                        // Deserialize process status.
                        let status: i32 =
                            i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
                        ::nvx::log!("process terminated (pid={:?}, status={:?})", pid, status);

                        self.ndaemons -= 1;
                    },
                },
                Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
            }
        }
    }
}

#[no_mangle]
pub fn main() {
    let testd: ProcessIdentifier = ProcessIdentifier::from(3);
    let mut procd: ProcessDaemon = ProcessDaemon::init(testd);

    procd.run();
    procd.shutdown();

    let magic_string: &str = "PANIC: Hello World!\n";
    let _ = ::nvx::debug::debug(magic_string.as_ptr(), magic_string.len());
}

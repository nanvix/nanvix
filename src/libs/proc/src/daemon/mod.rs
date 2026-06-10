// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupMessage,
};
use ::alloc::{
    collections::btree_map::BTreeMap,
    string::{
        String,
        ToString,
    },
};
use ::core::{
    ffi::CStr,
    str,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct ProcessDaemon {
    // FIXME: auto-signup process on process creation.
    processes: BTreeMap<ProcessIdentifier, (String, Option<ProcessIdentity>)>,
    /// Parent of each process, recorded when its process-creation scheduling event is observed.
    /// Used to decide whether a process termination should trigger system shutdown: only the
    /// boot/init workload (a process created directly by the kernel) initiates shutdown, whereas
    /// runtime-spawned children are simply reaped.
    parents: BTreeMap<ProcessIdentifier, ProcessIdentifier>,
}

impl ProcessDaemon {
    /// Initializes the process manager daemon.
    pub fn init() -> Result<Self, Error> {
        ::syslog::info!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;
        assert_eq!(mypid, ProcessIdentifier::PROCD, "process daemon has unexpected pid");

        // Acquire process management capabilities.
        ::syslog::info!("acquiring process managemnet capabilities...");
        ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true)?;

        // Subscribe to process termination.
        ::syslog::info!("subscribing to process termination...");
        ::sys::kcall::event::__kcall_evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Register,
        )?;

        Ok(Self {
            processes: BTreeMap::new(),
            parents: BTreeMap::new(),
        })
    }

    /// Runs the process manager daemon.
    /// Returns the exit status of the non-daemon process that triggered shutdown.
    pub fn run(&mut self) -> i32 {
        loop {
            match ::sys::kcall::ipc::__kcall_recv() {
                Ok(message) => {
                    ::syslog::info!("received message from={:?}", { message.source });
                    match message.message_type {
                        MessageType::Exception => unreachable!("should not receive exceptions"),
                        MessageType::Ipc => {
                            if let Err(e) = self.handle_ipc_message(message) {
                                ::syslog::error!("failed to handle IPC message (error={:?})", e);
                            }
                        },
                        MessageType::Interrupt => unreachable!("should not receive interrupts"),
                        MessageType::Ikc => unreachable!("should not receive IKC messages"),
                        MessageType::ProcessTerminationEvent => {
                            match self.handle_process_termination_event(message) {
                                Ok(Some(status)) => return status,
                                Ok(None) => continue,
                                Err(e) => {
                                    ::syslog::error!(
                                        "failed to handle scheduling event (error={:?})",
                                        e
                                    )
                                },
                            }
                        },
                        MessageType::PullResponse => {
                            ::syslog::error!("received unexpected pull response, ignoring");
                            continue;
                        },
                        MessageType::ProcessCreationEvent => {
                            if let Err(e) = self.handle_process_creation_event(message) {
                                ::syslog::error!(
                                    "failed to handle process creation event (error={:?})",
                                    e
                                );
                            }
                            continue;
                        },
                    }
                },
                Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
            }
        }
    }

    /// Handles a process-creation scheduling event.
    ///
    /// Records the new process and its parent so that, when the process later terminates, the
    /// daemon can tell whether it is the boot/init workload (created directly by the kernel) or a
    /// runtime-spawned child. Only termination of the former triggers system shutdown.
    fn handle_process_creation_event(&mut self, message: Message) -> Result<(), Error> {
        // Deserialize the identifier of the newly created process.
        let raw_pid_bytes: [u8; 4] = match message.payload[0..4].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                let reason: &str = "invalid process creation message payload";
                ::syslog::error!("handle_process_creation_event(): {reason:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let pid: ProcessIdentifier = ProcessIdentifier::from(i32::from_le_bytes(raw_pid_bytes));

        // Deserialize the identifier of the parent process.
        let raw_parent_bytes: [u8; 4] = match message.payload[4..8].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                let reason: &str = "invalid process creation message payload";
                ::syslog::error!("handle_process_creation_event(): {reason:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let parent: ProcessIdentifier =
            ProcessIdentifier::from(i32::from_le_bytes(raw_parent_bytes));

        ::syslog::info!("process created (pid={:?}, parent={:?})", pid, parent);
        self.parents.insert(pid, parent);

        Ok(())
    }

    fn handle_process_termination_event(&mut self, message: Message) -> Result<Option<i32>, Error> {
        // Deserialize process identifier.
        let raw_pid_bytes: [u8; 4] = match message.payload[0..4].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                let reason: &str = "invalid process termination message payload";
                ::syslog::error!("handle_process_termination_event(): {reason:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let pid: ProcessIdentifier = ProcessIdentifier::from(i32::from_le_bytes(raw_pid_bytes));

        ::syslog::info!("received scheduling event (pid={:?})", pid);

        // Deserialize process status.
        let status: i32 = i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
        ::syslog::info!("process terminated (pid={:?}, status={:?})", pid, status);

        // Forget the recorded lineage for the terminated process.
        let parent: Option<ProcessIdentifier> = self.parents.remove(&pid);

        // De-register process.
        if let Some((name, _identity)) = self.processes.remove(&pid) {
            ::syslog::info!("deregistering process (pid={:?}, name={:?}", pid, name,);

            // A daemon terminated — not a shutdown trigger (unless it crashed).
            if Self::is_daemon(&name) {
                if status != 0 {
                    ::syslog::error!(
                        "critical daemon {:?} terminated with non-zero status {} — triggering \
                         shutdown",
                        name,
                        status
                    );
                    return Ok(Some(status));
                }
                return Ok(None);
            }
        }

        // A non-daemon process terminated. Only the boot/init workload — a process created
        // directly by the kernel (parent is the kernel process) — initiates system shutdown.
        // Runtime-spawned children (created by another user process) are simply reaped, so that
        // workloads which spawn helper processes do not bring the whole system down when those
        // helpers exit. A process whose creation event has not been observed (no recorded parent)
        // is treated as a transient child and reaped without shutdown, since the boot workload's
        // creation is always observed long before it terminates.
        match parent {
            Some(parent) if parent == ProcessIdentifier::KERNEL => {
                // Return the exit status so procd can propagate it.
                Ok(Some(status))
            },
            _ => Ok(None),
        }
    }

    /// Returns `true` if `name` belongs to a system daemon that should not trigger shutdown.
    fn is_daemon(name: &str) -> bool {
        ::config::daemons::DAEMON_NAMES.contains(&name)
    }

    fn handle_ipc_message(&mut self, message: Message) -> Result<(), Error> {
        let destination: ProcessIdentifier = match { message.source }.as_id() {
            Ok(pid) => pid,
            Err(tid) => {
                let reason: &str = "invalid IPC message source";
                ::syslog::error!("handle_ipc_message(): {reason:?} (tid={:?})", tid);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

        ::syslog::info!("received system message (header={:?})", message.header);

        // Parse message.
        if let SystemMessageHeader::ProcessManagement = message.header {
            let message: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(message.payload)?;

            // Parse operation.
            match message.header {
                ProcessManagementMessageHeader::Signup => {
                    let message: SignupMessage = SignupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_signup(destination, message)?;
                    ::sys::kcall::ipc::__kcall_send(&message)?;
                },
                ProcessManagementMessageHeader::Lookup => {
                    let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_lookup(destination, message)?;
                    ::sys::kcall::ipc::__kcall_send(&message)?;
                },
                // Ignore all other messages.
                _ => {},
            }
        }

        Ok(())
    }

    // Handles a signup message.
    fn handle_signup(
        &mut self,
        destination: ProcessIdentifier,
        message: SignupMessage,
    ) -> Result<Message, Error> {
        let pid: ProcessIdentifier = message.pid;
        match CStr::from_bytes_until_nul(&message.name) {
            Ok(cstr) => match cstr.to_str() {
                Ok(name) => {
                    let s: String = name.to_string();

                    if s == ::config::daemons::MEMD_NAME {
                        ::syslog::info!("signup memory daemon");
                    } else {
                        ::syslog::info!("signup other process = {:?}", name);
                    }

                    ::syslog::info!("signing up process (pid={:?}, name={:?})", pid, s.as_bytes());
                    self.processes.insert(pid, (s, None));
                    message::signup_response(destination, pid, 0)
                },
                Err(_) => {
                    message::signup_response(destination, pid, ErrorCode::InvalidArgument.get())
                },
            },
            Err(_) => message::signup_response(destination, pid, ErrorCode::InvalidArgument.get()),
        }
    }

    // Handles a lookup message.
    pub fn handle_lookup(
        &self,
        destination: ProcessIdentifier,
        message: LookupMessage,
    ) -> Result<Message, Error> {
        let name: &str = match CStr::from_bytes_until_nul(&message.name) {
            Ok(name) => match name.to_str() {
                Ok(s) => s,
                Err(_) => {
                    let message: Message = message::lookup_response(
                        destination,
                        ProcessIdentifier::from(i32::MAX),
                        ErrorCode::InvalidArgument.get(),
                    )?;
                    return Ok(message);
                },
            },
            Err(_) => {
                let message: Message = message::lookup_response(
                    destination,
                    ProcessIdentifier::from(i32::MAX),
                    ErrorCode::InvalidArgument.get(),
                )?;
                return Ok(message);
            },
        };

        // Check if process is the memory daemon.
        for (pid, (pname, _identity)) in self.processes.iter() {
            ::syslog::info!("looking up process (name={:?}, pname={:?})", name, pname);

            if pname == name {
                let message: Message = message::lookup_response(destination, *pid, 0)?;
                return Ok(message);
            }
        }
        let message: Message = message::lookup_response(
            destination,
            ProcessIdentifier::from(i32::MAX),
            ErrorCode::NoSuchEntry.get(),
        )?;

        Ok(message)
    }

    pub fn shutdown(&mut self) {
        ::syslog::info!("shutting down process manager daemon...");

        for (pid, (pname, _identity)) in self.processes.iter() {
            ::syslog::info!("shutting down process (pid={:?}, name={:?})", pid, pname);
            let message: Message =
                message::shutdown_request(*pid, 0).expect("failed to broadcast shutdown message");
            ::sys::kcall::ipc::__kcall_send(&message)
                .expect("failed to broadcast shutdown message");
        }

        // Wait for memory daemon to terminate.
        while !self.processes.is_empty() {
            match ::sys::kcall::ipc::__kcall_recv() {
                Ok(message) => {
                    if message.message_type == MessageType::ProcessTerminationEvent {
                        // Deserialize process identifier.
                        let pid: ProcessIdentifier = ProcessIdentifier::from(i32::from_le_bytes(
                            message.payload[0..4].try_into().unwrap(),
                        ));

                        // Deserialize process status.
                        let status: i32 =
                            i32::from_le_bytes(message.payload[4..8].try_into().unwrap());

                        // De-register process.
                        if let Some((name, _identity)) = self.processes.remove(&pid) {
                            ::syslog::info!(
                                "process terminated (name={:?}, pid={:?}, status={:?})",
                                name,
                                pid,
                                status
                            );
                        } else {
                            ::syslog::info!(
                                "unknown process terminated (pid={:?}, status={:?})",
                                pid,
                                status
                            );
                        }
                    }
                },
                Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
            }
        }
    }
}

impl Drop for ProcessDaemon {
    fn drop(&mut self) {
        // Unsubscribe from scheduling events.
        ::syslog::info!("unsubscribing from scheduling events...");
        if let Err(e) = ::sys::kcall::event::__kcall_evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Unregister,
        ) {
            ::syslog::error!("failed to unsubscribe from scheduling events (error={:?})", e);
        }

        ::syslog::info!("shutting down process manager daemon...");
        if let Err(e) = ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
            ::syslog::error!("failed to release process management capabilities (error={:?})", e);
        }
    }
}

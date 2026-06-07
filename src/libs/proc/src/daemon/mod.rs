// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    GetParentMessage,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    RegisterChildMessage,
    SignupMessage,
};
use ::alloc::{
    collections::btree_map::BTreeMap,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use ::core::ffi::CStr;
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
// Structures
//==================================================================================================

///
/// # Description
///
/// Bookkeeping record for a process tracked by the process manager daemon.
///
struct ProcessRecord {
    /// Process name.
    name: String,
    /// Process identity (credentials).
    #[allow(dead_code)]
    identity: Option<ProcessIdentity>,
    /// Process identifier of the parent (`None` for daemons and the root application).
    parent: Option<ProcessIdentifier>,
    /// Process identifiers of the live children.
    children: Vec<ProcessIdentifier>,
}

impl ProcessRecord {
    /// Instantiates a new process record.
    fn new(name: String, parent: Option<ProcessIdentifier>) -> Self {
        Self {
            name,
            identity: None,
            parent,
            children: Vec::new(),
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct ProcessDaemon {
    // FIXME: auto-signup process on process creation.
    processes: BTreeMap<ProcessIdentifier, ProcessRecord>,
    /// Process identifier of the root application (first non-daemon process with no parent).
    root_app: Option<ProcessIdentifier>,
}

impl ProcessDaemon {
    /// Initializes the process manager daemon.
    pub fn init() -> Result<Self, Error> {
        ::syslog::info!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;
        assert_eq!(mypid, crate::PROCD, "process daemon has unexpected pid");

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
            root_app: None,
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
                            ::syslog::error!(
                                "received unexpected process creation event, ignoring"
                            );
                            continue;
                        },
                    }
                },
                Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
            }
        }
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

        // Look up the terminating process in the registry.
        if let Some(record) = self.processes.get(&pid) {
            let name: String = record.name.clone();
            let is_root: bool = self.root_app == Some(pid) || record.parent.is_none();

            // A daemon terminated — not a shutdown trigger (unless it crashed).
            if Self::is_daemon(&name) {
                ::syslog::info!("deregistering daemon (pid={:?}, name={:?})", pid, name);
                self.processes.remove(&pid);
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

            // The root application terminated — initiate shutdown and propagate its exit status.
            if is_root {
                ::syslog::info!("root application terminated (pid={:?}, status={:?})", pid, status);
                self.processes.remove(&pid);
                self.root_app = None;
                return Ok(Some(status));
            }

            // A forked child terminated. Re-parent its surviving children to the root
            // application (orphan adoption by init), then drop its bookkeeping. Without
            // `waitpid()` there is no reaping step, so the record is removed immediately.
            let parent: Option<ProcessIdentifier> = record.parent;

            self.reparent_children(pid);

            if let Some(parent) = parent {
                if let Some(record) = self.processes.get_mut(&parent) {
                    record.children.retain(|child| *child != pid);
                }
            }
            self.processes.remove(&pid);

            return Ok(None);
        }

        // The terminating process is unknown to the registry.
        if self.root_app.is_some() {
            // A forked child likely terminated before its lineage was registered (see the
            // RegisterChild race). With no `waitpid()` to reap it, the event is ignored: the
            // root application is still alive, so this is not a shutdown trigger.
            ::syslog::info!(
                "unregistered child terminated (pid={:?}, status={:?}) — ignoring",
                pid,
                status
            );
            return Ok(None);
        }

        // No root application has been registered yet — this is the root application terminating
        // without having forked. Initiate shutdown and propagate the exit status.
        Ok(Some(status))
    }

    /// Re-parents the surviving children of `pid` to the root application.
    fn reparent_children(&mut self, pid: ProcessIdentifier) {
        let root: ProcessIdentifier = match self.root_app {
            Some(root) => root,
            None => return,
        };

        // Nothing to do if the terminating process is the root application itself.
        if root == pid {
            return;
        }

        let children: Vec<ProcessIdentifier> = match self.processes.get(&pid) {
            Some(record) => record.children.clone(),
            None => return,
        };

        for child in children {
            if let Some(record) = self.processes.get_mut(&child) {
                record.parent = Some(root);
            }
            if let Some(record) = self.processes.get_mut(&root) {
                if !record.children.contains(&child) {
                    record.children.push(child);
                }
            }
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
                ProcessManagementMessageHeader::RegisterChild => {
                    let message: RegisterChildMessage =
                        RegisterChildMessage::from_bytes(message.payload);
                    let message: Message = self.handle_register_child(destination, message)?;
                    ::sys::kcall::ipc::__kcall_send(&message)?;
                },
                ProcessManagementMessageHeader::GetParent => {
                    let message: GetParentMessage = GetParentMessage::from_bytes(message.payload);
                    let message: Message = self.handle_get_parent(destination, message)?;
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
                    // Preserve any existing lineage (parent/children) that may have been recorded
                    // by `RegisterChild` before this signup. Only update the process name.
                    match self.processes.get_mut(&pid) {
                        Some(record) => {
                            record.name = s;
                        },
                        None => {
                            self.processes.insert(pid, ProcessRecord::new(s, None));
                        },
                    }
                    message::signup_response(destination, pid, 0)
                },
                Err(_) => {
                    message::signup_response(destination, pid, ErrorCode::InvalidArgument.get())
                },
            },
            Err(_) => message::signup_response(destination, pid, ErrorCode::InvalidArgument.get()),
        }
    }

    // Handles a register-child message.
    fn handle_register_child(
        &mut self,
        destination: ProcessIdentifier,
        message: RegisterChildMessage,
    ) -> Result<Message, Error> {
        let child: ProcessIdentifier = message.child;
        let parent: ProcessIdentifier = message.parent;

        // Reject spoofed registrations: only the parent process may register its own child.
        if destination != parent {
            ::syslog::warn!(
                "register_child: sender mismatch (sender={:?}, parent={:?}, child={:?})",
                destination,
                parent,
                child
            );
            return message::register_child_response(
                destination,
                ErrorCode::OperationNotPermitted.get(),
            );
        }

        ::syslog::info!("registering child (child={:?}, parent={:?})", child, parent);
        // Ensure that a record exists for the parent. If the parent is seen for the first time, it
        // has no parent of its own and is therefore the root application.
        self.processes
            .entry(parent)
            .or_insert_with(|| ProcessRecord::new(String::new(), None));

        // Identify the root application: the first non-daemon process with no parent of its own.
        if self.root_app.is_none() {
            if let Some(record) = self.processes.get(&parent) {
                if record.parent.is_none() && !Self::is_daemon(&record.name) {
                    self.root_app = Some(parent);
                }
            }
        }

        // A forked child inherits the name of its parent.
        let child_name: String = self
            .processes
            .get(&parent)
            .map(|record| record.name.clone())
            .unwrap_or_default();

        // Insert or update the child record.
        match self.processes.get_mut(&child) {
            Some(record) => {
                record.parent = Some(parent);
            },
            None => {
                self.processes
                    .insert(child, ProcessRecord::new(child_name, Some(parent)));
            },
        }

        // Append the child to the parent's list of children.
        if let Some(record) = self.processes.get_mut(&parent) {
            if !record.children.contains(&child) {
                record.children.push(child);
            }
        }

        message::register_child_response(destination, 0)
    }

    // Handles a get-parent message.
    fn handle_get_parent(
        &mut self,
        destination: ProcessIdentifier,
        message: GetParentMessage,
    ) -> Result<Message, Error> {
        let pid: ProcessIdentifier = message.pid;

        // Resolve the parent. Processes with no recorded parent (the root application) and unknown
        // processes report the process manager daemon as their parent (init-like semantics).
        let parent: ProcessIdentifier = self
            .processes
            .get(&pid)
            .and_then(|record| record.parent)
            .unwrap_or(crate::PROCD);

        ::syslog::info!("get parent (pid={:?}, parent={:?})", pid, parent);

        message::get_parent_response(destination, parent, 0)
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
        for (pid, record) in self.processes.iter() {
            ::syslog::info!("looking up process (name={:?}, pname={:?})", name, record.name);

            if record.name == name {
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

        // Drop bookkeeping for forked children: only live daemons need to be shut down cleanly,
        // and dead processes will never produce further termination events.
        self.processes
            .retain(|_pid, record| Self::is_daemon(&record.name));

        for (pid, record) in self.processes.iter() {
            ::syslog::info!("shutting down process (pid={:?}, name={:?})", pid, record.name);
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
                        if let Some(record) = self.processes.remove(&pid) {
                            ::syslog::info!(
                                "process terminated (name={:?}, pid={:?}, status={:?})",
                                record.name,
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

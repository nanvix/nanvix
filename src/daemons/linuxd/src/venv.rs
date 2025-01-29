// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};
use ::posix::venv::{
    message::{
        JoinEnvRequest,
        JoinEnvResponse,
        LeaveEnvRequest,
        LeaveEnvResponse,
    },
    VirtualEnvironmentIdentifier,
};
use ::std::collections::{
    BTreeMap,
    VecDeque,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Virtual environment.
///
#[derive(Debug)]
pub struct VirtualEnvironment {
    /// Identifier.
    id: VirtualEnvironmentIdentifier,
    /// Standard input messages not yet consumed.
    stdin_messages: VecDeque<Message>,
}

///
/// # Description
///
/// Virtual environment directory.
///
pub struct VirtualEnviromentDirectory {
    /// Next environment identifier.
    next_env: VirtualEnvironmentIdentifier,
    /// Virtual environments.
    processes: BTreeMap<ProcessIdentifier, VirtualEnvironment>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualEnvironment {
    ///
    /// # Description
    ///
    /// Creates a new virtual environment.
    ///
    fn new(id: VirtualEnvironmentIdentifier) -> Self {
        Self {
            id,
            stdin_messages: VecDeque::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Gets the identifier of the virtual environment.
    ///
    fn id(&self) -> VirtualEnvironmentIdentifier {
        self.id
    }

    ///
    /// # Description
    ///
    /// Pushes a message to the standard input of the virtual environment.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to push.
    ///
    pub fn push_stdin_message(&mut self, message: Message) {
        self.stdin_messages.push_back(message);
    }

    ///
    /// # Description
    ///
    /// Pops a message from the standard input of the virtual environment.
    ///
    /// # Returns
    ///
    /// If there are messages in the standard input, the function returns the next message.
    /// Otherwise, it returns `None`.
    ///
    pub fn pop_stdin_message(&mut self) -> Option<Message> {
        self.stdin_messages.pop_front()
    }
}

impl PartialEq for VirtualEnvironment {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for VirtualEnvironment {}

impl VirtualEnviromentDirectory {
    pub fn new() -> Self {
        Self {
            next_env: VirtualEnvironmentIdentifier::default(),
            processes: BTreeMap::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Handles a join() request to a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `pid`: Requesting process identifier.
    /// - `request`: Join request.
    ///
    /// # Returns
    ///
    /// A response message indicating whether the request was successful or not.
    ///
    ///
    pub fn join(&mut self, pid: ProcessIdentifier, request: JoinEnvRequest) -> Message {
        trace!("join(): pid={:?}, request={:?}", pid, request);

        // Check if the process is already in an environment.
        if self.processes.contains_key(&pid) {
            warn!("process {:?} is previously joined environment {:?}", pid, self.processes[&pid]);
            return crate::build_error(pid, ErrorCode::ResourceBusy);
        }

        let mut envid: VirtualEnvironmentIdentifier = request.env;

        // Check wether the process requested to join a new environment or an existing one.
        if envid == VirtualEnvironmentIdentifier::NEW {
            // Process requested to join a new environment.
            envid = self.next_env;
            self.next_env = self.next_env.next();
            self.processes.insert(pid, VirtualEnvironment::new(envid));
            info!("process {:?} joined new environment {:?}", pid, envid);
        } else {
            // Process requested to join an existing environment.

            // Check if environment exists.
            if !self.processes.values().any(|v| v.id() == envid) {
                warn!("process {:?} requested to join non-existing environment {:?}", pid, envid);
                return crate::build_error(pid, ErrorCode::NoSuchEntry);
            }

            // Join environment.
            self.processes.insert(pid, VirtualEnvironment::new(envid));
        }

        JoinEnvResponse::build(pid, envid)
    }

    ///
    /// # Description
    ///
    /// Handles a leave() request from a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `pid`: Requesting process identifier.
    /// - `request`: Leave request.
    ///
    /// # Returns
    ///
    /// A response message indicating whether the request was successful or not.
    ///
    pub fn leave(&mut self, pid: ProcessIdentifier, request: LeaveEnvRequest) -> Message {
        trace!("leave(): pid={:?}", pid);

        // Check if the process has joined an environment.
        if !self.processes.contains_key(&pid) {
            error!("process {:?} has not joined an environment", pid);
            return crate::build_error(pid, ErrorCode::NoSuchEntry);
        }

        let envid: VirtualEnvironmentIdentifier = request.env;

        // Check if the process has previously joined the environment.
        if self.processes[&pid].id() != envid {
            error!("process {:?} has not previously joined environment {:?}", pid, envid);
            return crate::build_error(pid, ErrorCode::InvalidArgument);
        }

        // Leave environment.
        self.processes.remove(&pid);

        LeaveEnvResponse::build(pid, envid)
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the virtual environment of a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    ///
    /// # Returns
    ///
    /// If there is a virtual environment associated with the process, the function returns a
    /// mutable reference to the virtual environment. Otherwise, it returns `None`.
    ///
    pub fn get_mut(&mut self, pid: ProcessIdentifier) -> Option<&mut VirtualEnvironment> {
        self.processes.get_mut(&pid)
    }
}

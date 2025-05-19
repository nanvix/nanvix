// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::std::collections::{
    BTreeMap,
    VecDeque,
};
use ::syscall::venv::VirtualEnvironmentIdentifier;

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
    /// Joins a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `envid`: Virtual environment identifier.
    ///
    /// # Returns
    ///
    /// On success, an identifier to the virtual environment which the process joined is returned.
    /// Otherwise, an error is returned.
    ///
    pub fn join(
        &mut self,
        pid: ProcessIdentifier,
        mut envid: VirtualEnvironmentIdentifier,
    ) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("join(): pid={pid:?}, envid={envid:?}");

        // Check if the process is already in an environment.
        if self.processes.contains_key(&pid) {
            error!("process {:?} is previously joined environment {:?}", pid, self.processes[&pid]);
            return Err(Error::new(
                ErrorCode::ResourceBusy,
                "process is already in an environment",
            ));
        };

        // Check wether the process requested to join a new environment or an existing one.
        if envid == VirtualEnvironmentIdentifier::NEW {
            // Process requested to join a new environment.
            envid = self.next_env;
            self.next_env = self.next_env.next();
            self.processes.insert(pid, VirtualEnvironment::new(envid));
            info!("process {pid:?} joined new environment {envid:?}");
        } else {
            // Process requested to join an existing environment.

            // Check if environment exists.
            if !self.processes.values().any(|v| v.id() == envid) {
                error!("process {pid:?} requested to join non-existing environment {envid:?}");
                return Err(Error::new(
                    ErrorCode::NoSuchEntry,
                    "virtual environment does not exist",
                ));
            }

            // Join environment.
            self.processes.insert(pid, VirtualEnvironment::new(envid));
        }

        Ok(envid)
    }

    ///
    /// # Description
    ///
    /// Leaves a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `envid`: Virtual environment identifier.
    ///
    /// # Returns
    ///
    /// On success, an identifier to the virtual environment which the process left is returned.
    /// Otherwise, an error is returned.
    ///
    #[allow(dead_code)]
    pub fn leave(
        &mut self,
        pid: ProcessIdentifier,
        envid: VirtualEnvironmentIdentifier,
    ) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("leave(): pid={pid:?}, envid={envid:?}");

        // Check if the process has joined an environment.
        if !self.processes.contains_key(&pid) {
            error!("process {pid:?} has not joined an environment");
            return Err(Error::new(
                ErrorCode::NoSuchEntry,
                "process has not joined an environment",
            ));
        }

        // Check if the process has previously joined the environment.
        if self.processes[&pid].id() != envid {
            error!("process {pid:?} has not previously joined environment {envid:?}");
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "process has not previously joined environment",
            ));
        }

        // Leave environment.
        self.processes.remove(&pid);

        Ok(envid)
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the virtual environment of a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    ///
    /// # Returns
    ///
    /// If there is a virtual environment associated with the process, the function returns a
    /// reference to the virtual environment. Otherwise, it returns `None`.
    ///
    pub fn get(&self, pid: ProcessIdentifier) -> Option<&VirtualEnvironment> {
        self.processes.get(&pid)
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

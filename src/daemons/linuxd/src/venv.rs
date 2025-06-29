// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::collections::{
    BTreeMap,
    VecDeque,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
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
    processes: BTreeMap<ThreadIdentifier, VirtualEnvironment>,
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
    /// - `tid`: Thread identifier.
    /// - `envid`: Virtual environment identifier.
    ///
    /// # Returns
    ///
    /// On success, an identifier to the virtual environment which the process joined is returned.
    /// Otherwise, an error is returned.
    ///
    pub fn join(
        &mut self,
        tid: ThreadIdentifier,
        mut envid: VirtualEnvironmentIdentifier,
    ) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("join(): tid={tid:?}, envid={envid:?}");

        // Check if the process is already in an environment.
        if self.processes.contains_key(&tid) {
            error!("process {:?} is previously joined environment {:?}", tid, self.processes[&tid]);
            return Err(Error::new(
                ErrorCode::ResourceBusy,
                "process is already in an environment",
            ));
        };

        // Check wether the process requested to join a new environment or an existing one.
        if envid == VirtualEnvironmentIdentifier::NEW {
            // Thread requested to join a new environment.
            envid = self.next_env;
            self.next_env = self.next_env.next();
            self.processes.insert(tid, VirtualEnvironment::new(envid));
            info!("process {tid:?} joined new environment {envid:?}");
        } else {
            // Thread requested to join an existing environment.

            // Check if environment exists.
            if !self.processes.values().any(|v| v.id() == envid) {
                error!("process {tid:?} requested to join non-existing environment {envid:?}");
                return Err(Error::new(
                    ErrorCode::NoSuchEntry,
                    "virtual environment does not exist",
                ));
            }

            // Join environment.
            self.processes.insert(tid, VirtualEnvironment::new(envid));
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
    /// - `tid`: Thread identifier.
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
        tid: ThreadIdentifier,
        envid: VirtualEnvironmentIdentifier,
    ) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("leave(): tid={tid:?}, envid={envid:?}");

        // Check if the process has joined an environment.
        if !self.processes.contains_key(&tid) {
            error!("process {tid:?} has not joined an environment");
            return Err(Error::new(
                ErrorCode::NoSuchEntry,
                "process has not joined an environment",
            ));
        }

        // Check if the process has previously joined the environment.
        if self.processes[&tid].id() != envid {
            error!("process {tid:?} has not previously joined environment {envid:?}");
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "process has not previously joined environment",
            ));
        }

        // Leave environment.
        self.processes.remove(&tid);

        Ok(envid)
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the virtual environment of a process.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// If there is a virtual environment associated with the process, the function returns a
    /// reference to the virtual environment. Otherwise, it returns `None`.
    ///
    pub fn get(&self, tid: ThreadIdentifier) -> Option<&VirtualEnvironment> {
        self.processes.get(&tid)
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the virtual environment of a process.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// If there is a virtual environment associated with the process, the function returns a
    /// mutable reference to the virtual environment. Otherwise, it returns `None`.
    ///
    pub fn get_mut(&mut self, tid: ThreadIdentifier) -> Option<&mut VirtualEnvironment> {
        self.processes.get_mut(&tid)
    }
}

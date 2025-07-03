// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    collections::{
        BTreeMap,
        VecDeque,
    },
    sync::mpsc::{
        channel,
        Receiver,
        Sender,
    },
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
    /// Input channel to this virtual environment.
    channel_tx: Sender<Message>,
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
    fn new(id: VirtualEnvironmentIdentifier, channel_tx: Sender<Message>) -> Self {
        Self {
            id,
            stdin_messages: VecDeque::new(),
            channel_tx,
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

    ///
    /// # Description
    ///
    /// Returns a cloned transmitter channel for the virtual environment.
    ///
    /// # Returns
    ///
    /// A transmitter channel for the virtual environment.
    ///
    pub fn get_channel_tx(&self) -> Sender<Message> {
        self.channel_tx.clone()
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
    /// On success, a tuple containing the identifier of the virtual environment which the process
    /// joined and a receiver for messages is returned. Otherwise, an error is returned.
    ///
    pub fn join(
        &mut self,
        tid: ThreadIdentifier,
        mut envid: VirtualEnvironmentIdentifier,
    ) -> Result<(VirtualEnvironmentIdentifier, Sender<Message>, Receiver<Message>), Error> {
        trace!("join(): tid={tid:?}, envid={envid:?}");

        // Check if the process is already in an environment.
        if let Some(env) = self.processes.get(&tid) {
            let reason: &str = "process is already in an environment";
            error!("join(): {reason:?} (tid={tid:?}, envid={envid:?}, env={env:?})");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        };

        // Check whether the process requested to join a new environment or an existing one.
        if envid != VirtualEnvironmentIdentifier::NEW {
            let reason: &str = "joining existing environments is not supported";
            error!("join(): {reason:?} (tid={tid:?}, envid={envid:?})");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let (channel_tx, channel_rx): (Sender<Message>, Receiver<Message>) = channel::<Message>();

        // Thread requested to join a new environment.
        envid = self.next_env;
        self.next_env = self.next_env.next();
        let env = VirtualEnvironment::new(envid, channel_tx.clone());
        self.processes.insert(tid, env);
        info!("process {tid:?} joined new environment {envid:?}");

        Ok((envid, channel_tx, channel_rx))
    }

    ///
    /// # Description
    ///
    /// Leaves a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// On success, an identifier to the virtual environment which the process left is returned.
    /// Otherwise, an error is returned.
    ///
    pub fn leave(&mut self, tid: ThreadIdentifier) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("leave(): tid={tid:?}");

        // Leave environment.
        match self.processes.remove(&tid) {
            Some(env) => {
                info!("process {tid:?} left environment {env:?}");
                Ok(env.id())
            },
            None => {
                let reason: &str = "process has not joined an environment";
                error!("leave(): {reason:?} (tid={tid:?})");
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
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

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    collections::BTreeMap,
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

///
/// # Description
///
/// Unique identifier for each user VM.
///
pub type UserVmIdentifier = u32;

///
/// # Description
///
/// Unique identifier for each thread of execution. Each thread will belong to one user VM.
///
pub type GlobalThreadIdentifier = u64;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Commands that we can send to a worker thread in a virtual environment.
///
pub enum VenvCommand {
    Work(Message),
    Shutdown,
}

///
/// # Description
///
/// Virtual environment.
///
#[derive(Debug)]
pub struct VirtualEnvironment {
    /// Identifier.
    id: VirtualEnvironmentIdentifier,
    /// Input channel to this virtual environment.
    channel_tx: Sender<VenvCommand>,
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
    processes: BTreeMap<GlobalThreadIdentifier, VirtualEnvironment>,
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
    fn new(id: VirtualEnvironmentIdentifier, channel_tx: Sender<VenvCommand>) -> Self {
        Self { id, channel_tx }
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
    /// Returns a cloned transmitter channel for the virtual environment.
    ///
    /// # Returns
    ///
    /// A transmitter channel for the virtual environment.
    ///
    pub fn get_channel_tx(&self) -> Sender<VenvCommand> {
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
    /// Generate a global thread identifier to index virtual environments.
    ///
    /// # Parameters
    ///
    /// - `uvmid`: User VM identifier.
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// On success, a u64 where the user VM id sits in the first 32 bits, and the thread id in the
    /// second 32 bits. On error an error is returned.
    ///
    fn get_gtid(
        uvmid: UserVmIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<GlobalThreadIdentifier, Error> {
        let tid: u32 = match u32::try_from(tid) {
            Ok(val) => val,
            Err(_) => {
                let reason: &str = "error clipping thread id to u32";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ValueOverflow, reason));
            },
        };

        let key: u64 = ((uvmid as u64) << 32) | (tid as u64);
        Ok(key)
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
        uvmid: UserVmIdentifier,
        tid: ThreadIdentifier,
        mut envid: VirtualEnvironmentIdentifier,
    ) -> Result<(VirtualEnvironmentIdentifier, Sender<VenvCommand>, Receiver<VenvCommand>), Error>
    {
        trace!("join(): uvmid={uvmid}, tid={tid:?}, envid={envid:?}");

        // Check if the process is already in an environment.
        let gtid: GlobalThreadIdentifier = Self::get_gtid(uvmid, tid)?;
        if let Some(env) = self.processes.get(&gtid) {
            let reason: &str = "process is already in an environment";
            error!("join(): {reason:?} (uvmid={uvmid}, tid={tid:?}, envid={envid:?}, env={env:?})");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        };

        // Check whether the process requested to join a new environment or an existing one.
        if envid != VirtualEnvironmentIdentifier::NEW {
            let reason: &str = "joining existing environments is not supported";
            error!("join(): {reason:?} (uvmid={uvmid}, tid={tid:?}, envid={envid:?})");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let (channel_tx, channel_rx): (Sender<VenvCommand>, Receiver<VenvCommand>) =
            channel::<VenvCommand>();

        // Thread requested to join a new environment.
        envid = self.next_env;
        self.next_env = self.next_env.next();
        let env = VirtualEnvironment::new(envid, channel_tx.clone());
        self.processes.insert(gtid, env);
        info!("process {tid:?} (uvmid={uvmid}) joined new environment {envid:?}");

        Ok((envid, channel_tx, channel_rx))
    }

    ///
    /// # Description
    ///
    /// Leaves a virtual environment.
    ///
    /// # Parameters
    ///
    /// - `uvmid`: User VM identifier.
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// On success, an identifier to the virtual environment which the process left is returned.
    /// Otherwise, an error is returned.
    ///
    pub fn leave(
        &mut self,
        uvmid: UserVmIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<VirtualEnvironmentIdentifier, Error> {
        trace!("leave(): uvmid={uvmid} tid={tid:?}");

        // Leave environment.
        let gtid: GlobalThreadIdentifier = Self::get_gtid(uvmid, tid)?;
        match self.processes.remove(&gtid) {
            Some(env) => {
                info!("process {tid:?} (uvm={uvmid}) left environment {env:?}");
                Ok(env.id())
            },
            None => {
                let reason: &str = "process has not joined an environment";
                error!("leave(): {reason:?} (uvmid={uvmid}, tid={tid:?})");
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
    /// - `uvmid`: User VM identifier.
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// If there is a virtual environment associated with the process, the function returns a
    /// reference to the virtual environment. Otherwise, it returns `None`.
    ///
    pub fn get(
        &self,
        uvmid: UserVmIdentifier,
        tid: ThreadIdentifier,
    ) -> Option<&VirtualEnvironment> {
        let gtid: GlobalThreadIdentifier = match Self::get_gtid(uvmid, tid) {
            Ok(gtid) => gtid,
            Err(e) => {
                error!(
                    "error getting global thread identifier (uvmid={uvmid}, tid={tid:?}, \
                     error={e:?})"
                );
                return None;
            },
        };

        self.processes.get(&gtid)
    }
}

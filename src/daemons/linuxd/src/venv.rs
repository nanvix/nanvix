// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::venv::{
    message::{
        JoinEnvRequest,
        JoinEnvResponse,
        LeaveEnvRequest,
        LeaveEnvResponse,
    },
    VirtualEnvironmentIdentifier,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};
use ::std::collections::BTreeMap;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Virtual environment directory.
///
///
pub struct VirtualEnviromentDirectory {
    /// Next environment identifier.
    next_env: VirtualEnvironmentIdentifier,
    /// Virtual environments.
    processes: BTreeMap<ProcessIdentifier, VirtualEnvironmentIdentifier>,
}

//==================================================================================================
// Implementations
//==================================================================================================

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

        let mut env: VirtualEnvironmentIdentifier = request.env;

        // Check wether the process requested to join a new environment or an existing one.
        if env == VirtualEnvironmentIdentifier::NEW {
            // Process requested to join a new environment.
            env = self.next_env;
            self.next_env = self.next_env.next();
            self.processes.insert(pid, env);
            info!("process {:?} joined new environment {:?}", pid, env);
        } else {
            // Process requested to join an existing environment.

            // Check if environment exists.
            if !self.processes.values().any(|&v| v == env) {
                warn!("process {:?} requested to join non-existing environment {:?}", pid, env);
                return crate::build_error(pid, ErrorCode::NoSuchEntry);
            }

            // Join environment.
            self.processes.insert(pid, env);
        }

        JoinEnvResponse::build(pid, env)
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

        let env: VirtualEnvironmentIdentifier = request.env;

        // Check if the process has previously joined the environment.
        if self.processes[&pid] != env {
            error!("process {:?} has not previously joined environment {:?}", pid, env);
            return crate::build_error(pid, ErrorCode::InvalidArgument);
        }

        // Leave environment.
        self.processes.remove(&pid);

        LeaveEnvResponse::build(pid, env)
    }
}

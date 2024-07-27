// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::process::state::ProcessState,
    thread::ZombieThread,
};

//==================================================================================================
// Zombie Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a zombie process. A zombie process is a process that has finished its
/// execution and is waiting for its parent to collect its exit status and release its resources.
///
pub struct ZombieProcess {
    zombie: Option<ZombieThread>,
    process: Option<ProcessState>,
    status: i32,
}

impl ZombieProcess {
    pub const KILLED: i32 = -1;

    pub fn new(process: ProcessState, zombie: ZombieThread, status: i32) -> Self {
        Self {
            zombie: Some(zombie),
            process: Some(process),
            status,
        }
    }

    pub fn state(&self) -> &ProcessState {
        self.process.as_ref().unwrap()
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        self.process.as_mut().unwrap()
    }

    pub fn bury(&mut self) -> (ZombieThread, ProcessState, i32) {
        (self.zombie.take().unwrap(), self.process.take().unwrap(), self.status)
    }
}

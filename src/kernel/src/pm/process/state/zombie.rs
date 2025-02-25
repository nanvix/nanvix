// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::state::ProcessState,
    thread::ZombieThread,
};
use ::alloc::boxed::Box;
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Zombie Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that finished its execution and is waiting for its parent
/// to collect its exit status and release its resources.
///
pub struct ZombieProcess {
    zombie_threads: NonEmptyVecDeque<ZombieThread>,
    process: Box<ProcessState>,
    status: i32,
}

impl ZombieProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        zombie_threads: NonEmptyVecDeque<ZombieThread>,
        status: i32,
    ) -> Self {
        Self {
            zombie_threads,
            process,
            status,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.process
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.process
    }

    pub fn bury(self) -> (NonEmptyVecDeque<ZombieThread>, Box<ProcessState>, i32) {
        (self.zombie_threads, self.process, self.status)
    }
}

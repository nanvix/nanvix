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
use ::sys::{
    pm::ThreadIdentifier,
    ExitStatus,
};
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
#[derive(Debug)]
pub struct ZombieProcess {
    zombie_threads: NonEmptyVecDeque<ZombieThread>,
    process: Box<ProcessState>,
    status: ExitStatus,
}

impl ZombieProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        zombie_threads: NonEmptyVecDeque<ZombieThread>,
        status: ExitStatus,
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

    pub fn bury(self) -> (NonEmptyVecDeque<ZombieThread>, Box<ProcessState>, ExitStatus) {
        (self.zombie_threads, self.process, self.status)
    }

    pub fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        self.zombie_threads.iter().any(|thread| thread.tid() == tid)
    }
}

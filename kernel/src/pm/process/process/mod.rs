// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod interrupted;
mod runnable;
mod running;
mod state;
mod suspended;
mod zombie;

//==================================================================================================
// Exports
//==================================================================================================

pub use interrupted::{
    InterruptReason,
    InterruptedProcess,
};
pub use runnable::RunnableProcess;
pub use running::RunningProcess;
pub use state::ProcessState;
pub use suspended::SleepingProcess;
pub use zombie::ZombieProcess;

pub enum ProcessRefMut<'a> {
    Runnable(&'a mut RunnableProcess),
    Running(&'a mut RunningProcess),
    Sleeping(&'a mut SleepingProcess),
    Interrupted(&'a mut InterruptedProcess),
    Zombie(&'a mut ZombieProcess),
}

impl<'a> ProcessRefMut<'a> {
    pub fn state_mut(&mut self) -> &mut ProcessState {
        match self {
            ProcessRefMut::Runnable(process) => process.state_mut(),
            ProcessRefMut::Running(process) => process.state_mut(),
            ProcessRefMut::Sleeping(process) => process.state_mut(),
            ProcessRefMut::Interrupted(process) => process.state_mut(),
            ProcessRefMut::Zombie(process) => process.state_mut(),
        }
    }
}

pub enum ProcessRef<'a> {
    Runnable(&'a RunnableProcess),
    Running(&'a RunningProcess),
    Sleeping(&'a SleepingProcess),
    Interrupted(&'a InterruptedProcess),
    Zombie(&'a ZombieProcess),
}

impl<'a> ProcessRef<'a> {
    pub fn state(&self) -> &ProcessState {
        match self {
            ProcessRef::Runnable(process) => process.state(),
            ProcessRef::Running(process) => process.state(),
            ProcessRef::Sleeping(process) => process.state(),
            ProcessRef::Interrupted(process) => process.state(),
            ProcessRef::Zombie(process) => process.state(),
        }
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    pm::{
        process::state::{
            interrupted::interrupt,
            sleeping::SleepingProcess,
            ProcessState,
            RunnableProcess,
            ZombieProcess,
        },
        thread::{
            InterruptedThread,
            ReadyThread,
            RunningThread,
            SleepingThread,
            ZombieThread,
        },
    },
};
use ::sys::pm::ThreadIdentifier;
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that is running.
///
pub struct RunningProcess {
    /// Process state.
    state: ProcessState,
    /// Running thread.
    running: RunningThread,
    /// Ready threads.
    ready: Option<NonEmptyVecDeque<ReadyThread>>,
    /// Interrupted threads.
    interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
    /// Sleeping threads.
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    /// Zombie threads.
    zombie: Option<NonEmptyVecDeque<ZombieThread>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RunningProcess {
    pub(super) fn new(
        state: ProcessState,
        running: RunningThread,
        ready: Option<NonEmptyVecDeque<ReadyThread>>,
        interrupted: Option<NonEmptyVecDeque<InterruptedThread>>,
        sleeping: Option<NonEmptyVecDeque<SleepingThread>>,
        zombie: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state,
            running,
            ready,
            interrupted_threads: interrupted,
            sleeping_threads: sleeping,
            zombie,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn schedule(mut self) -> (RunnableProcess, *mut ContextInformation) {
        let running_thread = self.running;
        let (ready_thread, ctx) = running_thread.schedule();

        (
            RunnableProcess::from_state_with_ready_thread(
                self.state,
                NonEmptyVecDeque::new(ready_thread),
                self.interrupted_threads.take(),
                self.sleeping_threads.take(),
                self.zombie.take(),
            ),
            ctx,
        )
    }

    pub fn sleep(
        mut self,
    ) -> Result<
        (RunnableProcess, *mut ContextInformation),
        (SleepingProcess, *mut ContextInformation),
    > {
        let (sleeping_thread, ctx) = self.running.sleep();

        // Push sleeping thread.
        let sleeping_threads = match self.sleeping_threads.take() {
            Some(mut sleeping_threads) => {
                sleeping_threads.push_back(sleeping_thread);
                sleeping_threads
            },
            None => NonEmptyVecDeque::new(sleeping_thread),
        };

        // Check if there are ready threads.
        if let Some(ready_threads) = self.ready.take() {
            return Ok((
                RunnableProcess::from_state_with_ready_thread(
                    self.state,
                    ready_threads,
                    self.interrupted_threads.take(),
                    self.sleeping_threads.take(),
                    self.zombie.take(),
                ),
                ctx,
            ));
        }

        // Check if there are interrupted threads.
        if let Some(interrupted_threads) = self.interrupted_threads.take() {
            return Ok((
                RunnableProcess::from_state_with_interrupted_threads(
                    self.state,
                    None,
                    interrupted_threads,
                    self.sleeping_threads.take(),
                    self.zombie.take(),
                ),
                ctx,
            ));
        }

        Err((SleepingProcess::new(self.state, sleeping_threads), ctx))
    }

    pub fn exit(
        mut self,
        status: i32,
    ) -> Result<(RunnableProcess, *mut ContextInformation), (ZombieProcess, *mut ContextInformation)>
    {
        let (zombie_thread, ctx) = self.running.exit();
        let mut zombie_threads: NonEmptyVecDeque<ZombieThread> =
            NonEmptyVecDeque::new(zombie_thread);
        if let Some(ready_threads) = self.ready.take() {
            let more_zombie_threads = NonEmptyVecDeque::map(ready_threads, ReadyThread::terminate);
            zombie_threads.append(more_zombie_threads);
        }

        // Collect interrupted threads.
        let mut interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>> =
            self.interrupted_threads.take();

        // Terminate all sleeping threads.
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            let more_interrupted_threads = NonEmptyVecDeque::map(sleeping_threads, interrupt);
            match interrupted_threads.as_mut() {
                None => interrupted_threads = Some(more_interrupted_threads),
                Some(interrupted_threads) => interrupted_threads.append(more_interrupted_threads),
            }
        }

        if let Some(interrupted_threads) = interrupted_threads {
            Ok((
                RunnableProcess::from_state_with_interrupted_threads(
                    self.state,
                    None,
                    interrupted_threads,
                    None,
                    Some(zombie_threads),
                ),
                ctx,
            ))
        } else {
            Err((ZombieProcess::new(self.state, zombie_threads, status), ctx))
        }
    }

    pub fn get_tid(&self) -> ThreadIdentifier {
        self.running.id()
    }
}

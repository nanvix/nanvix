// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::Vmem,
    pm::{
        clock,
        process::state::{
            interrupted::interrupt,
            InterruptedProcess,
            ProcessState,
            RunningProcess,
            ZombieProcess,
        },
        thread::{
            InterruptReason,
            InterruptedThread,
            ReadyThread,
            SleepingThread,
            ZombieThread,
        },
    },
};
use ::alloc::boxed::Box;
use ::sys::pm::ProcessIdentifier;
use ::type_safe::NonEmptyVecDeque;
use alloc::collections::vec_deque::VecDeque;
use sys::{
    error::ErrorCode,
    pm::ThreadIdentifier,
    time::SystemTime,
};

//==================================================================================================
// Runnable Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that is ready to run.
///
#[derive(Debug)]
pub struct RunnableProcess {
    state: Box<ProcessState>,
    ready_threads: NonEmptyVecDeque<ReadyThread>,
    interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl RunnableProcess {
    pub fn new(pid: ProcessIdentifier, ready_thread: ReadyThread, vmem: Vmem) -> Self {
        Self {
            state: Box::new(ProcessState::new(pid, vmem)),
            ready_threads: NonEmptyVecDeque::new(ready_thread),
            interrupted_threads: None,
            sleeping_threads: None,
            zombie_threads: None,
        }
    }

    pub(super) fn from_state(
        state: Box<ProcessState>,
        ready_threads: NonEmptyVecDeque<ReadyThread>,
        interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
        sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state,
            ready_threads,
            interrupted_threads,
            sleeping_threads,
            zombie_threads,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub(super) fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn run(mut self) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation) {
        let mut ready_threads: VecDeque<ReadyThread> = self.ready_threads.into();

        // Select thread with the earliest admission time.
        let mut index_selected_thread: usize = 0;
        for (i, thread) in ready_threads.iter().enumerate() {
            if thread.admission_time() < ready_threads[index_selected_thread].admission_time() {
                index_selected_thread = i;
            }
        }
        let next_thread: ReadyThread = match ready_threads.remove(index_selected_thread) {
            Some(thread) => thread,
            None => {
                // SAFETY: the following statement is unreachable because there should always be at
                // least one ready thread in a runnable process.
                unreachable!("no ready threads in runnable process");
            },
        };

        let (running_thread, interrupt_reason, next_context) = next_thread.run();
        (
            RunningProcess::new(
                self.state,
                running_thread,
                NonEmptyVecDeque::from(ready_threads),
                self.interrupted_threads.take(),
                self.sleeping_threads.take(),
                self.zombie_threads.take(),
            ),
            interrupt_reason,
            next_context,
        )
    }

    pub fn terminate(mut self) -> Result<InterruptedProcess, ZombieProcess> {
        // Terminate all ready threads.
        let mut more_zombie_threads: NonEmptyVecDeque<ZombieThread> =
            NonEmptyVecDeque::map(self.ready_threads, ReadyThread::terminate);

        // Collect zombie threads.
        let zombie_threads: NonEmptyVecDeque<ZombieThread> = match self.zombie_threads.take() {
            Some(zombie_threads) => {
                more_zombie_threads.append(zombie_threads);
                more_zombie_threads
            },
            None => more_zombie_threads,
        };

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
            Ok(InterruptedProcess::new(self.state, interrupted_threads, Some(zombie_threads)))
        } else {
            Err(ZombieProcess::new(self.state, zombie_threads, ErrorCode::Interrupted.into()))
        }
    }

    pub fn wakeup(mut self, tid: ThreadIdentifier) -> Result<Self, Self> {
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            match sleeping_threads.remove_if(|thread| thread.id() == tid) {
                Ok((sleeping_threads, sleeping_thread)) => {
                    let ready_thread: ReadyThread = sleeping_thread.wakeup();
                    self.ready_threads.push_back(ready_thread);
                    Ok(Self::from_state(
                        self.state,
                        self.ready_threads,
                        self.interrupted_threads.take(),
                        NonEmptyVecDeque::from(sleeping_threads),
                        self.zombie_threads.take(),
                    ))
                },
                Err(sleeping_threads) => {
                    self.sleeping_threads = Some(sleeping_threads);
                    Err(self)
                },
            }
        } else {
            Err(self)
        }
    }

    pub fn add_thread(mut self, ready_thread: ReadyThread) -> Self {
        trace!("add_thread(): self.pid={:?}, ready_thread={:?}", self.state.pid, ready_thread);
        self.ready_threads.push_back(ready_thread);
        self
    }

    pub fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Search in the list of ready threads.
        if self.ready_threads.iter().any(|thread| thread.id() == tid) {
            return true;
        }

        // Search in the list of interrupted threads.
        if let Some(interrupted_threads) = &self.interrupted_threads {
            if interrupted_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &self.sleeping_threads {
            if sleeping_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &self.zombie_threads {
            if zombie_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        false
    }

    pub fn earliest_admission_time(&self) -> SystemTime {
        self.ready_threads
            .iter()
            .map(|thread| thread.admission_time())
            .min()
            .unwrap_or(clock::now())
    }
}

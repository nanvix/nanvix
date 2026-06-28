// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::state::{
        interrupted::InterruptedProcess,
        runnable::RunnableProcess,
        ProcessState,
    },
    thread::{
        InterruptReason,
        InterruptedThread,
        ReadyThread,
        SleepingThread,
        ThreadRef,
        ThreadRefMut,
        ZombieThread,
    },
};
use ::alloc::{
    boxed::Box,
    collections::vec_deque::VecDeque,
};
use ::sys::{
    pm::ThreadIdentifier,
    time::SystemTime,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Suspended Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that is waiting for a condition to be satisfied.
///
#[derive(Debug)]
pub struct SleepingProcess {
    state: Box<ProcessState>,
    sleeping_threads: NonEmptyVecDeque<SleepingThread>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl SleepingProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        sleeping_threads: NonEmptyVecDeque<SleepingThread>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state: process,
            sleeping_threads,
            zombie_threads,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of a candidate thread that can take delivery of signal `signum`, i.e.
    /// a sleeping thread that does not currently block it.
    ///
    /// A blocked signal must remain pending rather than interrupt a blocking call, so a thread whose
    /// mask blocks `signum` is not a valid interruption candidate.
    ///
    /// # Parameters
    ///
    /// - `signum`: The signal number (1-based).
    ///
    /// # Returns
    ///
    /// The identifier of a sleeping thread that does not block `signum`, or [`None`] if every
    /// sleeping thread blocks it.
    ///
    pub fn candidate_tid_for(&self, signum: usize) -> Option<ThreadIdentifier> {
        let bit: u64 = 1u64 << (signum - 1);
        self.sleeping_threads
            .iter()
            .find(|thread| (thread.thread_state().blocked() & bit) == 0)
            .map(|thread| thread.id())
    }

    pub fn terminate(self) -> InterruptedProcess {
        let (mut sleeping_threads, sleeping_thread): (VecDeque<SleepingThread>, SleepingThread) =
            self.sleeping_threads.pop_front();

        let mut interrupted_threads: NonEmptyVecDeque<InterruptedThread> =
            NonEmptyVecDeque::new(sleeping_thread.interrupt(InterruptReason::Killed));

        while let Some(sleeping_thread) = sleeping_threads.pop_front() {
            interrupted_threads.push_back(sleeping_thread.interrupt(InterruptReason::Killed));
        }

        InterruptedProcess::new(self.state, interrupted_threads, self.zombie_threads)
    }

    pub fn wakeup(mut self, tid: ThreadIdentifier) -> Result<RunnableProcess, SleepingProcess> {
        let sleeping_threads: NonEmptyVecDeque<SleepingThread> = self.sleeping_threads;

        // Search for the sleeping thread.
        match sleeping_threads.remove_if(|thread| thread.id() == tid) {
            Ok((sleeping_threads, sleeping_thread)) => {
                let ready_thread: ReadyThread = sleeping_thread.wakeup();
                Ok(RunnableProcess::from_state(
                    self.state,
                    NonEmptyVecDeque::new(ready_thread),
                    None,
                    NonEmptyVecDeque::from(sleeping_threads),
                    self.zombie_threads.take(),
                ))
            },
            Err(sleeping_threads) => {
                self.sleeping_threads = sleeping_threads;
                Err(self)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Interrupts a single sleeping thread of this suspended process with the given reason, leaving
    /// any remaining threads asleep.
    ///
    /// This is the signal-interruption counterpart of [`Self::wakeup`]: where `wakeup` resumes a
    /// thread so its blocking call re-evaluates, this resumes a thread so its blocking call reports
    /// the interruption (and, for [`InterruptReason::Signaled`], returns `EINTR` or transparently
    /// restarts at the return-to-user checkpoint).
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread to interrupt.
    /// - `reason`: Reason recorded for the interruption.
    ///
    /// # Returns
    ///
    /// On success, the process is returned as an [`InterruptedProcess`] carrying the interrupted
    /// thread. If no sleeping thread matches `tid`, the unchanged [`SleepingProcess`] is returned.
    ///
    pub fn interrupt_thread(
        mut self,
        tid: ThreadIdentifier,
        reason: InterruptReason,
    ) -> Result<InterruptedProcess, SleepingProcess> {
        let sleeping_threads: NonEmptyVecDeque<SleepingThread> = self.sleeping_threads;

        // Search for the sleeping thread.
        match sleeping_threads.remove_if(|thread| thread.id() == tid) {
            Ok((sleeping_threads, sleeping_thread)) => {
                let interrupted_thread: InterruptedThread = sleeping_thread.interrupt(reason);
                Ok(InterruptedProcess::from_sleeping(
                    self.state,
                    NonEmptyVecDeque::from(sleeping_threads),
                    NonEmptyVecDeque::new(interrupted_thread),
                    self.zombie_threads.take(),
                ))
            },
            Err(sleeping_threads) => {
                self.sleeping_threads = sleeping_threads;
                Err(self)
            },
        }
    }

    pub fn wakeup_alarm(mut self, now: SystemTime) -> Result<InterruptedProcess, SleepingProcess> {
        let (mut sleeping_threads_to_process, sleeping_thread) = self.sleeping_threads.pop_front();

        // Check if thread has an alarm set.
        if let Some(alarm) = sleeping_thread.alarm() {
            // Check if alarm has expired.
            if now >= alarm {
                let interrupt_thread: InterruptedThread =
                    sleeping_thread.interrupt(InterruptReason::TimedOut);
                let mut interrupted_threads: NonEmptyVecDeque<InterruptedThread> =
                    NonEmptyVecDeque::new(interrupt_thread);
                let mut sleeping_threads: VecDeque<SleepingThread> = VecDeque::new();

                // Process all sleeping threads.
                while let Some(sleeping_thread) = sleeping_threads_to_process.pop_front() {
                    if let Some(alarm) = sleeping_thread.alarm() {
                        if now >= alarm {
                            interrupted_threads
                                .push_back(sleeping_thread.interrupt(InterruptReason::TimedOut));
                        } else {
                            sleeping_threads.push_back(sleeping_thread);
                        }
                    } else {
                        sleeping_threads.push_back(sleeping_thread);
                    }
                }

                return Ok(InterruptedProcess::from_sleeping(
                    self.state,
                    NonEmptyVecDeque::from(sleeping_threads),
                    interrupted_threads,
                    self.zombie_threads.take(),
                ));
            } else {
                // Alarm has not expired, fallthrough.
            }
        } else {
            // Thread does not have an alarm set, fallthrough.
        }

        let mut interrupted_threads: VecDeque<InterruptedThread> = VecDeque::new();
        let mut sleeping_threads: NonEmptyVecDeque<SleepingThread> =
            NonEmptyVecDeque::new(sleeping_thread);

        // Process all sleeping threads.
        while let Some(sleeping_thread) = sleeping_threads_to_process.pop_front() {
            if let Some(alarm) = sleeping_thread.alarm() {
                if now >= alarm {
                    interrupted_threads
                        .push_back(sleeping_thread.interrupt(InterruptReason::TimedOut));
                } else {
                    sleeping_threads.push_back(sleeping_thread);
                }
            } else {
                sleeping_threads.push_back(sleeping_thread);
            }
        }

        if let Some(interrupted_threads) = NonEmptyVecDeque::from(interrupted_threads) {
            Ok(InterruptedProcess::from_sleeping(
                self.state,
                Some(sleeping_threads),
                interrupted_threads,
                self.zombie_threads.take(),
            ))
        } else {
            self.sleeping_threads = sleeping_threads;
            Err(self)
        }
    }

    ///
    /// # Description
    ///
    /// Finds a thread in the target process.
    ///
    /// # Arguments
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If a thread that matches the specified thread identifier is found, then a reference to it is
    /// returned. Otherwise, empty is returned instead.
    ///
    pub fn find_thread(&self, tid: ThreadIdentifier) -> Option<ThreadRef<'_>> {
        if let Some(thread) = self
            .sleeping_threads
            .iter()
            .find(|thread| thread.id() == tid)
        {
            return Some(ThreadRef::Sleeping(thread));
        }

        if let Some(zombie_threads) = &self.zombie_threads {
            if let Some(thread) = zombie_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Zombie(thread));
            }
        }

        None
    }

    ///
    /// # Description
    ///
    /// Finds a thread in the target process.
    ///
    /// # Arguments
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If a thread that matches the specified thread identifier is found, then a mutable reference
    /// to it is returned. Otherwise, empty is returned instead.
    ///
    pub fn find_thread_mut(&mut self, tid: ThreadIdentifier) -> Option<ThreadRefMut<'_>> {
        if let Some(thread) = self
            .sleeping_threads
            .iter_mut()
            .find(|thread| thread.id() == tid)
        {
            return Some(ThreadRefMut::Sleeping(thread));
        }

        if let Some(zombie_threads) = &mut self.zombie_threads {
            if let Some(thread) = zombie_threads.iter_mut().find(|thread| thread.id() == tid) {
                return Some(ThreadRefMut::Zombie(thread));
            }
        }

        None
    }
}

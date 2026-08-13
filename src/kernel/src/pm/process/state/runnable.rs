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
        process::{
            state::{
                interrupted::interrupt,
                signal::SignalControl,
                InterruptedProcess,
                ProcessState,
                RunningProcess,
                ZombieProcess,
            },
            LifecycleTerminationCredit,
        },
        thread::{
            InterruptReason,
            InterruptedThread,
            ReadyThread,
            RunningThread,
            SleepingThread,
            ThreadRef,
            ThreadRefMut,
            ZombieThread,
        },
    },
};
use ::alloc::{
    boxed::Box,
    collections::vec_deque::VecDeque,
};
use ::sys::pm::ProcessIdentifier;
use ::type_safe::NonEmptyVecDeque;
use sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::ThreadIdentifier,
    time::SystemTime,
    ExitStatus,
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
    ///
    /// # Description
    ///
    /// Creates a process whose termination credit will be installed at lifecycle commit.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the new process.
    /// - `parent`: Identifier of the parent process.
    /// - `ready_thread`: Ready main thread of the process.
    /// - `vmem`: Address space of the process.
    ///
    /// # Returns
    ///
    /// A runnable process with no termination credit installed yet.
    ///
    pub(in crate::pm::process) fn new_uncommitted(
        pid: ProcessIdentifier,
        parent: ProcessIdentifier,
        ready_thread: ReadyThread,
        vmem: Vmem,
    ) -> Self {
        Self {
            state: Box::new(ProcessState::new(pid, parent, None, vmem)),
            ready_threads: NonEmptyVecDeque::new(ready_thread),
            interrupted_threads: None,
            sleeping_threads: None,
            zombie_threads: None,
        }
    }

    ///
    /// # Description
    ///
    /// Installs the capacity credit reserved for this process's termination record.
    ///
    /// # Parameters
    ///
    /// - `credit`: The capacity credit to install for the process's future termination record.
    ///
    pub(in crate::pm::process) fn install_termination_credit(
        &mut self,
        credit: LifecycleTerminationCredit,
    ) {
        self.state.install_termination_credit(credit);
    }

    pub(in crate::pm::process) fn new_kernel(
        pid: ProcessIdentifier,
        parent: ProcessIdentifier,
        ready_thread: ReadyThread,
        vmem: Vmem,
    ) -> Self {
        Self {
            state: Box::new(ProcessState::new(pid, parent, None, vmem)),
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

    ///
    /// # Description
    ///
    /// Installs the per-process signal control block into this process.
    ///
    /// Used by `fork()` to install the dispositions and restorer inherited from the parent into
    /// the freshly created child before it is enqueued onto the ready list.
    ///
    /// # Parameters
    ///
    /// - `signals`: The signal control block to install.
    ///
    pub fn set_signals(&mut self, signals: SignalControl) {
        self.state.set_signals(signals);
    }

    ///
    /// # Description
    ///
    /// Finds the next thread to run in the target process.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing:
    /// - The process in the running state.
    /// - The reason why the thread was interrupted, if any.
    /// - A pointer to the context information of the next thread to run.
    /// - An optional pointer to the thread data area of the next thread to run.
    ///
    pub fn run(
        mut self,
    ) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation, Option<VirtualAddress>)
    {
        // Select and remove the thread with the earliest admission time.
        // NOTE: uses `remove_min_by_key()` to operate directly on `NonEmptyVecDeque`, avoiding
        // the heap allocation that a `VecDeque` conversion would require.
        let (ready_threads, next_thread) =
            self.ready_threads.remove_min_by_key(|t| t.admission_time());

        let (running_thread, interrupt_reason, next_context, user_tda): (
            RunningThread,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = next_thread.run();
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
            user_tda,
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
            // Use pending exit status if set (from a prior exit() call), otherwise use
            // Interrupted error code.
            let final_status: ExitStatus = self
                .state
                .take_pending_exit_status()
                .unwrap_or_else(|| ErrorCode::Interrupted.into());
            Err(ZombieProcess::new(self.state, zombie_threads, final_status))
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

    pub fn earliest_admission_time(&self) -> SystemTime {
        self.ready_threads
            .iter()
            .map(|thread| thread.admission_time())
            .min()
            .unwrap_or(clock::now())
    }

    ///
    /// # Description
    ///
    /// Wakes every sleeping thread of this process whose alarm has expired, moving it into the
    /// ready set with an [`InterruptReason::TimedOut`] reason.
    ///
    /// This services per-thread timer alarms while the process remains runnable because it still
    /// has other ready threads. Without it, a sleeping thread parked inside a runnable process
    /// would never have its alarm serviced until the whole process quiesced, allowing a CPU-bound
    /// sibling thread to starve another thread's timed wait.
    ///
    /// # Parameters
    ///
    /// - `now`: The current system time.
    ///
    pub fn wakeup_expired_alarms(&mut self, now: SystemTime) {
        // Take the sleeping-thread list, returning early when there is nothing to service.
        let mut sleeping_threads: VecDeque<SleepingThread> = match self.sleeping_threads.take() {
            Some(sleeping_threads) => VecDeque::from(sleeping_threads),
            None => return,
        };

        // Visit each currently-sleeping thread exactly once, reusing the existing allocation.
        // Threads whose alarm has expired are woken and moved to the ready set; the rest are
        // rotated to the back of the same deque so they stay asleep without being revisited.
        // The counter bounds the loop to the initial length, so re-queued threads are skipped.
        let mut remaining: usize = sleeping_threads.len();
        while remaining > 0 {
            remaining -= 1;
            let Some(thread) = sleeping_threads.pop_front() else {
                break;
            };
            match thread.alarm() {
                Some(alarm) if now >= alarm => {
                    // Wake the thread carrying the TimedOut reason so the blocking kernel call
                    // returns Interrupted(TimedOut), mirroring the suspended-process alarm path.
                    let ready_thread: ReadyThread =
                        thread.interrupt(InterruptReason::TimedOut).resume();
                    self.ready_threads.push_back(ready_thread);
                },
                _ => sleeping_threads.push_back(thread),
            }
        }

        // Restore the threads that are still sleeping (None when all of them were woken).
        self.sleeping_threads = NonEmptyVecDeque::from(sleeping_threads);
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
        // Search in the list of ready threads.
        if let Some(thread) = self.ready_threads.iter().find(|thread| thread.id() == tid) {
            return Some(ThreadRef::Ready(thread));
        }

        // Search in the list of interrupted threads.
        if let Some(interrupted_threads) = &self.interrupted_threads {
            if let Some(thread) = interrupted_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Interrupted(thread));
            }
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &self.sleeping_threads {
            if let Some(thread) = sleeping_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Sleeping(thread));
            }
        }

        // Search in the list of zombie threads.
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
        // Search in the list of ready threads.
        if let Some(thread) = self
            .ready_threads
            .iter_mut()
            .find(|thread| thread.id() == tid)
        {
            return Some(ThreadRefMut::Ready(thread));
        }

        // Search in the list of interrupted threads.
        if let Some(interrupted_threads) = &mut self.interrupted_threads {
            if let Some(thread) = interrupted_threads
                .iter_mut()
                .find(|thread| thread.id() == tid)
            {
                return Some(ThreadRefMut::Interrupted(thread));
            }
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &mut self.sleeping_threads {
            if let Some(thread) = sleeping_threads
                .iter_mut()
                .find(|thread| thread.id() == tid)
            {
                return Some(ThreadRefMut::Sleeping(thread));
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &mut self.zombie_threads {
            if let Some(thread) = zombie_threads.iter_mut().find(|thread| thread.id() == tid) {
                return Some(ThreadRefMut::Zombie(thread));
            }
        }

        None
    }
}

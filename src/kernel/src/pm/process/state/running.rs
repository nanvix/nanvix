// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::Vmem,
    pm::{
        process::state::{
            interrupted::interrupt,
            sleeping::SleepingProcess,
            InterruptedProcess,
            ProcessState,
            RunnableProcess,
            ZombieProcessTransition,
        },
        sync::condvar::Condvar,
        thread::{
            InterruptedThread,
            PendingThreadTermination,
            ReadyThread,
            RunningThread,
            SleepingThread,
            ThreadRef,
            ThreadRefMut,
            ZombieThread,
        },
    },
};
use ::alloc::boxed::Box;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::VirtualAddress,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that is running.
///
#[derive(Debug)]
pub struct RunningProcess {
    /// Process state.
    state: Box<ProcessState>,
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
        state: Box<ProcessState>,
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

    ///
    /// # Description
    ///
    /// Returns a mutable reference to the running thread.
    ///
    /// # Returns
    ///
    /// A mutable reference to the running thread.
    ///
    pub fn running_mut(&mut self) -> &mut RunningThread {
        &mut self.running
    }

    ///
    /// # Description
    ///
    /// Returns whether this process has exactly one thread (the running thread), i.e. no ready,
    /// interrupted, sleeping, or zombie threads.
    ///
    /// # Returns
    ///
    /// `true` if the running thread is the only thread in the process, otherwise `false`.
    ///
    pub fn is_single_threaded(&self) -> bool {
        self.ready.is_none()
            && self.interrupted_threads.is_none()
            && self.sleeping_threads.is_none()
            && self.zombie.is_none()
    }

    ///
    /// # Description
    ///
    /// Replaces the running image of this process with a freshly built one, as part of `execv()`.
    /// The process identity and capabilities are preserved; only the address space and the running
    /// thread are swapped.
    ///
    /// The newly built thread is transitioned to the running state, and the outgoing thread is
    /// retired as a zombie that retains its kernel stack and execution context until the deferred
    /// reap that runs after the context switch into the new image. The outgoing address space is
    /// returned to the caller for deferred reclamation, because it is still the active page
    /// directory until the switch completes.
    ///
    /// This must only be called on a single-threaded process (see [`Self::is_single_threaded`]).
    ///
    /// # Parameters
    ///
    /// - `new_vmem`: Address space of the new image.
    /// - `new_thread`: Ready main thread of the new image.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - The outgoing address space, for deferred reclamation.
    /// - The outgoing thread as a zombie, for deferred kernel-stack reaping.
    /// - The pending termination record for the outgoing thread.
    /// - A pointer to the outgoing thread's context (the "from" side of the switch).
    /// - A pointer to the new thread's context (the "to" side of the switch).
    /// - The optional thread data area of the new thread.
    ///
    /// # Panics
    ///
    /// This function panics if the outgoing thread does not own a thread termination credit.
    ///
    pub(in crate::pm) fn replace_image(
        &mut self,
        new_vmem: Vmem,
        new_thread: ReadyThread,
    ) -> (
        Vmem,
        ZombieThread,
        PendingThreadTermination,
        *mut ContextInformation,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) {
        // Transition the freshly built thread into the running state; its context is the "to"
        // side of the upcoming switch.
        let (new_running, _reason, to_ctx, user_tda) = new_thread.run();

        // Install the new running thread and extract the outgoing one.
        let old_thread: RunningThread = core::mem::replace(&mut self.running, new_running);

        // Retire the outgoing thread. Its kernel stack and context survive inside the returned
        // zombie until the deferred reap that runs after the switch; its user-stack handle is
        // dropped so the harvest does not touch the new address space.
        let pid: ProcessIdentifier = self.state.pid();
        let (transition, from_ctx) = old_thread.exit_for_exec(pid);
        let (old_zombie, pending) = transition.into_parts();

        // Swap in the new address space, returning the old one for deferred reclamation.
        let old_vmem: Vmem = self.state.replace_vmem(new_vmem);

        // Reset signal dispositions for the new image: caught handlers point at code in the old
        // image, so they are reset to the default (SIG_IGN/SIG_DFL are preserved), the pending set
        // is cleared, and the restorer is dropped so the new image re-registers it at startup.
        self.state.signals_mut().reset_for_exec();

        (old_vmem, old_zombie, pending, from_ctx, to_ctx, user_tda)
    }

    pub fn schedule(mut self) -> (RunnableProcess, *mut ContextInformation) {
        let running_thread = self.running;
        let (ready_thread, ctx) = running_thread.schedule();

        let ready_threads = match self.ready.take() {
            Some(mut ready_threads) => {
                ready_threads.push_back(ready_thread);
                ready_threads
            },
            None => NonEmptyVecDeque::new(ready_thread),
        };

        (
            RunnableProcess::from_state(
                self.state,
                ready_threads,
                self.interrupted_threads.take(),
                self.sleeping_threads.take(),
                self.zombie.take(),
            ),
            ctx,
        )
    }

    pub fn sleep(
        mut self,
        alarm: Option<SystemTime>,
    ) -> Result<
        (RunnableProcess, *mut ContextInformation),
        (SleepingProcess, *mut ContextInformation),
    > {
        let (sleeping_thread, ctx) = self.running.sleep(alarm);

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
                RunnableProcess::from_state(
                    self.state,
                    ready_threads,
                    self.interrupted_threads.take(),
                    Some(sleeping_threads),
                    self.zombie.take(),
                ),
                ctx,
            ));
        }

        // Check if there are interrupted threads.
        if let Some(interrupted_threads) = self.interrupted_threads.take() {
            let interrupted_process: InterruptedProcess = InterruptedProcess::from_sleeping(
                self.state,
                Some(sleeping_threads),
                interrupted_threads,
                self.zombie.take(),
            );

            return Ok((interrupted_process.resume(), ctx));
        }

        Err((SleepingProcess::new(self.state, sleeping_threads, self.zombie.take()), ctx))
    }

    ///
    /// # Description
    ///
    /// Terminates the running process with the specified status. The running thread and every
    /// ready thread transition immediately to zombie state, while sleeping and interrupted threads
    /// are marked for termination. Each immediate thread transition is reported through
    /// `on_thread_termination`.
    ///
    /// # Parameters
    ///
    /// - `status`: Exit status of the process and its running thread.
    /// - `on_thread_termination`: Callback that commits each pending thread-termination record.
    ///
    /// # Returns
    ///
    /// `Ok((process, context))` if interrupted threads must resume before termination completes, or
    /// `Err((transition, context))` if the process immediately becomes a zombie.
    ///
    /// # Panics
    ///
    /// This function panics if an immediately terminated thread lacks its thread termination credit,
    /// or if an immediate process transition lacks its process termination credit.
    ///
    pub(in crate::pm) fn exit(
        mut self,
        status: ExitStatus,
        on_thread_termination: &mut impl FnMut(PendingThreadTermination),
    ) -> Result<
        (RunnableProcess, *mut ContextInformation),
        (ZombieProcessTransition, *mut ContextInformation),
    > {
        // Save the exit status before terminating any threads. This ensures that the intended
        // exit code from the first exit() call is preserved, even if subsequent thread cleanup
        // triggers additional exit() calls with different status values (e.g., ESRCH from
        // detached thread teardown). The set_pending_exit_status() method is a no-op when a
        // pending status is already set, so only the first caller's status is retained.
        self.state.set_pending_exit_status(status);

        let pid: ProcessIdentifier = self.state.pid();
        let (transition, ctx) = self.running.exit(pid, status);
        let (zombie_thread, pending) = transition.into_parts();
        on_thread_termination(pending);
        let mut zombie_threads: NonEmptyVecDeque<ZombieThread> = match self.zombie.take() {
            Some(mut zombie_threads) => {
                zombie_threads.push_back(zombie_thread);
                zombie_threads
            },
            None => NonEmptyVecDeque::new(zombie_thread),
        };

        // Terminate all ready threads.
        if let Some(ready_threads) = self.ready.take() {
            let more_zombie_threads = NonEmptyVecDeque::map(ready_threads, |thread| {
                let (zombie, pending) = thread.terminate(pid).into_parts();
                on_thread_termination(pending);
                zombie
            });
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
            let interrupted_process: InterruptedProcess = InterruptedProcess::from_sleeping(
                self.state,
                None, // Sleeping threads were converted to interrupted above.
                interrupted_threads,
                Some(zombie_threads),
            );

            Ok((interrupted_process.resume(), ctx))
        } else {
            // Use pending exit status (from the first exit() call). The unwrap_or fallback
            // should never be reached because set_pending_exit_status is called above, but is
            // kept as a defensive measure.
            let final_status: ExitStatus = self.state.take_pending_exit_status().unwrap_or(status);
            Err((ZombieProcessTransition::new(self.state, zombie_threads, final_status), ctx))
        }
    }

    ///
    /// # Description
    ///
    /// Exits the calling thread.
    ///
    /// # Parameters
    ///
    /// - `status`: Exit status.
    /// - `on_thread_termination`: Callback that commits the pending thread-termination record.
    ///
    /// # Returns
    ///
    /// A tuple containing the process-state transition and an optional detached zombie for deferred
    /// reaping is returned. The transition contains a runnable process when ready or interrupted
    /// threads remain, a sleeping process when only sleeping threads remain, or a zombie-process
    /// transition when no live threads remain.
    ///
    /// # Panics
    ///
    /// This function panics if the exiting thread does not own a thread termination credit, or if a
    /// final process transition lacks its process termination credit.
    ///
    #[allow(clippy::type_complexity)]
    pub(in crate::pm) fn exit_thread(
        self,
        status: ExitStatus,
        on_thread_termination: &mut impl FnMut(PendingThreadTermination),
    ) -> (
        Result<
            (Condvar, RunnableProcess, *mut ContextInformation),
            Result<
                (Condvar, SleepingProcess, *mut ContextInformation),
                (Condvar, ZombieProcessTransition, *mut ContextInformation),
            >,
        >,
        Option<ZombieThread>, // Detached zombie that must be reaped after the context switch.
    ) {
        let is_detached: bool = self.running.is_detached();
        let join_cond: Condvar = self.running.join_cond();

        // Extract all fields from self before running.exit() consumes self.running.
        let mut state: Box<ProcessState> = self.state;
        let ready: Option<NonEmptyVecDeque<ReadyThread>> = self.ready;
        let interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>> =
            self.interrupted_threads;
        let sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>> = self.sleeping_threads;
        let existing_zombies: Option<NonEmptyVecDeque<ZombieThread>> = self.zombie;

        let pid: ProcessIdentifier = state.pid();
        let (transition, ctx) = self.running.exit(pid, status);
        let (zombie_thread, pending) = transition.into_parts();
        on_thread_termination(pending);

        // Determine whether other threads remain. If so, a detached zombie must NOT be dropped
        // here — it owns the ContextInformation that `ctx` points to, and the context switch
        // will write through that pointer. Instead, return it for deferred reaping after the
        // context switch completes.
        let has_other_threads: bool =
            ready.is_some() || interrupted_threads.is_some() || sleeping_threads.is_some();

        let (zombie_threads, deferred_zombie): (
            Option<NonEmptyVecDeque<ZombieThread>>,
            Option<ZombieThread>,
        ) = if is_detached && has_other_threads {
            // Return the zombie for deferred reaping. Do NOT drop it here — the context
            // switch still needs the ContextInformation that lives inside the zombie's
            // ThreadState.
            (existing_zombies, Some(zombie_thread))
        } else {
            (
                Some(match existing_zombies {
                    Some(mut zombie_threads) => {
                        zombie_threads.push_back(zombie_thread);
                        zombie_threads
                    },
                    None => NonEmptyVecDeque::new(zombie_thread),
                }),
                None,
            )
        };

        if let Some(ready_threads) = ready {
            (
                Ok((
                    join_cond,
                    RunnableProcess::from_state(
                        state,
                        ready_threads,
                        interrupted_threads,
                        sleeping_threads,
                        zombie_threads,
                    ),
                    ctx,
                )),
                deferred_zombie,
            )
        } else if let Some(interrupted_threads) = interrupted_threads {
            let interrupted_process: InterruptedProcess = InterruptedProcess::from_sleeping(
                state,
                sleeping_threads,
                interrupted_threads,
                zombie_threads,
            );

            (Ok((join_cond, interrupted_process.resume(), ctx)), deferred_zombie)
        } else if let Some(sleeping_threads) = sleeping_threads {
            (
                Err(Ok((
                    join_cond,
                    SleepingProcess::new(state, sleeping_threads, zombie_threads),
                    ctx,
                ))),
                deferred_zombie,
            )
        } else {
            match zombie_threads {
                Some(zombie_threads) => {
                    // Use pending exit status if set (from a prior exit() call), otherwise use
                    // current thread's status.
                    let final_status: ExitStatus =
                        state.take_pending_exit_status().unwrap_or(status);
                    (
                        Err(Err((
                            join_cond,
                            ZombieProcessTransition::new(state, zombie_threads, final_status),
                            ctx,
                        ))),
                        deferred_zombie,
                    )
                },
                // Unreachable: zombie_threads is None only when is_detached && has_other_threads,
                // which guarantees at least one of the ready/interrupted/sleeping branches above
                // would match.
                None => unreachable!("no zombie threads and no other threads"),
            }
        }
    }

    pub fn get_tid(&self) -> ThreadIdentifier {
        self.running.id()
    }

    ///
    /// # Description
    ///
    /// Checks the guard watermark of the running thread's kernel stack for corruption.
    ///
    /// # Returns
    ///
    /// Upon success (watermark intact or no kernel stack), `Ok(())` is returned. Upon failure
    /// (watermark corrupted), an error is returned.
    ///
    #[inline]
    pub fn check_guard_watermark(&self) -> Result<(), Error> {
        self.running.check_guard_watermark()
    }

    ///
    /// # Description
    ///
    /// Returns the guard threshold of the running thread's kernel stack, if any.
    ///
    /// # Returns
    ///
    /// The guard threshold value, or `None` if the running thread has no kernel stack.
    ///
    #[cfg(feature = "exception-stack-guard")]
    #[inline]
    pub fn guard_threshold(&self) -> Option<u32> {
        self.running.guard_threshold()
    }

    ///
    /// # Description
    ///
    /// Adds a ready thread to the running process.
    ///
    /// # Parameters
    ///
    /// - `ready_thread`: Thread to add.
    ///
    pub fn add_thread(&mut self, ready_thread: ReadyThread) {
        trace!("self.pid={:?}, ready_thread={:?}", self.state.pid, ready_thread);
        match self.ready.as_mut() {
            Some(ready_threads) => {
                ready_threads.push_back(ready_thread);
            },
            None => {
                self.ready = Some(NonEmptyVecDeque::new(ready_thread));
            },
        }
    }

    pub fn wakeup(mut self, tid: ThreadIdentifier) -> Result<RunningProcess, RunningProcess> {
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            match sleeping_threads.remove_if(|thread| thread.id() == tid) {
                Ok((sleeping_threads, sleeping_thread)) => {
                    let ready_thread: ReadyThread = sleeping_thread.wakeup();

                    let ready_threads = match self.ready.take() {
                        Some(mut ready_threads) => {
                            ready_threads.push_back(ready_thread);
                            ready_threads
                        },
                        None => NonEmptyVecDeque::new(ready_thread),
                    };

                    Ok(Self::new(
                        self.state,
                        self.running,
                        Some(ready_threads),
                        self.interrupted_threads.take(),
                        NonEmptyVecDeque::from(sleeping_threads),
                        self.zombie.take(),
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

    #[allow(clippy::type_complexity)]
    pub fn try_join_thread(
        &mut self,
        tid: ThreadIdentifier,
    ) -> Result<ZombieThread, Result<Condvar, Error>> {
        // Check if the thread is the running thread.
        if self.running.id() == tid {
            let reason: &str = "thread is running";
            return Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)));
        }

        // Search for thread in zombie threads.
        if let Some(zombie_threads) = &self.zombie {
            for zombie_thread in zombie_threads.iter() {
                if zombie_thread.id() == tid && zombie_thread.is_detached() {
                    let reason: &str = "cannot join a detached thread";
                    error!("{reason} (tid={tid:?})");
                    return Err(Err(Error::new(ErrorCode::InvalidArgument, reason)));
                }
            }
        }
        if let Some(zombie_threads) = self.zombie.take() {
            match zombie_threads.remove_if(|thread| thread.id() == tid) {
                Ok((zombie_threads, zombie_thread)) => {
                    self.zombie = NonEmptyVecDeque::from(zombie_threads);
                    return Ok(zombie_thread);
                },
                Err(zombie_threads) => {
                    self.zombie = Some(zombie_threads);
                },
            }
        }

        // Search for thread in ready threads.
        if let Some(ready_threads) = &mut self.ready {
            for ready_thread in ready_threads.iter() {
                if ready_thread.id() == tid {
                    if ready_thread.is_detached() {
                        let reason: &str = "cannot join a detached thread";
                        error!("{reason} (tid={tid:?})");
                        return Err(Err(Error::new(ErrorCode::InvalidArgument, reason)));
                    }
                    let join_cond: Condvar = ready_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        // Search for thread in sleeping threads.
        if let Some(sleeping_threads) = &mut self.sleeping_threads {
            for sleeping_thread in sleeping_threads.iter() {
                if sleeping_thread.id() == tid {
                    if sleeping_thread.is_detached() {
                        let reason: &str = "cannot join a detached thread";
                        error!("{reason} (tid={tid:?})");
                        return Err(Err(Error::new(ErrorCode::InvalidArgument, reason)));
                    }
                    let join_cond: Condvar = sleeping_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        // Search for thread in interrupted threads.
        if let Some(interrupted_threads) = &mut self.interrupted_threads {
            for interrupted_thread in interrupted_threads.iter() {
                if interrupted_thread.id() == tid {
                    if interrupted_thread.is_detached() {
                        let reason: &str = "cannot join a detached thread";
                        error!("{reason} (tid={tid:?})");
                        return Err(Err(Error::new(ErrorCode::InvalidArgument, reason)));
                    }
                    let join_cond: Condvar = interrupted_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        let reason: &str = "thread not found";
        error!("{:?} (state={:?})", reason, self.state());
        Err(Err(Error::new(ErrorCode::NoSuchProcess, reason)))
    }

    ///
    /// # Description
    ///
    /// Detaches a thread so that it will be auto-harvested when it exits. If the thread is already
    /// a zombie, it is removed from the zombie queue and returned for immediate harvesting.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread to detach.
    ///
    /// # Returns
    ///
    /// On success, returns `Ok(None)` if the thread was marked detached, or `Ok(Some(zombie))` if
    /// the thread was already a zombie and should be harvested. On failure, returns an error.
    ///
    pub fn detach_thread(&mut self, tid: ThreadIdentifier) -> Result<Option<ZombieThread>, Error> {
        // Detach the running (calling) thread.
        if self.running.id() == tid {
            if self.running.is_detached() {
                let reason: &str = "thread is already detached";
                error!("{reason} (tid={tid:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            self.running.set_detached();
            return Ok(None);
        }

        // Search in zombie threads — if found, remove and return for immediate harvest.
        if let Some(zombie_threads) = self.zombie.take() {
            match zombie_threads.remove_if(|thread| thread.id() == tid) {
                Ok((zombie_threads, zombie_thread)) => {
                    self.zombie = NonEmptyVecDeque::from(zombie_threads);
                    return Ok(Some(zombie_thread));
                },
                Err(zombie_threads) => {
                    self.zombie = Some(zombie_threads);
                },
            }
        }

        // Search in ready threads.
        if let Some(ready_threads) = &mut self.ready {
            for ready_thread in ready_threads.iter_mut() {
                if ready_thread.id() == tid {
                    if ready_thread.is_detached() {
                        let reason: &str = "thread is already detached";
                        error!("{reason} (tid={tid:?})");
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    }
                    ready_thread.set_detached();
                    return Ok(None);
                }
            }
        }

        // Search in sleeping threads.
        if let Some(sleeping_threads) = &mut self.sleeping_threads {
            for sleeping_thread in sleeping_threads.iter_mut() {
                if sleeping_thread.id() == tid {
                    if sleeping_thread.is_detached() {
                        let reason: &str = "thread is already detached";
                        error!("{reason} (tid={tid:?})");
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    }
                    sleeping_thread.set_detached();
                    return Ok(None);
                }
            }
        }

        // Search in interrupted threads.
        if let Some(interrupted_threads) = &mut self.interrupted_threads {
            for interrupted_thread in interrupted_threads.iter_mut() {
                if interrupted_thread.id() == tid {
                    if interrupted_thread.is_detached() {
                        let reason: &str = "thread is already detached";
                        error!("{reason} (tid={tid:?})");
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    }
                    interrupted_thread.set_detached();
                    return Ok(None);
                }
            }
        }

        let reason: &str = "thread not found";
        error!("{reason} (tid={tid:?})");
        Err(Error::new(ErrorCode::NoSuchProcess, reason))
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
        // Check if the running thread matches.
        if self.running.id() == tid {
            return Some(ThreadRef::Running(&self.running));
        }

        // Search in the list of ready threads.
        if let Some(ready_threads) = &self.ready {
            if let Some(thread) = ready_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Ready(thread));
            }
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
        if let Some(zombie_threads) = &self.zombie {
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
        // Check if the running thread matches.
        if self.running.id() == tid {
            return Some(ThreadRefMut::Running(&mut self.running));
        }

        // Search in the list of ready threads.
        if let Some(ready_threads) = &mut self.ready {
            if let Some(thread) = ready_threads.iter_mut().find(|thread| thread.id() == tid) {
                return Some(ThreadRefMut::Ready(thread));
            }
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
        if let Some(zombie_threads) = &mut self.zombie {
            if let Some(thread) = zombie_threads.iter_mut().find(|thread| thread.id() == tid) {
                return Some(ThreadRefMut::Zombie(thread));
            }
        }

        None
    }
}

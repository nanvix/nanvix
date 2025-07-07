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
            InterruptedProcess,
            ProcessState,
            RunnableProcess,
            ZombieProcess,
        },
        sync::condvar::Condvar,
        thread::{
            InterruptedThread,
            ReadyThread,
            RunningThread,
            SleepingThread,
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
    pm::ThreadIdentifier,
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

    pub fn exit(
        mut self,
        status: ExitStatus,
    ) -> Result<(RunnableProcess, *mut ContextInformation), (ZombieProcess, *mut ContextInformation)>
    {
        let (zombie_thread, ctx) = self.running.exit(status);
        let mut zombie_threads: NonEmptyVecDeque<ZombieThread> = match self.zombie.take() {
            Some(mut zombie_threads) => {
                zombie_threads.push_back(zombie_thread);
                zombie_threads
            },
            None => NonEmptyVecDeque::new(zombie_thread),
        };

        // Terminate all ready threads.
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
            let interrupted_process: InterruptedProcess = InterruptedProcess::from_sleeping(
                self.state,
                self.sleeping_threads.take(),
                interrupted_threads,
                self.zombie.take(),
            );

            Ok((interrupted_process.resume(), ctx))
        } else {
            Err((ZombieProcess::new(self.state, zombie_threads, status), ctx))
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
    ///
    /// # Returns
    ///
    /// If the process becomes a runnable process, a tuple containing the runnable process and the
    /// context information of the running thread is returned. If the process becomes a sleeping
    /// process, a tuple containing the sleeping process and the context information of the running
    /// thread is returned. If the process becomes a zombie process, a tuple containing the zombie
    /// process and the context information of the running thread is returned.
    ///
    #[allow(clippy::type_complexity)]
    pub fn exit_thread(
        mut self,
        status: ExitStatus,
    ) -> Result<
        (Condvar, RunnableProcess, *mut ContextInformation),
        Result<
            (Condvar, SleepingProcess, *mut ContextInformation),
            (Condvar, ZombieProcess, *mut ContextInformation),
        >,
    > {
        let join_cond: Condvar = self.running.join_cond();

        let (zombie_thread, ctx) = self.running.exit(status);
        let zombie_threads: NonEmptyVecDeque<ZombieThread> = match self.zombie.take() {
            Some(mut zombie_threads) => {
                zombie_threads.push_back(zombie_thread);
                zombie_threads
            },
            None => NonEmptyVecDeque::new(zombie_thread),
        };

        if let Some(ready_threads) = self.ready.take() {
            Ok((
                join_cond,
                RunnableProcess::from_state(
                    self.state,
                    ready_threads,
                    self.interrupted_threads.take(),
                    self.sleeping_threads.take(),
                    Some(zombie_threads),
                ),
                ctx,
            ))
        } else if let Some(interrupted_threads) = self.interrupted_threads.take() {
            let interrupted_process: InterruptedProcess = InterruptedProcess::from_sleeping(
                self.state,
                self.sleeping_threads.take(),
                interrupted_threads,
                self.zombie.take(),
            );

            Ok((join_cond, interrupted_process.resume(), ctx))
        } else if let Some(sleeping_threads) = self.sleeping_threads.take() {
            Err(Ok((
                join_cond,
                SleepingProcess::new(self.state, sleeping_threads, Some(zombie_threads)),
                ctx,
            )))
        } else {
            Err(Err((join_cond, ZombieProcess::new(self.state, zombie_threads, status), ctx)))
        }
    }

    pub fn get_tid(&self) -> ThreadIdentifier {
        self.running.id()
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
        if let Some(zombie_threads) = self.zombie.take() {
            match zombie_threads.remove_if(|thread| thread.tid() == tid) {
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
                if ready_thread.tid() == tid {
                    let join_cond: Condvar = ready_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        // Search for thread in sleeping threads.
        if let Some(sleeping_threads) = &mut self.sleeping_threads {
            for sleeping_thread in sleeping_threads.iter() {
                if sleeping_thread.id() == tid {
                    let join_cond: Condvar = sleeping_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        // Search for thread in interrupted threads.
        if let Some(interrupted_threads) = &mut self.interrupted_threads {
            for interrupted_thread in interrupted_threads.iter() {
                if interrupted_thread.tid() == tid {
                    let join_cond: Condvar = interrupted_thread.join_cond();
                    return Err(Ok(join_cond));
                }
            }
        }

        let reason: &str = "thread not found";
        error!("join_thread(): {:?} (state={:?})", reason, self.state());
        Err(Err(Error::new(ErrorCode::NoSuchProcess, reason)))
    }

    pub fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Check if the running thread matches.
        if self.running.id() == tid {
            return true;
        }

        // Search in the list of ready threads.
        if let Some(ready_threads) = &self.ready {
            if ready_threads.iter().any(|thread| thread.tid() == tid) {
                return true;
            }
        }

        // Search in the list of interrupted threads.
        if let Some(interrupted_threads) = &self.interrupted_threads {
            if interrupted_threads.iter().any(|thread| thread.tid() == tid) {
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
        if let Some(zombie_threads) = &self.zombie {
            if zombie_threads.iter().any(|thread| thread.tid() == tid) {
                return true;
            }
        }

        false
    }
}

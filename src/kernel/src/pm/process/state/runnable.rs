// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::{
        ustack::UserStackAllocator,
        Vmem,
    },
    pm::{
        process::state::{
            interrupted::interrupt,
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
use sys::{
    error::ErrorCode,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Runnable Process
//==================================================================================================

pub struct RunnableProcessWithReadyThread {
    state: Box<ProcessState>,
    ready_threads: NonEmptyVecDeque<ReadyThread>,
    interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl RunnableProcessWithReadyThread {
    fn new(
        pid: ProcessIdentifier,
        ready_thread: ReadyThread,
        vmem: Vmem,
        user_stack_allocator: Option<UserStackAllocator>,
    ) -> Self {
        Self {
            state: Box::new(ProcessState::new(pid, vmem, user_stack_allocator)),
            ready_threads: NonEmptyVecDeque::new(ready_thread),
            interrupted_threads: None,
            sleeping_threads: None,
            zombie_threads: None,
        }
    }

    fn from_state(
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

    fn state(&self) -> &ProcessState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    fn run(mut self) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation) {
        let (ready_threads, next_thread) = self.ready_threads.pop_front();
        let (running_thread, next_context) = next_thread.resume();
        (
            RunningProcess::new(
                self.state,
                running_thread,
                NonEmptyVecDeque::from(ready_threads),
                self.interrupted_threads.take(),
                self.sleeping_threads.take(),
                self.zombie_threads.take(),
            ),
            None,
            next_context,
        )
    }

    fn terminate(mut self) -> Result<RunnableProcessWithInterruptedThreads, ZombieProcess> {
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
            Ok(RunnableProcessWithInterruptedThreads::new(
                self.state,
                None,
                interrupted_threads,
                None,
                Some(zombie_threads),
            ))
        } else {
            Err(ZombieProcess::new(self.state, zombie_threads, ErrorCode::Interrupted.into()))
        }
    }

    fn wakeup(mut self, tid: ThreadIdentifier) -> Result<Self, Self> {
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

    fn add_thread(mut self, ready_thread: ReadyThread) -> Self {
        trace!("add_thread(): self.pid={:?}, ready_thread={:?}", self.state.pid, ready_thread);
        self.ready_threads.push_back(ready_thread);

        self
    }

    fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Search in the list of ready threads.
        if self.ready_threads.iter().any(|thread| thread.tid() == tid) {
            return true;
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
        if let Some(zombie_threads) = &self.zombie_threads {
            if zombie_threads.iter().any(|thread| thread.tid() == tid) {
                return true;
            }
        }

        false
    }
}

pub struct RunnableProcessWithInterruptedThreads {
    state: Box<ProcessState>,
    ready_threads: Option<NonEmptyVecDeque<ReadyThread>>,
    interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl RunnableProcessWithInterruptedThreads {
    fn new(
        state: Box<ProcessState>,
        ready_threads: Option<NonEmptyVecDeque<ReadyThread>>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
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

    fn state(&self) -> &ProcessState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    fn run(mut self) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation) {
        let (interrupted_threads, next_thread) = self.interrupted_threads.pop_front();
        let (running_thread, reason, next_context) = next_thread.resume();
        (
            RunningProcess::new(
                self.state,
                running_thread,
                self.ready_threads.take(),
                NonEmptyVecDeque::from(interrupted_threads),
                self.sleeping_threads.take(),
                self.zombie_threads.take(),
            ),
            Some(reason),
            next_context,
        )
    }

    fn interrupt(thread: SleepingThread) -> InterruptedThread {
        thread.interrupt(InterruptReason::Killed)
    }

    fn terminate(mut self) -> Result<Self, ZombieProcess> {
        // Terminate all ready threads.
        let mut more_zombie_threads: Option<NonEmptyVecDeque<ZombieThread>> = self
            .ready_threads
            .take()
            .map(|ready_threads| NonEmptyVecDeque::map(ready_threads, ReadyThread::terminate));
        // Collect zombie threads.
        let zombie_threads: Option<NonEmptyVecDeque<ZombieThread>> =
            match self.zombie_threads.take() {
                Some(zombie_threads) => match more_zombie_threads.take() {
                    Some(mut more_zombie_threads) => {
                        more_zombie_threads.append(zombie_threads);
                        Some(more_zombie_threads)
                    },
                    None => Some(zombie_threads),
                },
                None => more_zombie_threads.take(),
            };

        // Collect interrupted threads.
        let mut interrupted_threads: NonEmptyVecDeque<InterruptedThread> = self.interrupted_threads;

        // Terminate all sleeping threads.
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            let more_interrupted_threads = NonEmptyVecDeque::map(sleeping_threads, Self::interrupt);
            interrupted_threads.append(more_interrupted_threads);
        }

        Ok(Self::new(self.state, None, interrupted_threads, None, zombie_threads))
    }

    fn wakeup(
        mut self,
        tid: ThreadIdentifier,
    ) -> Result<RunnableProcessWithReadyAndInteruptThread, Self> {
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            match sleeping_threads.remove_if(|thread| thread.id() == tid) {
                Ok((sleeping_threads, sleeping_thread)) => {
                    let ready_thread: ReadyThread = sleeping_thread.wakeup();
                    let ready_threads = match self.ready_threads.take() {
                        Some(mut ready_threads) => {
                            ready_threads.push_back(ready_thread);
                            ready_threads
                        },
                        None => NonEmptyVecDeque::new(ready_thread),
                    };
                    Ok(RunnableProcessWithReadyAndInteruptThread::new(
                        self.state,
                        ready_threads,
                        self.interrupted_threads,
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

    fn add_thread(
        mut self,
        ready_thread: ReadyThread,
    ) -> RunnableProcessWithReadyAndInteruptThread {
        trace!("add_thread(): self.pid={:?}, ready_thread={:?}", self.state.pid, ready_thread);
        let ready_threads = match self.ready_threads.take() {
            Some(mut ready_threads) => {
                ready_threads.push_back(ready_thread);
                ready_threads
            },
            None => NonEmptyVecDeque::new(ready_thread),
        };
        RunnableProcessWithReadyAndInteruptThread::new(
            self.state,
            ready_threads,
            self.interrupted_threads,
            self.sleeping_threads.take(),
            self.zombie_threads.take(),
        )
    }

    fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Search in the list of interrupted threads.
        if self
            .interrupted_threads
            .iter()
            .any(|thread| thread.tid() == tid)
        {
            return true;
        }

        // Search in the list of ready threads.
        if let Some(ready_threads) = &self.ready_threads {
            if ready_threads.iter().any(|thread| thread.tid() == tid) {
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
            if zombie_threads.iter().any(|thread| thread.tid() == tid) {
                return true;
            }
        }

        false
    }
}

pub struct RunnableProcessWithReadyAndInteruptThread {
    state: Box<ProcessState>,
    ready_threads: NonEmptyVecDeque<ReadyThread>,
    interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl RunnableProcessWithReadyAndInteruptThread {
    fn new(
        state: Box<ProcessState>,
        ready_threads: NonEmptyVecDeque<ReadyThread>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
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

    fn state(&self) -> &ProcessState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    fn run(mut self) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation) {
        let (ready_threads, next_thread) = self.ready_threads.pop_front();
        let (running_thread, next_context) = next_thread.resume();
        (
            RunningProcess::new(
                self.state,
                running_thread,
                NonEmptyVecDeque::from(ready_threads),
                Some(self.interrupted_threads),
                self.sleeping_threads.take(),
                self.zombie_threads.take(),
            ),
            None,
            next_context,
        )
    }

    fn interrupt(thread: SleepingThread) -> InterruptedThread {
        thread.interrupt(InterruptReason::Killed)
    }

    fn terminate(mut self) -> Result<RunnableProcessWithInterruptedThreads, ZombieProcess> {
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
        let mut interrupted_threads: NonEmptyVecDeque<InterruptedThread> = self.interrupted_threads;

        // Terminate all sleeping threads.
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            let more_interrupted_threads = NonEmptyVecDeque::map(sleeping_threads, Self::interrupt);
            interrupted_threads.append(more_interrupted_threads);
        }

        Ok(RunnableProcessWithInterruptedThreads::new(
            self.state,
            None,
            interrupted_threads,
            None,
            Some(zombie_threads),
        ))
    }

    fn wakeup(
        mut self,
        tid: ThreadIdentifier,
    ) -> Result<RunnableProcessWithReadyAndInteruptThread, Self> {
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            match sleeping_threads.remove_if(|thread| thread.id() == tid) {
                Ok((sleeping_threads, sleeping_thread)) => {
                    let ready_thread: ReadyThread = sleeping_thread.wakeup();
                    self.ready_threads.push_back(ready_thread);
                    Ok(RunnableProcessWithReadyAndInteruptThread::new(
                        self.state,
                        self.ready_threads,
                        self.interrupted_threads,
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

    fn add_thread(mut self, ready_thread: ReadyThread) -> Self {
        trace!("add_thread(): self.pid={:?}, ready_thread={:?}", self.state.pid, ready_thread);
        self.ready_threads.push_back(ready_thread);
        self
    }

    fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Search in the list of ready threads.
        if self.ready_threads.iter().any(|thread| thread.tid() == tid) {
            return true;
        }

        // Search in the list of interrupted threads.
        if self
            .interrupted_threads
            .iter()
            .any(|thread| thread.tid() == tid)
        {
            return true;
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &self.sleeping_threads {
            if sleeping_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &self.zombie_threads {
            if zombie_threads.iter().any(|thread| thread.tid() == tid) {
                return true;
            }
        }

        false
    }
}

///
/// # Description
///
/// A type that represents a process that is ready to run.
///
pub enum RunnableProcess {
    WithReadyThread(RunnableProcessWithReadyThread),
    WithInterruptedThreads(RunnableProcessWithInterruptedThreads),
    ReadyAndInteruptThread(RunnableProcessWithReadyAndInteruptThread),
}

impl RunnableProcess {
    pub fn new(
        pid: ProcessIdentifier,
        ready_thread: ReadyThread,
        vmem: Vmem,
        user_stack_allocator: Option<UserStackAllocator>,
    ) -> Self {
        RunnableProcess::WithReadyThread(RunnableProcessWithReadyThread::new(
            pid,
            ready_thread,
            vmem,
            user_stack_allocator,
        ))
    }

    pub fn from_state_with_ready_thread(
        state: Box<ProcessState>,
        ready_threads: NonEmptyVecDeque<ReadyThread>,
        interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
        sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        RunnableProcess::WithReadyThread(RunnableProcessWithReadyThread::from_state(
            state,
            ready_threads,
            interrupted_threads,
            sleeping_threads,
            zombie_threads,
        ))
    }

    pub fn from_state_with_interrupted_threads(
        state: Box<ProcessState>,
        ready_threads: Option<NonEmptyVecDeque<ReadyThread>>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
        sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        RunnableProcess::WithInterruptedThreads(RunnableProcessWithInterruptedThreads::new(
            state,
            ready_threads,
            interrupted_threads,
            sleeping_threads,
            zombie_threads,
        ))
    }

    pub fn from_state_with_ready_and_interrupted_threads(
        state: Box<ProcessState>,
        ready_threads: NonEmptyVecDeque<ReadyThread>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
        sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        RunnableProcess::ReadyAndInteruptThread(RunnableProcessWithReadyAndInteruptThread::new(
            state,
            ready_threads,
            interrupted_threads,
            sleeping_threads,
            zombie_threads,
        ))
    }

    pub fn state(&self) -> &ProcessState {
        match self {
            RunnableProcess::WithReadyThread(process) => process.state(),
            RunnableProcess::WithInterruptedThreads(process) => process.state(),
            RunnableProcess::ReadyAndInteruptThread(process) => process.state(),
        }
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        match self {
            RunnableProcess::WithReadyThread(process) => process.state_mut(),
            RunnableProcess::WithInterruptedThreads(process) => process.state_mut(),
            RunnableProcess::ReadyAndInteruptThread(process) => process.state_mut(),
        }
    }

    pub fn terminate(self) -> Result<RunnableProcess, ZombieProcess> {
        match self {
            RunnableProcess::WithReadyThread(process) => match process.terminate() {
                Ok(process) => Ok(RunnableProcess::WithInterruptedThreads(process)),
                Err(process) => Err(process),
            },
            RunnableProcess::WithInterruptedThreads(process) => match process.terminate() {
                Ok(process) => Ok(RunnableProcess::WithInterruptedThreads(process)),
                Err(process) => Err(process),
            },
            RunnableProcess::ReadyAndInteruptThread(process) => match process.terminate() {
                Ok(process) => Ok(RunnableProcess::WithInterruptedThreads(process)),
                Err(process) => Err(process),
            },
        }
    }

    pub fn run(self) -> (RunningProcess, Option<InterruptReason>, *mut ContextInformation) {
        match self {
            RunnableProcess::WithReadyThread(process) => process.run(),
            RunnableProcess::WithInterruptedThreads(process) => process.run(),
            RunnableProcess::ReadyAndInteruptThread(process) => process.run(),
        }
    }

    pub fn wakeup(self, tid: ThreadIdentifier) -> Result<RunnableProcess, RunnableProcess> {
        match self {
            RunnableProcess::WithReadyThread(process) => match process.wakeup(tid) {
                Ok(process) => Ok(RunnableProcess::WithReadyThread(process)),
                Err(process) => Err(RunnableProcess::WithReadyThread(process)),
            },
            RunnableProcess::WithInterruptedThreads(process) => match process.wakeup(tid) {
                Ok(process) => Ok(RunnableProcess::ReadyAndInteruptThread(process)),
                Err(process) => Err(RunnableProcess::WithInterruptedThreads(process)),
            },
            RunnableProcess::ReadyAndInteruptThread(process) => match process.wakeup(tid) {
                Ok(process) => Ok(RunnableProcess::ReadyAndInteruptThread(process)),
                Err(process) => Err(RunnableProcess::ReadyAndInteruptThread(process)),
            },
        }
    }

    pub fn add_thread(self, ready_thread: ReadyThread) -> Self {
        match self {
            RunnableProcess::WithReadyThread(process) => {
                RunnableProcess::WithReadyThread(process.add_thread(ready_thread))
            },
            RunnableProcess::WithInterruptedThreads(process) => {
                RunnableProcess::ReadyAndInteruptThread(process.add_thread(ready_thread))
            },
            RunnableProcess::ReadyAndInteruptThread(process) => {
                RunnableProcess::ReadyAndInteruptThread(process.add_thread(ready_thread))
            },
        }
    }

    pub fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        match self {
            RunnableProcess::WithReadyThread(process) => process.has_thread(tid),
            RunnableProcess::WithInterruptedThreads(process) => process.has_thread(tid),
            RunnableProcess::ReadyAndInteruptThread(process) => process.has_thread(tid),
        }
    }
}

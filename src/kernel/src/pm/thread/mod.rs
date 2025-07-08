// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::{
        kstack::KernelStack,
        ustack::UserStack,
    },
    pm::{
        clock,
        sync::{
            condvar::Condvar,
            mutex::MutexGuard,
        },
    },
};
use ::alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    fmt,
};
use ::core::{
    fmt::Debug,
    pin::Pin,
};
use ::sys::{
    error::ErrorCode,
    pm::{
        MutexAddress,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};

//==================================================================================================
// Thread State
//==================================================================================================

struct ThreadState {
    /// Thread identifier.
    id: ThreadIdentifier,
    /// Kernel stack.
    kernel_stack: Option<KernelStack>,
    /// User stack.
    user_stack: Option<UserStack>,
    /// Condition variable for join.
    join_cond: Condvar,
    /// Execution context.
    context: Pin<Box<ContextInformation>>,
    /// Lookup table of locked mutexes.
    locked_mutexes: BTreeMap<MutexAddress, MutexGuard>,
    /// Interrupt reason, if any.
    interrupt_reason: Option<InterruptReason>,
}

impl ThreadState {
    fn new(
        id: ThreadIdentifier,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self {
            id,
            context: Box::pin(context),
            kernel_stack,
            user_stack,
            join_cond: Condvar::new(),
            locked_mutexes: BTreeMap::new(),
            interrupt_reason: None,
        }
    }

    fn context_mut(&mut self) -> *mut ContextInformation {
        self.context.as_mut().get_mut() as *mut ContextInformation
    }
}

impl fmt::Debug for ThreadState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Thread {{ id: {:?} }}", self.id)
    }
}

impl Drop for ThreadState {
    fn drop(&mut self) {
        if !self.locked_mutexes.is_empty() {
            error!(
                "drop(): dropping thread state with locked mutexes (self.id={:?}, \
                 self.locked_mutexes={:?})",
                self.id, self.locked_mutexes
            );
        }
    }
}

//==================================================================================================
// Running Thread
//==================================================================================================

#[derive(Debug)]
pub struct RunningThread(Box<ThreadState>);

impl RunningThread {
    pub fn sleep(mut self, alarm: Option<SystemTime>) -> (SleepingThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (
            SleepingThread {
                thread: self.0,
                alarm,
            },
            ctx,
        )
    }

    pub fn schedule(mut self) -> (ReadyThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (ReadyThread::from_state(self.0), ctx)
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the target thread.
    ///
    /// # Returns
    ///
    /// The identifier of the target thread.
    ///
    pub fn id(&self) -> ThreadIdentifier {
        self.0.id
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads waiting on the join condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn join_cond(&self) -> Condvar {
        // NOTE: we must wake up all, otherwise some threads can be left waiting forever.
        self.0.join_cond.clone()
    }

    pub fn exit(mut self, status: ExitStatus) -> (ZombieThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (
            ZombieThread {
                state: self.0,
                status,
            },
            ctx,
        )
    }

    ///
    /// # Description
    ///
    /// Stores a mutex guard in the target thread.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    /// - `guard`: Mutex guard.
    ///
    pub fn put_mutex_guard(&mut self, mutex_addr: MutexAddress, guard: MutexGuard) {
        self.0.locked_mutexes.insert(mutex_addr, guard);
    }

    ///
    /// # Description
    ///
    /// Returns the mutex guard associated with the target thread.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    ///
    /// # Returns
    ///
    /// If the mutex guard is found, it is returned. Otherwise, `None` is returned instead.
    ///
    pub fn take_mutex_guard(&mut self, mutex_addr: MutexAddress) -> Option<MutexGuard> {
        self.0.locked_mutexes.remove(&mutex_addr)
    }
}

//==================================================================================================
// Ready Thread
//==================================================================================================

#[derive(Debug)]
pub struct ReadyThread {
    state: Box<ThreadState>,
    admission_time: SystemTime,
}

impl ReadyThread {
    pub fn new(
        id: ThreadIdentifier,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self {
            state: Box::new(ThreadState::new(id, kernel_stack, user_stack, context)),
            admission_time: clock::now(),
        }
    }

    fn from_state(state: Box<ThreadState>) -> Self {
        Self {
            state,
            admission_time: clock::now(),
        }
    }

    pub fn tid(&self) -> ThreadIdentifier {
        self.state.id
    }

    pub fn run(mut self) -> (RunningThread, Option<InterruptReason>, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        let interrupt_reason: Option<InterruptReason> = self.state.interrupt_reason.take();
        (RunningThread(self.state), interrupt_reason, ctx)
    }

    pub fn terminate(self) -> ZombieThread {
        ZombieThread {
            state: self.state,
            status: ErrorCode::Interrupted.into(),
        }
    }

    pub fn join_cond(&self) -> Condvar {
        self.state.join_cond.clone()
    }

    pub fn admission_time(&self) -> SystemTime {
        self.admission_time
    }
}

//==================================================================================================
// Sleeping Thread
//==================================================================================================

#[derive(Debug)]
pub struct SleepingThread {
    thread: Box<ThreadState>,
    alarm: Option<SystemTime>,
}

impl SleepingThread {
    pub fn wakeup(self) -> ReadyThread {
        ReadyThread::from_state(self.thread)
    }

    pub fn interrupt(self, reason: InterruptReason) -> InterruptedThread {
        InterruptedThread {
            thread: self.thread,
            reason,
        }
    }

    pub fn id(&self) -> ThreadIdentifier {
        self.thread.id
    }

    pub fn join_cond(&self) -> Condvar {
        self.thread.join_cond.clone()
    }

    pub fn alarm(&self) -> Option<SystemTime> {
        self.alarm
    }
}

//==================================================================================================
// Interrupted Thread
//==================================================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptReason {
    /// Process was killed.
    Killed,
    /// Timer expired.
    TimedOut,
}

#[derive(Debug)]
pub struct InterruptedThread {
    thread: Box<ThreadState>,
    reason: InterruptReason,
}

impl InterruptedThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.thread.id
    }

    pub fn resume(mut self) -> ReadyThread {
        self.thread.interrupt_reason = Some(self.reason);
        ReadyThread::from_state(self.thread)
    }

    pub fn join_cond(&self) -> Condvar {
        self.thread.join_cond.clone()
    }
}

//==================================================================================================
// Zombie Thread
//==================================================================================================

#[derive(Debug)]
pub struct ZombieThread {
    status: ExitStatus,
    state: Box<ThreadState>,
}

impl ZombieThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.state.id
    }

    pub fn harvest(mut self) -> (Option<KernelStack>, Option<UserStack>) {
        (self.state.kernel_stack.take(), self.state.user_stack.take())
    }

    pub fn status(&self) -> ExitStatus {
        self.status
    }
}

//==================================================================================================
// Thread Manager
//==================================================================================================

pub struct ThreadManager {
    next_id: ThreadIdentifier,
}

impl ThreadManager {
    fn new() -> (ReadyThread, Self) {
        let kernel: ReadyThread =
            ReadyThread::new(From::<i32>::from(0), None, None, ContextInformation::default());
        (
            kernel,
            Self {
                next_id: From::<i32>::from(1),
            },
        )
    }

    pub fn create_thread(
        &mut self,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> ReadyThread {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(<i32>::from(self.next_id) + 1);

        ReadyThread::new(id, kernel_stack, user_stack, context)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Initializes the thread manager.
pub fn init() -> (ReadyThread, ThreadManager) {
    // TODO: check for double initialization.

    ThreadManager::new()
}

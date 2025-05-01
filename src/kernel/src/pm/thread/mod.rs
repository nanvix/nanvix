// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::ustack::UserStack,
    pm::sync::{
        condvar::Condvar,
        mutex::MutexGuard,
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
    ExitStatus,
};

//==================================================================================================
// Thread State
//==================================================================================================

struct ThreadState {
    /// Thread identifier.
    id: ThreadIdentifier,
    /// User stack.
    user_stack: Option<UserStack>,
    /// Condition variable for join.
    join_cond: Condvar,
    /// Execution context.
    context: Pin<Box<ContextInformation>>,
    /// Lookup table of locked mutexes.
    locked_mutexes: BTreeMap<MutexAddress, MutexGuard>,
}

impl ThreadState {
    fn new(
        id: ThreadIdentifier,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self {
            id,
            context: Box::pin(context),
            user_stack,
            join_cond: Condvar::new(),
            locked_mutexes: BTreeMap::new(),
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

//==================================================================================================
// Running Thread
//==================================================================================================

pub struct RunningThread(ThreadState);

impl RunningThread {
    pub fn sleep(mut self) -> (SleepingThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (SleepingThread(self.0), ctx)
    }

    pub fn schedule(mut self) -> (ReadyThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (ReadyThread(self.0), ctx)
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

pub struct ReadyThread(ThreadState);

impl ReadyThread {
    pub fn new(
        id: ThreadIdentifier,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self(ThreadState::new(id, user_stack, context))
    }

    pub fn tid(&self) -> ThreadIdentifier {
        self.0.id
    }

    pub fn resume(mut self) -> (RunningThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (RunningThread(self.0), ctx)
    }

    pub fn terminate(self) -> ZombieThread {
        ZombieThread {
            state: self.0,
            status: ErrorCode::Interrupted.into(),
        }
    }

    pub fn join_cond(&self) -> Condvar {
        self.0.join_cond.clone()
    }
}

//==================================================================================================
// Sleeping Thread
//==================================================================================================

pub struct SleepingThread(ThreadState);

impl SleepingThread {
    pub fn wakeup(self) -> ReadyThread {
        ReadyThread(self.0)
    }

    pub fn interrupt(self, reason: InterruptReason) -> InterruptedThread {
        InterruptedThread {
            thread: self.0,
            reason,
        }
    }

    pub fn id(&self) -> ThreadIdentifier {
        self.0.id
    }

    pub fn join_cond(&self) -> Condvar {
        self.0.join_cond.clone()
    }
}

//==================================================================================================
// Interrupted Thread
//==================================================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptReason {
    /// Process was killed.
    Killed,
}
pub struct InterruptedThread {
    thread: ThreadState,
    reason: InterruptReason,
}

impl InterruptedThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.thread.id
    }

    pub fn resume(mut self) -> (RunningThread, InterruptReason, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.thread.context_mut();
        (RunningThread(self.thread), self.reason, ctx)
    }

    pub fn join_cond(&self) -> Condvar {
        self.thread.join_cond.clone()
    }
}

//==================================================================================================
// Zombie Thread
//==================================================================================================

#[allow(unused)]
pub struct ZombieThread {
    status: ExitStatus,
    state: ThreadState,
}

impl ZombieThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.state.id
    }

    pub fn harvest(mut self) -> Option<UserStack> {
        self.state.user_stack.take()
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
            ReadyThread::new(From::<u32>::from(0), None, ContextInformation::default());
        (
            kernel,
            Self {
                next_id: From::<u32>::from(1),
            },
        )
    }

    pub fn create_thread(
        &mut self,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> ReadyThread {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(Into::<usize>::into(self.next_id) + 1);

        ReadyThread(ThreadState::new(id, user_stack, context))
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

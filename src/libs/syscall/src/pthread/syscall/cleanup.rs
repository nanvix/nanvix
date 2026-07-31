// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::tda::Pointer;
use ::alloc::{
    collections::btree_map::BTreeMap,
    vec::Vec,
};
use ::spin::{
    Lazy,
    Mutex,
};
use ::sysapi::{
    ffi::c_void,
    sys_types::pthread_t,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A cancellation cleanup handler.
struct CleanupHandler {
    /// Routine to invoke.
    routine: Option<extern "C" fn(*mut c_void)>,
    /// Argument to pass to the routine.
    arg: Pointer,
}

impl CleanupHandler {
    /// Invokes this cleanup handler.
    fn call(self) {
        if let Some(routine) = self.routine {
            let arg: *mut c_void = self.arg.into();
            routine(arg);
        }
    }
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Cancellation cleanup handlers, indexed by thread identifier.
static CLEANUP_HANDLERS: Lazy<Mutex<BTreeMap<pthread_t, Vec<CleanupHandler>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns the identifier of the calling thread.
fn current_thread(operation: &str) -> Option<pthread_t> {
    match super::try_pthread_self() {
        Ok(tid) => Some(tid),
        Err(error) => {
            ::syslog::warn!("{operation}(): failed to get thread identifier ({error:?})");
            None
        },
    }
}

/// Removes and returns the top cleanup handler for `tid`.
fn pop_handler(tid: pthread_t) -> Option<CleanupHandler> {
    let mut handlers = CLEANUP_HANDLERS.lock();
    let (handler, is_empty): (Option<CleanupHandler>, bool) = {
        let stack: &mut Vec<CleanupHandler> = handlers.get_mut(&tid)?;
        let handler: Option<CleanupHandler> = stack.pop();
        (handler, stack.is_empty())
    };

    if is_empty {
        handlers.remove(&tid);
    }

    handler
}

/// Pushes a cancellation cleanup handler for the calling thread.
pub fn pthread_cleanup_push(routine: Option<extern "C" fn(*mut c_void)>, arg: *mut c_void) {
    let Some(tid): Option<pthread_t> = current_thread("_pthread_cleanup_push") else {
        return;
    };
    let handler: CleanupHandler = CleanupHandler {
        routine,
        arg: Pointer::from(arg),
    };

    CLEANUP_HANDLERS
        .lock()
        .entry(tid)
        .or_default()
        .push(handler);
}

/// Pops the top cancellation cleanup handler for the calling thread and optionally invokes it.
pub fn pthread_cleanup_pop(execute: bool) {
    let Some(tid): Option<pthread_t> = current_thread("_pthread_cleanup_pop") else {
        return;
    };

    if let Some(handler) = pop_handler(tid) {
        if execute {
            handler.call();
        }
    }
}

/// Invokes and removes all cancellation cleanup handlers for the calling thread.
pub(super) fn run() {
    let Some(tid): Option<pthread_t> = current_thread("pthread_exit") else {
        return;
    };

    while let Some(handler) = pop_handler(tid) {
        handler.call();
    }
}

/// Removes all cancellation cleanup handlers for the calling thread without invoking them.
pub(super) fn discard() {
    let Some(tid): Option<pthread_t> = current_thread("pthread_start") else {
        return;
    };

    CLEANUP_HANDLERS.lock().remove(&tid);
}

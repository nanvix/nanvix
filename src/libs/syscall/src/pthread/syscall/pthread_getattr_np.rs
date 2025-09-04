// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall::STACKS;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_types::{
    c_size_t,
    pthread_attr_t,
    pthread_t,
};
use sysalloc::Address;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the thread attributes object with the actual values of a given thread.
///
/// # Parameters
///
/// * `thread` - Thread whose attributes will be retrieved.
/// * `attr` - Place where the thread attributes will be stored.
///
/// # Returns
///
/// On success, this function returns empty. Otherwise, it returns an error indicating the cause
/// of the failure.
///
/// # Errors
///
/// The following errors can be returned by this function:
///
/// - [`ErrorCode::ResourceBusy`] if `attr` references a thread attribute object that was already
///   initialized.
///
pub fn pthread_getattr_np(thread: pthread_t, attr: &mut pthread_attr_t) -> Result<(), Error> {
    ::syslog::trace!("pthread_getattr_np(): thread={:?}, attr={:p}", thread, attr as *const _);

    // Check if `attr` references a thread attributes object that was already initialized.
    if attr.is_initialized != 0 {
        let reason: &'static str = "thread attributes object was already initialized";
        ::syslog::error!("pthread_getattr_np(): {reason} (attr={:p})", attr as *const _);
        return Err(Error::new(ErrorCode::ResourceBusy, reason));
    }

    *attr = pthread_attr_t::default();

    // Try to retrieve the thread's stack info. If no entry is found, the caller is the main thread
    // of the current process and the previously-initialized default attributes contain the correct
    // values for the main thread's stack.
    if let Some(stack) = STACKS.lock().get(&thread) {
        attr.stackaddr = stack.base().into_raw_value() as *mut _;
        attr.stacksize = stack.size() as c_size_t;
    }

    Ok(())
}

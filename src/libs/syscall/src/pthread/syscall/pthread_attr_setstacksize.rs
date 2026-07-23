// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::config::memory_layout::USER_STACK_MIN_SIZE;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_types::{
    c_size_t,
    pthread_attr_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the stack size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stacksize`: Stack size to set.
///
/// # Return Value
///
/// On success, this function returns empty. Otherwise, it returns an error indicating the reason
/// for the failure.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`] if `attr` references an uninitialized thread attributes object.
/// - [`ErrorCode::InvalidArgument`] if `stacksize` is smaller than the minimum thread stack size.
///
pub fn pthread_attr_setstacksize(
    attr: &mut pthread_attr_t,
    stacksize: c_size_t,
) -> Result<(), Error> {
    ::syslog::trace!(
        "pthread_attr_setstacksize(): attr={:p}, stacksize={stacksize}",
        attr as *const _
    );

    // Ensure the attributes object is initialized.
    if attr.is_initialized == 0 {
        let reason: &'static str = "thread attributes object was not initialized";
        ::syslog::warn!(
            "pthread_attr_setstacksize(): {reason} (attr={:p}, stacksize={stacksize})",
            attr as *const _
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Ensure the requested stack size is supported.
    if (stacksize as usize) < USER_STACK_MIN_SIZE {
        let reason: &'static str = "stack size is smaller than the minimum thread stack size";
        ::syslog::warn!(
            "pthread_attr_setstacksize(): {reason} (attr={:p}, stacksize={stacksize})",
            attr as *const _
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    attr.stacksize = stacksize;

    Ok(())
}

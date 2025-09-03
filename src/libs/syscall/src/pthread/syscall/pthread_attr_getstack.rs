// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_void,
    sys_types::{
        c_size_t,
        pthread_attr_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the stack address and size attributes from a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stackaddr`: Output stack base address.
/// - `stacksize`: Output stack size.
///
/// # Return Value
///
/// On success, this function returns empty. Otherwise, it returns an error indicating the reason
/// for the failure.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`] if `attr` references a thread attribute object that was not
///   initialized.
///
pub fn pthread_attr_getstack(
    attr: &pthread_attr_t,
    stackaddr: &mut *mut c_void,
    stacksize: &mut c_size_t,
) -> Result<(), Error> {
    ::syslog::trace!(
        "pthread_attr_getstack(): attr={:p}, stackaddr={:p}, stacksize={:p}",
        attr as *const _,
        stackaddr as *const _,
        stacksize as *const _
    );

    // Ensure the attributes object is initialized.
    if attr.is_initialized == 0 {
        let reason: &'static str = "thread attributes object was not initialized";
        ::syslog::error!("pthread_attr_getstack(): {reason} (attr={:p})", attr as *const _);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Store stack attributes.
    *stackaddr = attr.stackaddr;
    *stacksize = attr.stacksize;

    Ok(())
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_types::pthread_attr_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes a thread attributes object with default values.
///
/// # Parameters
///
/// - `attr` - Mutable reference to the thread attributes object to be initialized.
///
/// # Return Value
///
/// On success, this function returns empty. Otherwise, it returns an error indicating the reason
/// for the failure.
///
/// # Errors
///
/// The following errors can be returned by this function:
///
/// - [`ErrorCode::InvalidArgument`] if `attr` references a thread attribute object that was already
///   initialized.
///
/// # Notes
///
/// - Calling this function on a thread attributes object that has already been initialized results
///   in undefined behavior.
///
/// - When a thread attributes object is no longer required, it should be destroyed using the
///   `pthread_attr_destroy()` function.
///
/// - This function always succeed, but portable applications should nevertheless handle a possible
///   error return.
///
pub fn pthread_attr_init(attr: &mut pthread_attr_t) -> Result<(), Error> {
    ::syslog::trace!("pthread_attr_init(): attr={:p}", attr as *const _);

    // Check if `attr` references a thread attributes object that was already initialized.
    if attr.is_initialized != 0 {
        let reason: &'static str = "thread attributes object was already initialized";
        ::syslog::error!("pthread_attr_init(): {reason} (attr={:p})", attr as *const _);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    *attr = pthread_attr_t::default();

    Ok(())
}

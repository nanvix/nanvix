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
/// Destroys a thread attributes object.
///
/// # Parameters
///
/// - `attr` - Mutable reference to the thread attributes object to be destroyed.
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
/// - [`ErrorCode::InvalidArgument`] if `attr` references a thread attribute object that was not
///   initialized.
///
/// # Notes
///
/// - Destroying a thread attributes object has no effect on threads that were created using that
///   object.
///
/// - This function always succeed, but portable applications should nevertheless handle a possible
///   error return.
///
pub fn pthread_attr_destroy(attr: &mut pthread_attr_t) -> Result<(), Error> {
    ::syslog::trace!("pthread_attr_destroy(): attr={:p}", attr as *const _);

    // Check if `attr` references a thread attributes object that was not initialized.
    if attr.is_initialized == 0 {
        let reason: &'static str = "thread attributes object was not initialized";
        ::syslog::error!("pthread_attr_destroy(): {reason} (attr={:p})", attr as *const _);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    attr.is_initialized = 0;

    Ok(())
}

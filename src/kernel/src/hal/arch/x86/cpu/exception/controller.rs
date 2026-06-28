// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::{
    ContextInformation,
    ExceptionInformation,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// High-level exception dispatcher.
///
/// # Parameters
///
/// - `excp` Exception information.
/// - `ctx`  Context information.
///
/// # Safety
///
/// This function is unsafe for the following conditions:
/// - It dereferences raw pointers.
/// - It accesses global variables.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_exception(
    excp: *const ExceptionInformation,
    ctx: *mut ContextInformation,
) {
    super::exception_controller::dispatch(&*excp, &mut *ctx);
}

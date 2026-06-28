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
/// - `ctx`  Context information.
/// - `excp` Exception information.
///
/// # Note
///
/// On x86_64, the hooks.S assembly passes the context pointer in RDI (first argument) and the
/// exception information pointer in RSI (second argument). This is the opposite order from x86.
///
/// # Safety
///
/// This function is unsafe for the following conditions:
/// - It dereferences raw pointers.
/// - It accesses global variables.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_exception(
    ctx: *mut ContextInformation,
    excp: *const ExceptionInformation,
) {
    super::exception_controller::dispatch(&*excp, &mut *ctx);
}

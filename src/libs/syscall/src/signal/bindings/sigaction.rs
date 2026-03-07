// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigaction_t;
use ::sys::{
    error::ErrorCode,
    number::KcallNumber,
    signal::{
        SIG_DFL,
        SIG_IGN,
    },
};
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Registers or queries a signal handler for the given signal number.
///
/// # Parameters
///
/// - `signum`: Signal number.
/// - `act`: Pointer to the new signal action (may be null for query-only).
/// - `oldact`: Pointer to receive the previous signal action (may be null).
///
/// # Returns
///
/// `0` on success, `-1` on error (with errno set).
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction_t,
    oldact: *mut sigaction_t,
) -> c_int {
    // Determine the new handler to install.
    let (new_handler, flags): (u32, u32) = if act.is_null() {
        // Query-only: pass sentinel to kernel.
        (u32::MAX, 0)
    } else {
        let sa: &sigaction_t = unsafe { &*act };
        let handler_addr: usize = sa.sa_handler as usize;
        let handler_u32: u32 = match handler_addr {
            SIG_DFL => 0,
            SIG_IGN => 1,
            addr => addr as u32,
        };
        (handler_u32, sa.sa_flags as u32)
    };

    let result: i64 = ::sys::kcall3!(
        KcallNumber::Sigaction.into(),
        signum as u32,
        new_handler,
        flags
    );

    if result < 0 {
        unsafe {
            *__errno_location() = match ErrorCode::try_from(result) {
                Ok(code) => code.get(),
                Err(_) => ErrorCode::InvalidSysCall.get(),
            };
        }
        return -1;
    }

    // Store the old handler in oldact if requested.
    if !oldact.is_null() {
        let old_handler_addr: usize = result as usize;
        let old_sa: &mut sigaction_t = unsafe { &mut *oldact };
        old_sa.sa_handler = unsafe { core::mem::transmute(old_handler_addr) };
        old_sa.sa_flags = 0;
        old_sa.sa_mask.bits = [0u64; 16];
    }

    0
}

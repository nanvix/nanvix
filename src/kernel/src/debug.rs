// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_debug(buf: &[u8]) -> Result<(), Error> {
    let message: &str = match core::str::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => {
            let reason: &str = "invalid UTF-8";
            error!("{reason} (error={e:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };
    // On Hyperlight, write directly to DebugPrint port (bypass klog buffer
    // which has stale CoW state after snapshot restore).
    #[cfg(feature = "hyperlight")]
    ::hyperlight_guest::exit::debug_print(message);
    #[cfg(not(feature = "hyperlight"))]
    unsafe { crate::klog::puts(message) };

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for writing a debug message from user space.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: User-space pointer to the message buffer.
/// - `arg1`: Size of the message in bytes.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
#[cfg_attr(feature = "hyperlight", allow(unused_variables))]
pub fn debug(pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Maximum size of a debug message buffer (from kernel configuration).
    const BUFFER_SIZE: usize = config::kernel::DEBUG_BUFFER_SIZE;
    let user_buffer: usize = arg0 as usize;
    let size: usize = arg1 as usize;

    // skip zero-length messages
    if size == 0 {
        return KcallResult::ok();
    }

    // Sanity check message size.
    if size > BUFFER_SIZE {
        let reason: &str = "message too large";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    let mut kernel_buffer: [u8; BUFFER_SIZE + 1] = [0; BUFFER_SIZE + 1];

    let src: VirtualAddress = VirtualAddress::new(user_buffer);
    let dst: VirtualAddress = VirtualAddress::new(kernel_buffer.as_mut_ptr() as usize);

    // On Hyperlight (shared PD), user VAs are directly accessible.
    // Skip vmcopy_from_user (uses __phys_memcpy which breaks after CoW
    // because GPAs are stale). Just read the user buffer directly.
    #[cfg(feature = "hyperlight")]
    {
        let user_buf: &[u8] = unsafe {
            core::slice::from_raw_parts(user_buffer as *const u8, size)
        };
        return match do_debug(user_buf) {
            Ok(()) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        };
    }

    #[cfg(not(feature = "hyperlight"))]
    {
        if let Err(e) = pm.vmcopy_from_user(pid, dst, src, size) {
            return KcallResult::Error(e.code.into());
        }

        let buf: &[u8] = unsafe { core::slice::from_raw_parts(kernel_buffer.as_ptr(), size) };

        match do_debug(buf) {
            Ok(()) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        }
    }
}

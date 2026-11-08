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
// Constants
//==================================================================================================

/// Marker that is written after a truncated debug message.
const TRUNCATION_MARKER: &str = "...";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes a debug message to the kernel log.
///
/// # Parameters
///
/// - `buf`: Message to write.
/// - `truncated`: Whether `buf` holds only the leading part of the message.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
fn do_debug(buf: &[u8], truncated: bool) -> Result<(), Error> {
    let message: &str = match core::str::from_utf8(buf) {
        Ok(s) => s,
        // The cut of a truncated message may fall in the middle of its last character, which is
        // then dropped. The first chunk holds all bytes that precede that character.
        Err(e) if truncated && e.error_len().is_none() => {
            buf.utf8_chunks().next().map_or("", |chunk| chunk.valid())
        },
        Err(e) => {
            let reason: &str = "invalid UTF-8";
            error!("{reason} (error={e:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };
    // SAFETY: the standard output device is present, initialized, and accessed
    // exclusively from a single core with interrupts disabled.
    unsafe { crate::klog::puts(message) };

    if truncated {
        // SAFETY: same as above.
        unsafe { crate::klog::puts(TRUNCATION_MARKER) };
    }

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for writing a debug message from user space. At most `DEBUG_BUFFER_SIZE`
/// bytes of the message are written: a longer message is truncated at a character boundary and
/// followed by a truncation marker.
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

    // Copy at most a buffer-full of the message, so that the work of this kernel call stays
    // bounded. Truncate a longer message rather than reject it, so that its leading part still
    // reaches the kernel log.
    let truncated: bool = size > BUFFER_SIZE;
    let size: usize = size.min(BUFFER_SIZE);

    let mut kernel_buffer: [u8; BUFFER_SIZE + 1] = [0; BUFFER_SIZE + 1];

    let src: VirtualAddress = VirtualAddress::new(user_buffer);
    let dst: VirtualAddress = VirtualAddress::new(kernel_buffer.as_mut_ptr() as usize);

    if let Err(e) = pm.vmcopy_from_user(pid, dst, src, size) {
        return KcallResult::Error(e.code.into());
    }

    let buf: &[u8] = unsafe { core::slice::from_raw_parts(kernel_buffer.as_ptr(), size) };

    match do_debug(buf, truncated) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    kcall2,
    number::KcallNumber,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Maximum number of bytes that a single debug kernel call writes. The kernel truncates longer
/// buffers.
///
pub const MAX_DEBUG_MESSAGE_SIZE: usize = ::config::kernel::DEBUG_BUFFER_SIZE;

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes a buffer to the kernel's standard output device. The kernel writes at most
/// [`MAX_DEBUG_MESSAGE_SIZE`] bytes of the buffer.
///
/// # Parameters
/// - `buf` - Buffer to write.
/// - `size` - Number of bytes to write.
///
/// # Return Values
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
#[unsafe(no_mangle)]
pub fn __kcall_debug(buf: *const u8, size: usize) -> Result<(), Error> {
    let result: i64 = kcall2!(KcallNumber::Debug.into(), buf as u32, size as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to debug()"))
    }
}

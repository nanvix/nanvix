// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    number::KcallNumber,
};

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes a buffer to the kernel's standard output device.
///
/// # Parameters
/// - `buf` - Buffer to write.
/// - `size` - Number of bytes to write.
///
/// # Return Values
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn debug(buf: *const u8, size: usize) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::arch::kcall2(KcallNumber::Debug.into(), buf as u32, size as u32) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to debug()"))
    }
}

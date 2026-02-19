// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that stores the result of a failed kernel call.
///
#[derive(Default, Debug, Clone, Copy)]
pub struct KcallError(i32);

//==================================================================================================
// Implementations
//==================================================================================================

impl From<ErrorCode> for KcallError {
    fn from(code: ErrorCode) -> Self {
        // Negate the error code following the Linux convention: kernel calls return negative errno
        // values on failure so that they can be distinguished from non-negative success values.
        KcallError(-code.get())
    }
}

impl From<KcallError> for i32 {
    fn from(result: KcallError) -> Self {
        result.0
    }
}

impl From<KcallError> for i64 {
    fn from(result: KcallError) -> Self {
        result.0 as i64
    }
}

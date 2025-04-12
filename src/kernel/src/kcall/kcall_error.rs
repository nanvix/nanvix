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
        KcallError(-code.get())
    }
}

impl From<KcallError> for i32 {
    fn from(result: KcallError) -> Self {
        result.0
    }
}

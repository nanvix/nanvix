// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that stores the result of a successful kernel call.
///
#[derive(Default, Debug, Clone, Copy)]
pub struct KcallSuccess(i32);

//==================================================================================================
// Implementations
//==================================================================================================

impl From<usize> for KcallSuccess {
    fn from(code: usize) -> Self {
        match code.try_into() {
            Ok(code) => KcallSuccess(code),
            Err(error) => {
                panic!("failed to convert usize to i32 (usize={:?}, error={:?})", code, error)
            },
        }
    }
}

impl From<u32> for KcallSuccess {
    fn from(code: u32) -> Self {
        match code.try_into() {
            Ok(code) => KcallSuccess(code),
            Err(error) => {
                panic!("failed to convert u32 to i32 (u32={:?}, error={:?})", code, error)
            },
        }
    }
}

impl From<KcallSuccess> for i32 {
    fn from(result: KcallSuccess) -> Self {
        result.0
    }
}

impl From<KcallSuccess> for i64 {
    fn from(result: KcallSuccess) -> Self {
        result.0 as i64
    }
}

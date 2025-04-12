// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//========================================A==========================================================

use crate::kcall::{
    KcallError,
    KcallSuccess,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Copy)]
pub enum KcallResult {
    Success(KcallSuccess),
    Error(KcallError),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl From<KcallResult> for i32 {
    fn from(result: KcallResult) -> Self {
        match result {
            KcallResult::Success(success) => success.into(),
            KcallResult::Error(error) => error.into(),
        }
    }
}

impl KcallResult {
    pub fn ok() -> Self {
        KcallResult::Success(KcallSuccess::default())
    }
}

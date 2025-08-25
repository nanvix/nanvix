// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Imports
//==================================================================================================

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

impl From<KcallResult> for i64 {
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

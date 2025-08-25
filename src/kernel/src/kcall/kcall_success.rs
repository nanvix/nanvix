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
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that stores the result of a successful kernel call.
///
#[derive(Default, Debug, Clone, Copy)]
pub struct KcallSuccess(i64);

//==================================================================================================
// Implementations
//==================================================================================================

impl From<u32> for KcallSuccess {
    fn from(code: u32) -> Self {
        KcallSuccess(code.into())
    }
}

impl From<i32> for KcallSuccess {
    fn from(code: i32) -> Self {
        KcallSuccess(code.into())
    }
}

impl From<i64> for KcallSuccess {
    fn from(code: i64) -> Self {
        KcallSuccess(code)
    }
}

impl From<KcallSuccess> for i64 {
    fn from(result: KcallSuccess) -> Self {
        result.0
    }
}

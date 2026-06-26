// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    prog::Prog,
    types::regex_t,
};
use alloc::boxed::Box;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Releases the storage associated with the compiled expression `preg`.
///
/// # Parameters
///
/// - `preg`: Compiled expression produced by `regcomp()`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `preg` and reclaims the boxed
/// program it owns. The caller must ensure that `preg` was initialized by `regcomp()` and is not
/// used again after this call (without a fresh `regcomp()`).
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn regfree(preg: *mut regex_t) {
    if preg.is_null() {
        return;
    }
    let priv_: *mut core::ffi::c_void = (*preg).priv_;
    if priv_.is_null() {
        return;
    }
    drop(Box::from_raw(priv_.cast::<Prog>()));
    (*preg).priv_ = core::ptr::null_mut();
}

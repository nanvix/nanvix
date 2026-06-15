// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(unused_imports)]
use ::vstd::prelude::*;

///
/// # Description
///
/// Converts an integer address to a mutable raw byte pointer.
///
/// This helper exists because the Verus parser rejects `addr as *mut u8` in
/// verified function bodies. Centralizing the cast in a single helper makes it
/// easy to reuse across the codebase when preparing modules for verification.
///
/// # Parameters
///
/// - `addr`: The integer address to convert.
///
/// # Returns
///
/// A mutable raw pointer to a byte at the given address.
///
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        result as usize == addr,
)]
#[inline]
pub fn usize_to_mut_ptr(addr: usize) -> *mut u8 {
    addr as *mut u8
}

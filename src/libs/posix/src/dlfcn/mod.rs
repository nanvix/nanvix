// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_char,
    c_void,
};
use ::core::mem;

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use syscall::dlclose;
        pub use syscall::dlopen;
        pub use syscall::dlsym;
        pub use syscall::dladdr;
    }
}

//==================================================================================================
// DlInfo
//==================================================================================================

///
/// # Description
///
/// A structure that holds information about a symbol.
///
pub struct DlInfo {
    /// The name of the mapped object.
    pub dli_fname: *const c_char,
    /// The base address of the mapped object.
    pub dli_fbase: *const c_void,
    /// The name of the symbol.
    pub dli_sname: *const c_char,
    /// The base address of the symbol.
    pub dli_saddr: *const c_void,
}

::nvx::sys::static_assert_size!(DlInfo, DlInfo::_SIZE);

impl DlInfo {
    /// Size of the `DlInfo` structure, used for static assertions.
    const _SIZE: usize = mem::size_of::<*const c_char>() // Size of `dli_fname`
        + mem::size_of::<*const c_void>() // Size of `dli_fbase`
        + mem::size_of::<*const c_char>() // Size of `dli_sname`
        + mem::size_of::<*const c_void>(); // Size of `dli_saddr`
}

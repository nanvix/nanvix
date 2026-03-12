// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::sysapi::ffi::{
    c_char,
    c_void,
};

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(all(feature = "syscall", feature = "dlfcn"))] {
        mod syscall;
        pub use syscall::DlHandle;
        pub use syscall::dlclose;
        pub use syscall::dlopen;
        pub use syscall::dlsym;
        pub use syscall::dladdr;
        pub use syscall::dlinit;
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

::static_assert::assert_eq_size!(DlInfo, DlInfo::_SIZE);

impl DlInfo {
    /// Size of the `DlInfo` structure, used for static assertions.
    const _SIZE: usize = mem::size_of::<*const c_char>() // Size of `dli_fname`
        + mem::size_of::<*const c_void>() // Size of `dli_fbase`
        + mem::size_of::<*const c_char>() // Size of `dli_sname`
        + mem::size_of::<*const c_void>(); // Size of `dli_saddr`
}

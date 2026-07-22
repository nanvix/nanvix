// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_char,
    c_int,
    c_void,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Relocations are performed at an implementation-defined time.
pub const RTLD_LAZY: c_int = 0x1;

/// Relocations are performed when the object is loaded.
pub const RTLD_NOW: c_int = 0x2;

/// Symbols are available for relocation processing of other objects.
pub const RTLD_GLOBAL: c_int = 0x4;

/// Symbols are not made available to other objects.
pub const RTLD_LOCAL: c_int = 0x0;

/// Pseudo-handle that searches the global symbol scope.
#[allow(clippy::zero_ptr)]
pub const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void;

//==================================================================================================
// Structures
//==================================================================================================

/// Symbol information returned by `dladdr()`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DlInfo {
    /// Pathname of the mapped object.
    pub dli_fname: *const c_char,
    /// Base address of the mapped object.
    pub dli_fbase: *mut c_void,
    /// Name of the nearest symbol.
    pub dli_sname: *const c_char,
    /// Exact address of the symbol.
    pub dli_saddr: *mut c_void,
}

::static_assert::assert_eq_size!(DlInfo, 4 * ::core::mem::size_of::<*const c_void>());
::static_assert::assert_eq_align!(DlInfo, ::core::mem::align_of::<*const c_void>());
::static_assert::assert_eq!(::core::mem::offset_of!(DlInfo, dli_fname) == 0);
::static_assert::assert_eq!(
    ::core::mem::offset_of!(DlInfo, dli_fbase) == ::core::mem::size_of::<*const c_void>()
);
::static_assert::assert_eq!(
    ::core::mem::offset_of!(DlInfo, dli_sname) == 2 * ::core::mem::size_of::<*const c_void>()
);
::static_assert::assert_eq!(
    ::core::mem::offset_of!(DlInfo, dli_saddr) == 3 * ::core::mem::size_of::<*const c_void>()
);

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::{
    errno::ENOSYS,
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the host name of the system. Nanvix exposes a fixed host name and provides no facility to
/// change it, so this interface is a stub that always fails. It exists so that portable software
/// referencing `sethostname()` compiles and links; the host name remains read-only and the
/// limitation is documented as a gap.
///
/// # Parameters
///
/// - `name`: Pointer to the new host name (ignored).
/// - `len`: Length in bytes of the new host name (ignored).
///
/// # Returns
///
/// `-1`, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it writes the thread-local `errno` location.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sethostname(_name: *const c_char, _len: c_size_t) -> c_int {
    unsafe {
        *__errno_location() = ENOSYS;
    }
    -1
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Pseudo-terminal (PTY) interfaces.
//!
//! Nanvix does not provide a pseudo-terminal subsystem, so these interfaces are stubs that fail
//! with `ENOSYS`. They exist so that portable software which references the POSIX PTY API compiles
//! and links; applets that depend on PTYs are non-functional and documented as gaps.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::core::ptr::null_mut;
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
/// Opens an unused pseudo-terminal master device. Nanvix has no pseudo-terminal subsystem, so this
/// always fails.
///
/// # Parameters
///
/// - `flags`: File status flags for the new descriptor.
///
/// # Returns
///
/// `-1`, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it modifies global state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn posix_openpt(_flags: c_int) -> c_int {
    set_errno(ENOSYS);
    -1
}

///
/// # Description
///
/// Grants access to the slave pseudo-terminal associated with a master. Nanvix has no
/// pseudo-terminal subsystem, so this always fails.
///
/// # Parameters
///
/// - `fd`: File descriptor of the master pseudo-terminal.
///
/// # Returns
///
/// `-1`, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it modifies global state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn grantpt(_fd: c_int) -> c_int {
    set_errno(ENOSYS);
    -1
}

///
/// # Description
///
/// Unlocks the slave pseudo-terminal associated with a master. Nanvix has no pseudo-terminal
/// subsystem, so this always fails.
///
/// # Parameters
///
/// - `fd`: File descriptor of the master pseudo-terminal.
///
/// # Returns
///
/// `-1`, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it modifies global state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn unlockpt(_fd: c_int) -> c_int {
    set_errno(ENOSYS);
    -1
}

///
/// # Description
///
/// Returns the name of the slave pseudo-terminal associated with a master. Nanvix has no
/// pseudo-terminal subsystem, so this always fails.
///
/// # Parameters
///
/// - `fd`: File descriptor of the master pseudo-terminal.
///
/// # Returns
///
/// A null pointer, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it modifies global state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ptsname(_fd: c_int) -> *mut c_char {
    set_errno(ENOSYS);
    null_mut()
}

///
/// # Description
///
/// Reentrant variant of [`ptsname`]. Nanvix has no pseudo-terminal subsystem, so this always
/// fails.
///
/// # Parameters
///
/// - `fd`: File descriptor of the master pseudo-terminal.
/// - `buf`: Destination buffer for the slave name.
/// - `buflen`: Size of `buf`, in bytes.
///
/// # Returns
///
/// `ENOSYS`. Unlike most interfaces, `ptsname_r` returns the error number directly rather than
/// setting `errno`.
///
/// # Safety
///
/// This function is unsafe because it receives raw pointers from foreign callers.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ptsname_r(_fd: c_int, _buf: *mut c_char, _buflen: c_size_t) -> c_int {
    ENOSYS
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        grantpt,
        posix_openpt,
        ptsname,
        ptsname_r,
        unlockpt,
    };
    use crate::set_errno;
    use ::core::ptr::null_mut;
    use ::sysapi::{
        errno::{
            __errno_location,
            ENOSYS,
        },
        ffi::c_int,
    };

    fn get_errno() -> c_int {
        unsafe { *__errno_location() }
    }

    #[test]
    fn pty_stubs_fail_with_enosys() {
        set_errno(0);
        assert_eq!(unsafe { posix_openpt(0) }, -1);
        assert_eq!(get_errno(), ENOSYS);

        set_errno(0);
        assert_eq!(unsafe { grantpt(0) }, -1);
        assert_eq!(get_errno(), ENOSYS);

        set_errno(0);
        assert_eq!(unsafe { unlockpt(0) }, -1);
        assert_eq!(get_errno(), ENOSYS);

        set_errno(0);
        assert!(unsafe { ptsname(0) }.is_null());
        assert_eq!(get_errno(), ENOSYS);

        set_errno(0);
        assert_eq!(unsafe { ptsname_r(0, null_mut(), 0) }, ENOSYS);
        assert_eq!(get_errno(), 0);
    }
}

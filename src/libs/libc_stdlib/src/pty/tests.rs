// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::{
    unistd,
    unistd::bindings,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all UID/GID tests.
pub fn run() -> Result<(), Error> {
    test_getuid()?;
    test_getgid()?;
    test_geteuid()?;
    test_getegid()?;
    test_setuid()?;
    test_seteuid()?;
    test_setgid()?;
    test_setegid()?;
    Ok(())
}

/// Tests whether we can get the user ID of the calling process with `getuid()`.
fn test_getuid() -> Result<(), Error> {
    let _uid = unistd::getuid()?;
    Ok(())
}

/// Tests whether we can get the group ID of the calling process with `getgid()`.
fn test_getgid() -> Result<(), Error> {
    let _gid = unistd::getgid()?;
    Ok(())
}

/// Tests whether we can get the effective user ID of the calling process with `geteuid()`.
fn test_geteuid() -> Result<(), Error> {
    let _euid = unistd::geteuid()?;
    Ok(())
}

/// Tests whether we can get the effective group ID of the calling process with `getegid()`.
fn test_getegid() -> Result<(), Error> {
    let _egid = unistd::getegid()?;
    Ok(())
}

/// Tests whether `setuid()` can be used to set the real user ID of the calling process.
fn test_setuid() -> Result<(), Error> {
    let uid = unistd::getuid()?;
    let ret = unsafe { bindings::setuid::setuid(uid) };
    assert_eq!(ret, 0, "setuid() failed");
    Ok(())
}

/// Tests whether `seteuid()` can be used to set the effective user ID of the calling process.
fn test_seteuid() -> Result<(), Error> {
    let uid = unistd::getuid()?;
    let ret = unsafe { bindings::seteuid::seteuid(uid) };
    assert_eq!(ret, 0, "seteuid() failed");
    Ok(())
}

/// Tests whether `setgid()` can be used to set the real group ID of the calling process.
fn test_setgid() -> Result<(), Error> {
    let gid = unistd::getgid()?;
    let ret = unsafe { bindings::setgid::setgid(gid) };
    assert_eq!(ret, 0, "setgid() failed");
    Ok(())
}

/// Tests whether `setegid()` can be used to set the effective group ID of the calling process.
fn test_setegid() -> Result<(), Error> {
    let gid = unistd::getgid()?;
    let ret = unsafe { bindings::setegid::setegid(gid) };
    assert_eq!(ret, 0, "setegid() failed");
    Ok(())
}

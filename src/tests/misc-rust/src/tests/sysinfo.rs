// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::{
    sys::utsname,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all system information tests.
pub fn run() -> Result<(), Error> {
    test_uname()?;
    test_gethostname()?;
    Ok(())
}

/// Tests whether we can get system information with `uname()`.
fn test_uname() -> Result<(), Error> {
    let info = utsname::uname()?;

    // Check if the system information fields are not empty.
    assert!(info.sysname[0] != 0, "uname(): sysname must not be empty");
    assert!(info.nodename[0] != 0, "uname(): nodename must not be empty");
    assert!(info.release[0] != 0, "uname(): release must not be empty");
    assert!(info.version[0] != 0, "uname(): version must not be empty");
    assert!(info.machine[0] != 0, "uname(): machine must not be empty");

    Ok(())
}

/// Tests whether `gethostname()` works.
fn test_gethostname() -> Result<(), Error> {
    let hostname = unistd::gethostname();
    assert!(!hostname.is_empty(), "gethostname(): hostname must not be empty");
    Ok(())
}

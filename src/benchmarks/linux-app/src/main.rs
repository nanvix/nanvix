// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::{
    fcntl,
    time::{
        self,
        timespec,
        CLOCK_MONOTONIC,
    },
    unistd,
    venv,
    venv::VirtualEnvironmentIdentifier,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let env: VirtualEnvironmentIdentifier = venv::join(VirtualEnvironmentIdentifier::NEW)?;
    ::nvx::log!("joined environment {:?}", env);

    let mut res: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_getres(CLOCK_MONOTONIC, &mut res) {
        0 => {
            ::nvx::log!("clock resolution: {}s {}ns", { res.tv_sec }, { res.tv_nsec });
        },
        errno => {
            panic!("failed to get clock resolution: {:?}", errno);
        },
    }

    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_gettime(CLOCK_MONOTONIC, &mut tp) {
        0 => {
            ::nvx::log!("clock time: {}s {}ns", { tp.tv_sec }, { tp.tv_nsec });
        },
        errno => {
            panic!("failed to get clock time: {:?}", errno);
        },
    }

    // Create a file named `foo.txt`.
    let fd: i32 = match fcntl::openat(
        fcntl::AT_FDCWD,
        "foo.tmp",
        fcntl::O_CREAT | fcntl::O_RDONLY,
        fcntl::S_IRUSR | fcntl::S_IWUSR,
    ) {
        fd if fd >= 0 => {
            ::nvx::log!("opened file foo.txt with fd {}", fd);
            fd
        },
        errno => {
            panic!("failed to open file foo.txt: {:?}", errno);
        },
    };

    // Close file.
    match unistd::close(fd) {
        0 => {
            ::nvx::log!("closed file foo.txt");
        },
        errno => {
            panic!("failed to close file foo.txt: {:?}", errno);
        },
    }

    // Rename `foo.txt` to `bar.txt`.
    match fcntl::renameat(fcntl::AT_FDCWD, "foo.tmp", fcntl::AT_FDCWD, "bar.tmp") {
        0 => {
            ::nvx::log!("renamed file foo.txt to bar.txt");
        },
        errno => {
            panic!("failed to rename file foo.txt to bar.txt: {:?}", errno);
        },
    }

    // Unlink file named `foo.txt`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "bar.tmp", 0) {
        0 => {
            ::nvx::log!("unlinked file foo.txt");
        },
        errno => {
            panic!("failed to unlink file foo.txt: {:?}", errno);
        },
    }

    venv::leave(env)?;
    ::nvx::log!("left environment {:?}", env);

    Ok(())
}

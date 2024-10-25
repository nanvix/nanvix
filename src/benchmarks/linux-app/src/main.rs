// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::{
    fcntl,
    sys::{
        self,
        stat::stat,
    },
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

    // Create a file named `foo.tmp`.
    let fd: i32 = match fcntl::openat(
        fcntl::AT_FDCWD,
        "foo.tmp",
        fcntl::O_CREAT | fcntl::O_RDONLY,
        fcntl::S_IRUSR | fcntl::S_IWUSR,
    ) {
        fd if fd >= 0 => {
            ::nvx::log!("opened file foo.tmp with fd {}", fd);
            fd
        },
        errno => {
            panic!("failed to open file foo.tmp: {:?}", errno);
        },
    };

    // Synchronize a file's in-core state with storage device.
    match unistd::fdatasync(fd) {
        0 => {
            ::nvx::log!("synchronized file foo.tmp with storage device");
        },
        errno => {
            panic!("failed to synchronize file foo.tmp with storage device: {:?}", errno);
        },
    }

    // Close file.
    match unistd::close(fd) {
        0 => {
            ::nvx::log!("closed file foo.tmp");
        },
        errno => {
            panic!("failed to close file foo.tmp: {:?}", errno);
        },
    }

    // Rename `foo.tmp` to `bar.tmp`.
    match fcntl::renameat(fcntl::AT_FDCWD, "foo.tmp", fcntl::AT_FDCWD, "bar.tmp") {
        0 => {
            ::nvx::log!("renamed file foo.tmp to bar.tmp");
        },
        errno => {
            panic!("failed to rename file foo.tmp to bar.tmp: {:?}", errno);
        },
    }

    // Get status of file named `bar.tmp`.
    let mut st: stat = stat::default();
    match sys::stat::fstatat(fcntl::AT_FDCWD, "bar.tmp", &mut st, 0) {
        0 => {
            ::nvx::log!("got status of file bar.tmp");
            ::nvx::log!("file statistics:");
            ::nvx::log!("  st_dev: {}", { st.st_dev });
            ::nvx::log!("  st_ino: {}", { st.st_ino });
            ::nvx::log!("  st_mode: {}", { st.st_mode });
            ::nvx::log!("  st_nlink: {}", { st.st_nlink });
            ::nvx::log!("  st_uid: {}", { st.st_uid });
            ::nvx::log!("  st_gid: {}", { st.st_gid });
            ::nvx::log!("  st_rdev: {}", { st.st_rdev });
            ::nvx::log!("  st_size: {}", { st.st_size });
            ::nvx::log!("  st_blksize: {}", { st.st_blksize });
            ::nvx::log!("  st_blocks: {}", { st.st_blocks });
            ::nvx::log!("  st_atime: {}s {}ns", { st.st_atim.tv_sec }, { st.st_atim.tv_nsec });
            ::nvx::log!("  st_mtime: {}s {}ns", { st.st_mtim.tv_sec }, { st.st_mtim.tv_nsec });
            ::nvx::log!("  st_ctime: {}s {}ns", { st.st_ctim.tv_sec }, { st.st_ctim.tv_nsec });
        },
        errno => {
            panic!("failed to get status of file bar.tmp: {:?}", errno);
        },
    }

    // Unlink file named `foo.tmp`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "bar.tmp", 0) {
        0 => {
            ::nvx::log!("unlinked file foo.tmp");
        },
        errno => {
            panic!("failed to unlink file foo.tmp: {:?}", errno);
        },
    }

    venv::leave(env)?;
    ::nvx::log!("left environment {:?}", env);

    Ok(())
}

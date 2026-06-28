// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::Error;
use ::sysapi::fcntl::{
    atflags::AT_FDCWD,
    file_access_mode::O_RDONLY,
    file_control_request::{
        F_DUPFD,
        F_SETFD,
    },
    file_descriptor_flags::FD_CLOEXEC,
};
use ::syscall::{
    fcntl,
    unistd::do_execv,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the target program in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";
/// Maximum length of a decimal file descriptor string plus NUL terminator.
const FD_ARG_CAPACITY: usize = 12;
/// Minimum descriptor number used for the close-on-exec probe.
const CLOEXEC_PROBE_MIN_FD: i32 = 32;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Formats `fd` as a decimal string in `buffer` and returns the initialized prefix.
fn format_fd_arg(fd: i32, buffer: &mut [u8; FD_ARG_CAPACITY]) -> &str {
    let mut value: u32 = fd as u32;
    let mut digits: [u8; FD_ARG_CAPACITY] = [0; FD_ARG_CAPACITY];
    let mut len: usize = 0;
    loop {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
        if value == 0 {
            break;
        }
    }
    for index in 0..len {
        buffer[index] = digits[len - 1 - index];
    }
    // SAFETY: all bytes written above are ASCII decimal digits.
    unsafe { ::core::str::from_utf8_unchecked(&buffer[..len]) }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the `execv()` test. It replaces its own image with the target program loaded
/// from the mounted ramfs. On success this does not return: the target program runs in place and
/// writes the success sentinel. Reaching the code after [`do_execv`] therefore indicates failure.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let fd: i32 = fcntl::openat(AT_FDCWD, TARGET_PATH, O_RDONLY, 0)?;
    let cloexec_fd: i32 = fcntl::fcntl(fd, F_DUPFD, Some(CLOEXEC_PROBE_MIN_FD))?;
    ::syscall::unistd::close(fd)?;
    fcntl::fcntl(cloexec_fd, F_SETFD, Some(FD_CLOEXEC))?;
    let mut fd_arg_buffer: [u8; FD_ARG_CAPACITY] = [0; FD_ARG_CAPACITY];
    let fd_arg: &str = format_fd_arg(cloexec_fd, &mut fd_arg_buffer);

    // Replace this image with the target program. The argument vector's first element is the
    // conventional program name.
    let error: Error = do_execv(TARGET_PATH, &["target", fd_arg], &[]);

    // Only reached if execv() failed; surface the error so the test fails with a non-zero status.
    ::syslog::error!("execv({TARGET_PATH}) failed: {error:?}");
    Err(error)
}

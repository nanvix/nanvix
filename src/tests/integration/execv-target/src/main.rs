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

use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_control_request::F_GETFD,
    unistd::STDOUT_FILENO,
};
use ::syscall::{
    fcntl,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns the second command-line argument, which carries the descriptor number opened by the
/// image that called `execv()`.
fn inherited_fd_arg() -> Result<&'static str, Error> {
    let argc: i32 = ::nvx_crt0::ARGC.load(Ordering::SeqCst);
    if argc < 2 {
        ::syslog::error!("execv-target: missing inherited fd argument (argc={argc})");
        return Err(Error::new(ErrorCode::InvalidArgument, "missing inherited fd argument"));
    }
    let argv: *const *const u8 = ::nvx_crt0::ARGV.load(Ordering::SeqCst) as *const *const u8;
    if argv.is_null() {
        ::syslog::error!("execv-target: argv is null");
        return Err(Error::new(ErrorCode::InvalidArgument, "argv is null"));
    }
    let arg: *const u8 = unsafe { *argv.add(1) };
    if arg.is_null() {
        ::syslog::error!("execv-target: fd argument is null");
        return Err(Error::new(ErrorCode::InvalidArgument, "fd argument is null"));
    }
    unsafe { ::core::ffi::CStr::from_ptr(arg.cast()) }
        .to_str()
        .map_err(|_| {
            ::syslog::error!("execv-target: fd argument is not UTF-8");
            Error::new(ErrorCode::InvalidArgument, "fd argument is not UTF-8")
        })
}

/// Parses a non-negative decimal file descriptor.
fn parse_fd(arg: &str) -> Result<i32, Error> {
    let mut value: i32 = 0;
    if arg.is_empty() {
        ::syslog::error!("execv-target: empty fd argument");
        return Err(Error::new(ErrorCode::InvalidArgument, "empty fd argument"));
    }
    for byte in arg.bytes() {
        if !byte.is_ascii_digit() {
            ::syslog::error!("execv-target: fd argument is not decimal (arg={arg:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, "fd argument is not decimal"));
        }
        let digit: i32 = i32::from(byte - b'0');
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(digit))
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "fd argument overflowed"))?;
    }
    Ok(value)
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the target program loaded by the `execv()` test. It writes the success sentinel
/// to the standard output and exits, demonstrating that `execv()` replaced the calling image with
/// this distinct program.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let fd: i32 = parse_fd(inherited_fd_arg()?)?;
    match fcntl::fcntl(fd, F_GETFD, None) {
        Ok(_) => {
            ::syslog::error!("execv-target: FD_CLOEXEC descriptor survived (fd={fd})");
            unistd::write(STDOUT_FILENO, "failed".as_bytes())?;
            return Err(Error::new(
                ErrorCode::InvalidFileDescriptor,
                "FD_CLOEXEC descriptor survived",
            ));
        },
        Err(error)
            if matches!(error.code, ErrorCode::InvalidFileDescriptor | ErrorCode::BadFile) => {},
        Err(error) => {
            ::syslog::error!("execv-target: fcntl(F_GETFD) failed unexpectedly (error={error:?})");
            return Err(error);
        },
    }

    let magic_string: &[u8] = "ok".as_bytes();
    unistd::write(STDOUT_FILENO, magic_string)?;

    Ok(())
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! System call benchmark guest program.
//!
//! Serves batched system call requests from the host benchmark driver (`nanvix-bench`) over the
//! console, following the wire protocol of `syscall-bench-common`. For each request, the program
//! issues the requested system call the requested number of times back to back and then
//! acknowledges the batch, so that the host can time it.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_access_mode::{
        O_RDONLY,
        O_RDWR,
    },
    ffi::{
        c_int,
        c_void,
    },
    sys_ioctl::{
        TCGETS,
        TCSETS,
    },
    sys_stat,
    termios::{
        Termios,
        ECHO,
        ECHOE,
        ECHOK,
        ICANON,
        ICRNL,
        IEXTEN,
        ISIG,
        IXON,
        ONLCR,
        OPOST,
        VMIN,
        VTIME,
    },
    time::{
        clock_ids::CLOCK_MONOTONIC,
        timespec,
    },
    unistd::{
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::{
    errno::__errno_location,
    fcntl,
    sched,
    sys,
    time,
    unistd::{
        self,
        bindings::getpgrp::getpgrp,
    },
};
use ::syscall_bench_common::{
    decode_request,
    SyscallOp,
    ACK_ERR,
    ACK_OK,
    REQUEST_SIZE,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the device on which file system calls operate.
///
/// The null device lives in the synthetic device namespace of the VFS daemon, so calls on it
/// exercise the daemon round trip without touching any backing store.
const NULL_DEVICE_PATH: &str = "/dev/null";

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Report the outcome of the setup, so that the host fails fast instead of timing out.
    let null_fd: c_int = match setup() {
        Ok(fd) => fd,
        Err(error) => {
            let _ = send_ack(ACK_ERR);
            return Err(error);
        },
    };
    send_ack(ACK_OK)?;

    // Serve requests until the host closes the console.
    let mut frame: [u8; REQUEST_SIZE] = [0; REQUEST_SIZE];
    while read_request(&mut frame)? {
        let ack: u8 = match decode_request(&frame) {
            Some((op, count)) => match run_batch(op, count, null_fd) {
                Ok(()) => ACK_OK,
                Err(_) => ACK_ERR,
            },
            None => ACK_ERR,
        };
        send_ack(ack)?;
    }

    unistd::close(null_fd)
}

//==================================================================================================
// Protocol
//==================================================================================================

/// Prepares the program to serve requests and returns a descriptor open on the null device.
fn setup() -> Result<c_int, Error> {
    configure_raw_console(STDIN_FILENO)?;
    fcntl::open(NULL_DEVICE_PATH, O_RDWR, 0)
}

/// Switches the console to raw mode, so that request frames reach the program byte for byte.
///
/// The VFS daemon applies a line discipline to the console that, by default, buffers input until
/// a newline, echoes it back, and turns control bytes into signals. Any of these would corrupt the
/// binary request frames and acknowledgements.
fn configure_raw_console(fd: c_int) -> Result<(), Error> {
    let mut termios: Termios = Termios::console_default();

    // SAFETY: the argument points to a valid, writable `Termios` for the duration of the call.
    unsafe { sys::ioctl::ioctl(fd, TCGETS, (&raw mut termios).cast::<c_void>()) }?;

    termios.c_iflag &= !(ICRNL | IXON);
    termios.c_oflag &= !(OPOST | ONLCR);
    termios.c_lflag &= !(ISIG | ICANON | ECHO | ECHOE | ECHOK | IEXTEN);
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    // SAFETY: the argument points to a valid, readable `Termios` for the duration of the call.
    unsafe { sys::ioctl::ioctl(fd, TCSETS, (&raw mut termios).cast::<c_void>()) }?;

    Ok(())
}

/// Reads the next request frame from the console.
///
/// Returns `Ok(false)` when the host closes the console before sending another frame.
fn read_request(frame: &mut [u8; REQUEST_SIZE]) -> Result<bool, Error> {
    let mut offset: usize = 0;
    while offset < frame.len() {
        match unistd::read(STDIN_FILENO, &mut frame[offset..])? {
            0 if offset == 0 => return Ok(false),
            0 => return Err(Error::new(ErrorCode::InvalidMessage, "truncated request frame")),
            count => offset += count as usize,
        }
    }

    Ok(true)
}

/// Sends a one-byte acknowledgement to the host.
fn send_ack(ack: u8) -> Result<(), Error> {
    unistd::write(STDOUT_FILENO, &[ack])?;
    Ok(())
}

//==================================================================================================
// System Calls
//==================================================================================================

/// Issues `op` `count` times back to back, stopping at the first error.
fn run_batch(op: SyscallOp, count: u32, null_fd: c_int) -> Result<(), Error> {
    match op {
        SyscallOp::Noop => Ok(()),
        SyscallOp::GetPpid => repeat(count, || unistd::getppid().map(|_| ())),
        SyscallOp::ClockGettime => {
            let mut now: timespec = timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            repeat(count, || time::clock_gettime(CLOCK_MONOTONIC, &mut Some(&mut now)))
        },
        SyscallOp::SchedYield => repeat(count, sched::sched_yield),
        SyscallOp::GetPgrp => repeat(count, checked_getpgrp),
        SyscallOp::Fstat => {
            let mut status: sys_stat::stat = sys_stat::stat::default();
            repeat(count, || sys::stat::fstat(null_fd, &mut status))
        },
        SyscallOp::Stat => {
            let mut status: sys_stat::stat = sys_stat::stat::default();
            repeat(count, || sys::stat::stat(NULL_DEVICE_PATH, &mut status))
        },
        SyscallOp::Read => {
            let mut buffer: [u8; 1] = [0; 1];
            repeat(count, || unistd::read(null_fd, &mut buffer).map(|_| ()))
        },
        SyscallOp::Write => {
            let buffer: [u8; 1] = [0; 1];
            repeat(count, || unistd::write(null_fd, &buffer).map(|_| ()))
        },
        SyscallOp::OpenClose => repeat(count, || {
            let fd: c_int = fcntl::open(NULL_DEVICE_PATH, O_RDONLY, 0)?;
            unistd::close(fd)
        }),
    }
}

/// Calls `syscall` `count` times, stopping at the first error.
fn repeat(count: u32, mut syscall: impl FnMut() -> Result<(), Error>) -> Result<(), Error> {
    for _ in 0..count {
        syscall()?;
    }

    Ok(())
}

/// Issues `getpgrp()` through its C library entry point, which reports failures through `errno`.
fn checked_getpgrp() -> Result<(), Error> {
    if getpgrp() >= 0 {
        return Ok(());
    }

    // SAFETY: `__errno_location()` returns a valid pointer to the `errno` of the calling thread.
    let errno: c_int = unsafe { *__errno_location() };
    Err(Error::new(ErrorCode::try_from(errno)?, "getpgrp() failed"))
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! VFS benchmark guest program.
//!
//! Serves file system requests from the host benchmark driver (`nanvix-bench`) over the console,
//! following the wire protocol of `vfs-bench-common`. For each request, the program runs one VFS
//! operation and acknowledges it, so that the host can time it.
//!
//! The program is an ordinary process of a standalone image. It never mounts or otherwise touches
//! the benchmark image: every operation goes through the system call interface and is served by the
//! VFS daemon, which owns the image.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::alloc::boxed::Box;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::{
        atflags::{
            AT_FDCWD,
            AT_REMOVEDIR,
        },
        file_access_mode::{
            O_RDONLY,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
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
    sys_types::mode_t,
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
    unistd::{
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::{
    dirent::{
        self,
        DirectoryStream,
    },
    fcntl,
    sys,
    unistd,
};
use ::vfs_bench_common::{
    VfsOp,
    ACK_ERR,
    ACK_OK,
    ACK_READ_ONLY,
    MAX_PATH_LEN,
    SCRATCH_DIR,
    SCRATCH_FILE,
    SCRATCH_RENAMED_FILE,
    SCRATCH_SUBDIR,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Read buffer size for sequential reads.
const READ_BUF_SIZE: usize = 4096;

/// Write payload size for sequential writes.
const WRITE_SIZE: usize = 4096;

/// Byte that the sequential write fills its payload with.
const WRITE_PATTERN: u8 = 0x42;

/// Permissions of the file that the write operations create.
const FILE_MODE: mode_t = 0o644;

/// Permissions of the directory that the directory creation operation creates.
const DIRECTORY_MODE: mode_t = 0o700;

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Report the outcome of the setup, so that the host fails fast instead of timing out.
    if let Err(error) = setup() {
        let _ = send_ack(ACK_ERR);
        return Err(error);
    }
    send_ack(ACK_OK)?;

    // Serve requests until the host closes the console.
    let mut header: [u8; 2] = [0; 2];
    let mut path_buffer: [u8; MAX_PATH_LEN] = [0; MAX_PATH_LEN];
    while read_exact(&mut header)? {
        let opcode: u8 = header[0];
        let path_len: usize = usize::from(header[1]);
        if !read_exact(&mut path_buffer[..path_len])? {
            return Err(Error::new(ErrorCode::InvalidMessage, "truncated request"));
        }

        let ack: u8 = match handle_request(opcode, &path_buffer[..path_len]) {
            Ok(()) => ACK_OK,
            Err(error) => failure_ack(&error),
        };
        send_ack(ack)?;
    }

    Ok(())
}

//==================================================================================================
// Protocol
//==================================================================================================

/// Prepares the program to serve requests.
fn setup() -> Result<(), Error> {
    configure_raw_console(STDIN_FILENO)?;

    // The scratch area exists only in the benchmark image, so this also checks that the VFS daemon
    // serves that image at the root of the file system.
    let mut status: sys_stat::stat = sys_stat::stat::default();
    sys::stat::stat(SCRATCH_DIR, &mut status)
}

/// Switches the console to raw mode, so that requests reach the program byte for byte.
///
/// The VFS daemon applies a line discipline to the console that, by default, buffers input until
/// a newline, echoes it back, and turns control bytes into signals. Any of these would corrupt the
/// binary requests and acknowledgements.
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

/// Fills `buffer` with bytes read from the console.
///
/// Returns `Ok(false)` when the host closes the console before sending the first byte.
fn read_exact(buffer: &mut [u8]) -> Result<bool, Error> {
    let mut offset: usize = 0;
    while offset < buffer.len() {
        match unistd::read(STDIN_FILENO, &mut buffer[offset..])? {
            0 if offset == 0 => return Ok(false),
            0 => return Err(Error::new(ErrorCode::InvalidMessage, "truncated request")),
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

/// Returns the acknowledgement that reports a failed operation to the host.
///
/// `vfsd` rejects a write to a read-only mount with a permission error. The host needs to tell that
/// rejection apart from any other failure, to check that it got the mount mode that it asked for.
fn failure_ack(error: &Error) -> u8 {
    match error.code {
        ErrorCode::PermissionDenied | ErrorCode::ReadOnlyFileSystem => ACK_READ_ONLY,
        _ => ACK_ERR,
    }
}

/// Decodes a request and runs the operation that it asks for.
fn handle_request(opcode: u8, path: &[u8]) -> Result<(), Error> {
    let op: VfsOp = VfsOp::from_u8(opcode)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "unknown opcode"))?;
    let path: &str = core::str::from_utf8(path)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "path is not valid UTF-8"))?;

    run_operation(op, path)
}

//==================================================================================================
// Operations
//==================================================================================================

/// Runs `op` once. Operations that read the image act on `path`; operations that write to it work
/// in the scratch area and remove what they create.
fn run_operation(op: VfsOp, path: &str) -> Result<(), Error> {
    match op {
        VfsOp::Noop => Ok(()),
        VfsOp::Stat => {
            let mut status: sys_stat::stat = sys_stat::stat::default();
            sys::stat::stat(path, &mut status)
        },
        VfsOp::OpenClose => with_open_file(path, O_RDONLY, 0, |_fd| Ok(())),
        VfsOp::Readdir => read_directory(path),
        VfsOp::SeqRead => with_open_file(path, O_RDONLY, 0, read_to_end),
        VfsOp::CreateUnlink => {
            create_scratch_file(|_fd| Ok(()))?;
            unistd::unlink(SCRATCH_FILE)
        },
        VfsOp::MkdirRmdir => {
            sys::stat::mkdir(SCRATCH_SUBDIR, DIRECTORY_MODE)?;
            fcntl::unlinkat(AT_FDCWD, SCRATCH_SUBDIR, AT_REMOVEDIR)
        },
        VfsOp::Rename => {
            create_scratch_file(|_fd| Ok(()))?;
            fcntl::rename(SCRATCH_FILE, SCRATCH_RENAMED_FILE)?;
            unistd::unlink(SCRATCH_RENAMED_FILE)
        },
        VfsOp::SeqWrite => {
            create_scratch_file(|fd| {
                write_all(fd, &[WRITE_PATTERN; WRITE_SIZE])?;
                unistd::fsync(fd)
            })?;
            unistd::unlink(SCRATCH_FILE)
        },
    }
}

/// Opens `path`, runs `body` on the resulting descriptor, and closes the descriptor.
///
/// The descriptor is closed even if `body` fails, in which case the error of `body` is returned.
fn with_open_file(
    path: &str,
    flags: c_int,
    mode: mode_t,
    body: impl FnOnce(c_int) -> Result<(), Error>,
) -> Result<(), Error> {
    let fd: c_int = fcntl::open(path, flags, mode)?;
    let result: Result<(), Error> = body(fd);
    let closed: Result<(), Error> = unistd::close(fd);

    result?;
    closed
}

/// Creates the scratch file, runs `body` on it, and closes it.
fn create_scratch_file(body: impl FnOnce(c_int) -> Result<(), Error>) -> Result<(), Error> {
    with_open_file(SCRATCH_FILE, O_WRONLY | O_CREAT | O_TRUNC, FILE_MODE, body)
}

/// Reads every entry of the directory at `path`.
fn read_directory(path: &str) -> Result<(), Error> {
    let mut directory: Box<DirectoryStream> = dirent::opendir(path)?;

    // Close the directory even if reading it fails.
    let listing: Result<(), Error> = loop {
        match dirent::readdir(&mut directory) {
            Ok(Some(_entry)) => {},
            Ok(None) => break Ok(()),
            Err(error) => break Err(error),
        }
    };
    let closed: Result<(), Error> = dirent::closedir(&mut directory);

    listing?;
    closed
}

/// Reads the file open on `fd` from start to end.
fn read_to_end(fd: c_int) -> Result<(), Error> {
    let mut buffer: [u8; READ_BUF_SIZE] = [0; READ_BUF_SIZE];
    while unistd::read(fd, &mut buffer)? != 0 {}

    Ok(())
}

/// Writes the whole of `data` to the file open on `fd`.
fn write_all(fd: c_int, mut data: &[u8]) -> Result<(), Error> {
    while !data.is_empty() {
        match unistd::write(fd, data)? {
            0 => return Err(Error::new(ErrorCode::TryAgain, "write made no progress")),
            written => data = &data[written as usize..],
        }
    }

    Ok(())
}

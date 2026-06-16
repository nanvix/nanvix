// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm,
        pm,
    },
    mm::Address,
    pm::Capability,
};
use ::sysapi::unistd::{
    STDIN_FILENO,
    STDOUT_FILENO,
};
use ::syscall::unistd;
use ::vfs_bench_common::{
    VfsOp,
    ACK_ERR,
    ACK_OK,
    MAX_PATH_LEN,
    MOUNT_READONLY,
    MOUNT_WRITABLE,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

/// Read buffer size for sequential reads.
const READ_BUF_SIZE: usize = 4096;

/// Write payload size for sequential writes.
const WRITE_SIZE: usize = 4096;

//==================================================================================================
// Helpers
//==================================================================================================

/// Converts a [`vfs::Fat32Error`] to an [`Error`].
fn fat_err(e: vfs::Fat32Error, reason: &'static str) -> Error {
    Error::new(ErrorCode::from(e), reason)
}

/// Reads exactly `buf.len()` bytes from a file descriptor, looping on short reads.
fn read_exact(fd: i32, buf: &mut [u8]) -> Result<(), Error> {
    let mut offset: usize = 0;
    while offset < buf.len() {
        match unistd::read(fd, &mut buf[offset..]) {
            Ok(0) => return Err(Error::new(ErrorCode::TryAgain, "unexpected EOF")),
            Ok(n) => offset += n as usize,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Initialize VFS.
    vfs::init().map_err(|e| fat_err(e, "vfs init failed"))?;

    // Read mount configuration byte from host (writable or read-only).
    let mut config_buf: [u8; 1] = [0u8; 1];
    read_exact(STDIN_FILENO, &mut config_buf)?;
    let readonly: bool = match config_buf[0] {
        MOUNT_WRITABLE => false,
        MOUNT_READONLY => true,
        _unknown => {
            let _ = unistd::write(STDOUT_FILENO, &[ACK_ERR]);
            return Err(Error::new(ErrorCode::InvalidArgument, "unknown mount config byte"));
        },
    };

    // Mount RAMFS image from MMIO.
    if let Err(e) = mount_ramfs(readonly) {
        let _ = unistd::write(STDOUT_FILENO, &[ACK_ERR]);
        return Err(e);
    }

    // Acknowledge mount complete.
    let _ = unistd::write(STDOUT_FILENO, &[ACK_OK]);

    // Enter command loop: read [opcode][path_len][path] from stdin, execute, write ack to stdout.
    let mut cmd_buf: [u8; 1] = [0u8; 1];
    let mut path_len_buf: [u8; 1] = [0u8; 1];
    let mut path_buf: [u8; MAX_PATH_LEN] = [0u8; MAX_PATH_LEN];

    loop {
        // Read one opcode byte from stdin. EOF means the host is done.
        match unistd::read(STDIN_FILENO, &mut cmd_buf) {
            Err(_) | Ok(0) => break,
            Ok(_) => {},
        }

        // Read path length and path bytes.
        if read_exact(STDIN_FILENO, &mut path_len_buf).is_err() {
            break;
        }
        let path_len: usize = path_len_buf[0] as usize;
        if path_len > 0 && read_exact(STDIN_FILENO, &mut path_buf[..path_len]).is_err() {
            break;
        }
        let path: &str = match core::str::from_utf8(&path_buf[..path_len]) {
            Ok(s) => s,
            Err(_) => {
                let _ = unistd::write(STDOUT_FILENO, &[ACK_ERR]);
                continue;
            },
        };

        let result: Result<(), Error> = match VfsOp::from_u8(cmd_buf[0]) {
            Some(VfsOp::Noop) => Ok(()),
            Some(VfsOp::Stat) => do_stat(path),
            Some(VfsOp::OpenClose) => do_open_close(path),
            Some(VfsOp::Readdir) => do_readdir(path),
            Some(VfsOp::CreateScratch) => do_create_scratch(),
            Some(VfsOp::SeqRead) => do_seq_read(path),
            Some(VfsOp::CreateUnlink) => do_create_unlink(),
            Some(VfsOp::MkdirRmdir) => do_mkdir_rmdir(),
            Some(VfsOp::Rename) => do_rename(),
            Some(VfsOp::SeqWrite) => do_seq_write(),
            None => Err(Error::new(ErrorCode::InvalidArgument, "unknown opcode")),
        };

        // Write ack byte.
        let ack: [u8; 1] = if result.is_ok() { [ACK_OK] } else { [ACK_ERR] };
        let _ = unistd::write(STDOUT_FILENO, &ack);
    }

    Ok(())
}

//==================================================================================================
// RAMFS Setup
//==================================================================================================

/// Mounts the RAMFS image provided via the MMIO region at `/`.
///
/// On MicroVM the MMIO region is mapped read-write in guest physical memory,
/// so we mount it in place without copying to the heap.  The MMIO allocation
/// is intentionally kept alive for the lifetime of the process.
///
/// # Parameters
///
/// - `readonly`: If `true`, the mount is registered as read-only, enabling
///   negative caching and blocking write operations on the mount.
fn mount_ramfs(readonly: bool) -> Result<(), Error> {
    // Acquire IO management capability.
    pm::__kcall_capctl(Capability::IoManagement, true)?;

    // Attach RAMFS MMIO region.
    mm::__kcall_mmio_alloc(RAMFS_MMIO_TAG)?;

    // Get region info.
    let info: ::sys::mm::MmioRegionInfo = mm::__kcall_mmio_info(RAMFS_MMIO_TAG)?;
    let total_size: usize = info.size();

    // Mount the FAT image directly from the MMIO region.
    unsafe {
        vfs::mount_image("/", info.base().as_ptr() as *mut u8, total_size, readonly)
            .map_err(|e| fat_err(e, "mount ramfs failed"))?;
    }

    // Release IO management capability but keep the MMIO region allocated —
    // the mounted FAT references it for the process lifetime.
    pm::__kcall_capctl(Capability::IoManagement, false)?;

    Ok(())
}

//==================================================================================================
// Operation Handlers
//==================================================================================================

/// `stat()` on the given file path.
fn do_stat(path: &str) -> Result<(), Error> {
    let _ = vfs::stat(path).map_err(|e| fat_err(e, "stat failed"))?;
    Ok(())
}

/// `open()` + `close()` on the given file path.
fn do_open_close(path: &str) -> Result<(), Error> {
    let _file = vfs::open(path).map_err(|e| fat_err(e, "open failed"))?;
    Ok(())
}

/// `read_dir()` on the given directory path.
fn do_readdir(path: &str) -> Result<(), Error> {
    let _entries = vfs::read_dir(path).map_err(|e| fat_err(e, "read_dir failed"))?;
    Ok(())
}

/// `create_mount()` + `unmount()` cycle for a scratch FAT directory.
fn do_create_scratch() -> Result<(), Error> {
    vfs::create_mount("/scratch_bench", 256 * 1024)
        .map_err(|e| fat_err(e, "create scratch mount failed"))?;
    vfs::unmount("/scratch_bench").map_err(|e| fat_err(e, "unmount scratch failed"))?;
    Ok(())
}

/// Sequential read of the file at the given path.
fn do_seq_read(path: &str) -> Result<(), Error> {
    let mut file = vfs::open(path).map_err(|e| fat_err(e, "open for read failed"))?;
    let mut buf: [u8; READ_BUF_SIZE] = [0u8; READ_BUF_SIZE];
    loop {
        let n: usize = file.read(&mut buf).map_err(|e| fat_err(e, "read failed"))?;
        if n == 0 {
            break;
        }
    }
    Ok(())
}

/// File creation + deletion cycle.
fn do_create_unlink() -> Result<(), Error> {
    vfs::create_mount("/scratch3", 256 * 1024).map_err(|e| fat_err(e, "create scratch3 failed"))?;
    let file = vfs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("/scratch3/tmpfile.dat")
        .map_err(|e| fat_err(e, "create file failed"))?;
    drop(file);
    vfs::unlink("/scratch3/tmpfile.dat").map_err(|e| fat_err(e, "unlink failed"))?;
    vfs::unmount("/scratch3").map_err(|e| fat_err(e, "unmount scratch3 failed"))?;
    Ok(())
}

/// `mkdir()` + `rmdir()` cycle.
fn do_mkdir_rmdir() -> Result<(), Error> {
    vfs::create_mount("/scratch2", 256 * 1024).map_err(|e| fat_err(e, "create scratch2 failed"))?;
    vfs::mkdir("/scratch2/benchdir").map_err(|e| fat_err(e, "mkdir failed"))?;
    vfs::rmdir("/scratch2/benchdir").map_err(|e| fat_err(e, "rmdir failed"))?;
    vfs::unmount("/scratch2").map_err(|e| fat_err(e, "unmount scratch2 failed"))?;
    Ok(())
}

/// `rename()` on a file within a scratch mount.
fn do_rename() -> Result<(), Error> {
    vfs::create_mount("/scratch4", 256 * 1024).map_err(|e| fat_err(e, "create scratch4 failed"))?;
    let file = vfs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("/scratch4/rename_src.dat")
        .map_err(|e| fat_err(e, "create rename_src failed"))?;
    drop(file);
    vfs::rename("/scratch4/rename_src.dat", "/scratch4/rename_dst.dat")
        .map_err(|e| fat_err(e, "rename failed"))?;
    vfs::unmount("/scratch4").map_err(|e| fat_err(e, "unmount scratch4 failed"))?;
    Ok(())
}

/// Sequential write of a 4 KiB payload to a scratch file, then clean up.
fn do_seq_write() -> Result<(), Error> {
    vfs::create_mount("/scratch", 512 * 1024).map_err(|e| fat_err(e, "create scratch failed"))?;
    let payload: [u8; WRITE_SIZE] = [0x42u8; WRITE_SIZE];
    let mut file = vfs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("/scratch/bench_write.dat")
        .map_err(|e| fat_err(e, "open for write failed"))?;
    file.write(&payload)
        .map_err(|e| fat_err(e, "write failed"))?;
    file.flush().map_err(|e| fat_err(e, "flush failed"))?;
    drop(file);
    vfs::unmount("/scratch").map_err(|e| fat_err(e, "unmount scratch failed"))?;
    Ok(())
}

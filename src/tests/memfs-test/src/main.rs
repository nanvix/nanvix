// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Validation test for the memfs POSIX interception layer.
//!
//! This test validates that the in-memory FAT32 filesystem correctly intercepts
//! standard POSIX file operations (`open`, `read`, `fstat`, `lseek`, `close`).
//! It loads a FAT32 image via the RAMFS MMIO region, mounts it using
//! `memfs_init_from_ramfs()`, and then performs file I/O through the standard C
//! library calls which should be transparently served from memory.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![allow(clippy::needless_return)]
#![allow(dead_code)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// External C Functions (POSIX bindings from syscall/posix)
//==================================================================================================

extern "C" {
    fn open(path: *const i8, flags: c_int, mode: u32) -> c_int;
    fn read(fd: c_int, buf: *mut u8, count: usize) -> isize;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn fstat(fd: c_int, buf: *mut u8) -> c_int;
    fn close(fd: c_int) -> c_int;
}

//==================================================================================================
// Constants
//==================================================================================================

/// O_RDONLY flag for open().
const O_RDONLY: c_int = 0;

/// SEEK_SET for lseek().
const SEEK_SET: c_int = 0;

/// SEEK_END for lseek().
const SEEK_END: c_int = 2;

//==================================================================================================
// Entry Point
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    write_stderr(b"[memfs-test] starting\n");

    // Step 1: Initialize memfs from RAMFS MMIO — do it manually to diagnose.
    write_stderr(b"[memfs-test] step 1a: fat32::init()\n");
    if let Err(_e) = fat32::init() {
        write_stderr(b"[memfs-test] FAIL: fat32::init() failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "fat32::init failed"));
    }
    write_stderr(b"[memfs-test] step 1a: PASS\n");

    write_stderr(b"[memfs-test] step 1b: capctl IoManagement\n");
    if let Err(_e) = ::sys::kcall::pm::capctl(::sys::pm::Capability::IoManagement, true) {
        write_stderr(b"[memfs-test] FAIL: capctl failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "capctl failed"));
    }
    write_stderr(b"[memfs-test] step 1b: PASS\n");

    write_stderr(b"[memfs-test] step 1c: mmio_alloc\n");
    let ramfs_tag: u64 = u64::from_be_bytes(*b"RAMFS   ");
    if let Err(_e) = ::sys::kcall::mm::mmio_alloc(ramfs_tag) {
        write_stderr(b"[memfs-test] FAIL: mmio_alloc failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "mmio_alloc failed"));
    }
    write_stderr(b"[memfs-test] step 1c: PASS\n");

    write_stderr(b"[memfs-test] step 1d: mmio_info\n");
    let info = ::sys::kcall::mm::mmio_info(ramfs_tag)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "mmio_info failed"))?;
    let _total_size: usize = ::sys::mm::Address::into_raw_value(info.base());
    write_stderr(b"[memfs-test] step 1d: PASS (got info)\n");

    let region_size: usize = info.size();
    write_stderr(b"[memfs-test] step 1e: copying RAMFS to heap\n");
    let mut buffer: alloc::vec::Vec<u8> = alloc::vec![0u8; region_size];
    unsafe {
        let src: *const u8 = info.base().as_ptr();
        ::core::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), region_size);
    }
    write_stderr(b"[memfs-test] step 1e: PASS\n");

    write_stderr(b"[memfs-test] step 1f: mmio_free\n");
    let _ = ::sys::kcall::mm::mmio_free(ramfs_tag);
    let _ = ::sys::kcall::pm::capctl(::sys::pm::Capability::IoManagement, false);
    write_stderr(b"[memfs-test] step 1f: PASS\n");

    write_stderr(b"[memfs-test] step 1g: fat32::mount\n");
    let leaked: &'static mut [u8] = alloc::boxed::Box::leak(buffer.into_boxed_slice());
    unsafe {
        if let Err(_e) = fat32::mount("/data", leaked.as_mut_ptr(), leaked.len()) {
            write_stderr(b"[memfs-test] FAIL: fat32::mount failed\n");
            return Err(Error::new(ErrorCode::InvalidArgument, "fat32::mount failed"));
        }
    }
    write_stderr(b"[memfs-test] step 1g: PASS\n");

    // Now init the memfs interception layer.
    write_stderr(b"[memfs-test] step 1h: syscall::memfs::init()\n");
    if let Err(_) = ::syscall::memfs::init() {
        write_stderr(b"[memfs-test] FAIL: memfs::init() failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "memfs::init failed"));
    }
    write_stderr(b"[memfs-test] step 1h: PASS\n");

    // Step 2: Verify memfs_file_size works.
    write_stderr(b"[memfs-test] step 2: memfs_file_size\n");
    let size: i64 =
        unsafe { ::syscall::memfs::memfs_file_size(b"/data/test.txt\0".as_ptr() as *const i8) };
    if size != 18 {
        write_stderr(b"[memfs-test] FAIL: test.txt size mismatch\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "test.txt size mismatch"));
    }
    write_stderr(b"[memfs-test] step 2: PASS (size=18)\n");

    // Step 3: Open file via POSIX open() — should be intercepted by memfs.
    write_stderr(b"[memfs-test] step 3: open via POSIX\n");
    let fd: c_int = unsafe { open(b"/data/test.txt\0".as_ptr() as *const i8, O_RDONLY, 0) };
    if fd < 0 {
        write_stderr(b"[memfs-test] FAIL: open returned negative fd\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "open failed"));
    }
    write_stderr(b"[memfs-test] step 3: PASS (fd >= 1024 means memfs)\n");

    // Step 4: Read file contents via POSIX read().
    write_stderr(b"[memfs-test] step 4: read via POSIX\n");
    let mut buf: [u8; 64] = [0u8; 64];
    let n: isize = unsafe { read(fd, buf.as_mut_ptr(), buf.len()) };
    if n != 18 {
        write_stderr(b"[memfs-test] FAIL: read returned wrong count\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "read count mismatch"));
    }
    if &buf[..18] != b"Hello from memfs!\n" {
        write_stderr(b"[memfs-test] FAIL: read content mismatch\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "read content mismatch"));
    }
    write_stderr(b"[memfs-test] step 4: PASS (content matches)\n");

    // Step 5: Seek back to start and re-read.
    write_stderr(b"[memfs-test] step 5: lseek + re-read\n");
    let pos: i64 = unsafe { lseek(fd, 0, SEEK_SET) };
    if pos != 0 {
        write_stderr(b"[memfs-test] FAIL: lseek to start failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "lseek failed"));
    }
    let n2: isize = unsafe { read(fd, buf.as_mut_ptr(), 5) };
    if n2 != 5 || &buf[..5] != b"Hello" {
        write_stderr(b"[memfs-test] FAIL: re-read after seek mismatch\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "re-read mismatch"));
    }
    write_stderr(b"[memfs-test] step 5: PASS\n");

    // Step 6: Seek to end to get file size.
    write_stderr(b"[memfs-test] step 6: lseek SEEK_END\n");
    let end_pos: i64 = unsafe { lseek(fd, 0, SEEK_END) };
    if end_pos != 18 {
        write_stderr(b"[memfs-test] FAIL: SEEK_END returned wrong position\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "SEEK_END mismatch"));
    }
    write_stderr(b"[memfs-test] step 6: PASS\n");

    // Step 7: Close the file.
    write_stderr(b"[memfs-test] step 7: close\n");
    let ret: c_int = unsafe { close(fd) };
    if ret != 0 {
        write_stderr(b"[memfs-test] FAIL: close returned error\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "close failed"));
    }
    write_stderr(b"[memfs-test] step 7: PASS\n");

    // Step 8: Open and read the binary data file.
    write_stderr(b"[memfs-test] step 8: binary data file\n");
    let fd2: c_int = unsafe { open(b"/data/data.bin\0".as_ptr() as *const i8, O_RDONLY, 0) };
    if fd2 < 0 {
        write_stderr(b"[memfs-test] FAIL: open data.bin failed\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "open data.bin failed"));
    }
    let data_size: i64 =
        unsafe { ::syscall::memfs::memfs_file_size(b"/data/data.bin\0".as_ptr() as *const i8) };
    if data_size != 4096 {
        write_stderr(b"[memfs-test] FAIL: data.bin size mismatch\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "data.bin size mismatch"));
    }
    // Read first 256 bytes and verify pattern.
    let mut data_buf: [u8; 256] = [0u8; 256];
    let n3: isize = unsafe { read(fd2, data_buf.as_mut_ptr(), 256) };
    if n3 != 256 {
        write_stderr(b"[memfs-test] FAIL: data.bin read count mismatch\n");
        return Err(Error::new(ErrorCode::InvalidArgument, "data.bin read count mismatch"));
    }
    for i in 0..256 {
        if data_buf[i] != i as u8 {
            write_stderr(b"[memfs-test] FAIL: data.bin pattern mismatch\n");
            return Err(Error::new(ErrorCode::InvalidArgument, "data.bin pattern mismatch"));
        }
    }
    let _: c_int = unsafe { close(fd2) };
    write_stderr(b"[memfs-test] step 8: PASS\n");

    // All tests passed — output magic string for test runner.
    write_stderr(b"[memfs-test] ALL TESTS PASSED\n");
    let magic: &[u8] = b"ok";
    ::syscall::unistd::write(::sysapi::unistd::STDOUT_FILENO, magic)?;

    Ok(())
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Writes a byte string to stderr for diagnostic output.
fn write_stderr(msg: &[u8]) {
    let _ = ::syscall::unistd::write(::sysapi::unistd::STDERR_FILENO, msg);
}

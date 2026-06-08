// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Benchmark for the host directory mount feature.
//!
//! Measures the latency of common VFS operations on the `/mnt` mount point:
//! - Sequential 4 KiB reads
//! - Sequential 4 KiB writes
//! - File creation
//!
//! Results are written via syslog for collection by the benchmark harness.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of iterations per benchmark.
const ITERATIONS: usize = 100;

/// Buffer size for read/write operations.
const BUF_SIZE: usize = 4096;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns the current timestamp counter value.
fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::x86::_rdtsc()
    }
    #[cfg(not(target_arch = "x86"))]
    {
        0
    }
}

/// Prints a CSV benchmark result line.
fn report(name: &str, total_cycles: u64, iterations: usize) {
    let avg: u64 = total_cycles / iterations as u64;
    ::syslog::info!("mount-bench,{},{},{}", name, avg, iterations);
}

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("mount-bench: starting host mount benchmark");

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Benchmark 1: Sequential 4 KiB reads from a pre-existing file.
    {
        let pathname: FileSystemPath = FileSystemPath::new("/mnt/bench-4k.bin")?;
        let mut total: u64 = 0;
        let mut buf: [u8; BUF_SIZE] = [0u8; BUF_SIZE];

        for _ in 0..ITERATIONS {
            let file: RegularFile =
                FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;

            let start: u64 = rdtsc();
            let _n: usize = file.read(&mut buf)?;
            let end: u64 = rdtsc();
            total += end.wrapping_sub(start);
            // File is closed when dropped.
        }
        report("read-4k", total, ITERATIONS);
    }

    // Benchmark 2: Sequential 4 KiB writes.
    {
        let pathname: FileSystemPath = FileSystemPath::new("/mnt/bench-write.bin")?;
        let data: [u8; BUF_SIZE] = [0xCDu8; BUF_SIZE];
        let mut total: u64 = 0;

        for _ in 0..ITERATIONS {
            let mut file: RegularFile =
                FileSystem::create_regular_file(&pathname, Some(permissions))?;

            let start: u64 = rdtsc();
            file.write(&data)?;
            let end: u64 = rdtsc();
            total += end.wrapping_sub(start);
        }
        report("write-4k", total, ITERATIONS);
    }

    // Benchmark 3: File creation (create + drop/close).
    {
        let mut total: u64 = 0;

        for i in 0..ITERATIONS {
            let mut name_buf: [u8; 32] = [0u8; 32];
            let prefix: &[u8] = b"/mnt/tmp-";
            name_buf[..prefix.len()].copy_from_slice(prefix);
            let mut offset: usize = prefix.len();

            // Write iteration number as ASCII digits.
            let mut num: usize = i;
            let mut digits: [u8; 10] = [0u8; 10];
            let mut d: usize = 0;
            loop {
                digits[d] = b'0' + (num % 10) as u8;
                d += 1;
                num /= 10;
                if num == 0 {
                    break;
                }
            }
            for j in (0..d).rev() {
                name_buf[offset] = digits[j];
                offset += 1;
            }
            let suffix: &[u8] = b".bin";
            name_buf[offset..offset + suffix.len()].copy_from_slice(suffix);
            offset += suffix.len();

            let path_str: &str =
                core::str::from_utf8(&name_buf[..offset]).unwrap_or("/mnt/tmp-0.bin");
            let pathname: FileSystemPath = FileSystemPath::new(path_str)?;

            let start: u64 = rdtsc();
            let _file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
            let end: u64 = rdtsc();
            total += end.wrapping_sub(start);
        }
        report("create-file", total, ITERATIONS);
    }

    ::syslog::info!("mount-bench: benchmark complete");
    Ok(())
}

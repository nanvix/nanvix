// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Syscall Latency Microbenchmark
//!
//! Measures the latency of local kernel calls and linuxd round-trip syscalls using the TSC.
//! Reports per-call nanoseconds for comparing the legacy vmbus path against the
//! shared-memory ring buffer path, including payload-carrying `write()` / `read()` and
//! `pwrite()` / `pread()` sweeps that cross the 4 KiB page boundary.

#![no_std]
#![no_main]

extern crate alloc;
extern crate libc_string;
extern crate nvx;

use ::sys::error::Error;
#[cfg(feature = "payload-sweep")]
use ::sys::error::ErrorCode;
#[cfg(feature = "payload-sweep")]
use ::sysapi::fcntl::{
    atflags::AT_FDCWD,
    file_access_mode::O_RDWR,
    file_creation_flags::{
        O_CREAT,
        O_TRUNC,
    },
};
#[cfg(feature = "payload-sweep")]
use ::sysapi::sys_stat::file_mode::{
    S_IRUSR,
    S_IWUSR,
};
#[cfg(not(feature = "payload-sweep-only"))]
use ::sysapi::{
    fcntl::file_control_request,
    unistd::STDOUT_FILENO,
};
use ::syscall::fcntl;
#[cfg(feature = "payload-sweep")]
use ::syscall::unistd;

/// Number of iterations per benchmark phase.
#[cfg(not(feature = "payload-sweep-only"))]
const ITERATIONS: u32 = 100;
/// Number of iterations per payload-size benchmark point.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_ITERATIONS_SMALL: u32 = 32;
/// Number of iterations per payload-size benchmark point above one page.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_ITERATIONS_LARGE: u32 = 16;
/// Warmup iterations before each payload-size point.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_WARMUP_ITERATIONS: u32 = 4;
/// Threshold beyond which payload points use fewer iterations to keep the total runtime bounded.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_LARGE_THRESHOLD: usize = 4096;
/// Payload sizes for the payload-carrying syscall sweeps.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_SIZES: &[usize] = &[
    32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768, 65536,
];
/// Payload sizes for the positioned payload-carrying syscall sweeps.
#[cfg(feature = "payload-sweep")]
const POSITIONED_PAYLOAD_SIZES: &[usize] = &[
    32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768, 65536, 131072,
];
/// Temporary file used by the payload benchmarks.
#[cfg(feature = "payload-sweep")]
const PAYLOAD_BENCH_FILE: &str = "syscall-bench-payload.tmp";

/// Reads the TSC (timestamp counter) for nanosecond-precision timing.
#[inline]
fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let lo: u32;
        let hi: u32;
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
    #[cfg(target_arch = "x86")]
    unsafe {
        let lo: u32;
        let hi: u32;
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
}

/// Performs a trivial kernel call (getpid) and returns the TSC delta.
#[inline]
#[cfg(not(feature = "payload-sweep-only"))]
fn bench_getpid() -> u64 {
    let start: u64 = rdtsc();
    let _ = ::sys::kcall::pm::getpid();
    let end: u64 = rdtsc();
    end.wrapping_sub(start)
}

/// Performs a trivial kernel call (gettid) and returns the TSC delta.
#[inline]
#[cfg(not(feature = "payload-sweep-only"))]
fn bench_gettid() -> u64 {
    let start: u64 = rdtsc();
    let _ = ::sys::kcall::pm::gettid();
    let end: u64 = rdtsc();
    end.wrapping_sub(start)
}

/// Performs a simple linuxd round-trip using `fcntl(F_GETFL)` on stdout.
#[inline]
#[cfg(not(feature = "payload-sweep-only"))]
fn bench_linuxd_fcntl_getfl() -> Result<u64, Error> {
    let start: u64 = rdtsc();
    let _flags: i32 = fcntl::fcntl(STDOUT_FILENO, file_control_request::F_GETFL, None)?;
    let end: u64 = rdtsc();
    Ok(end.wrapping_sub(start))
}

/// Performs a `pwrite()` round-trip using linuxd and returns the TSC delta.
#[inline]
#[cfg(feature = "payload-sweep")]
fn bench_linuxd_pwrite(fd: i32, buffer: &[u8]) -> Result<u64, Error> {
    let start: u64 = rdtsc();
    let nwritten: usize = unistd::pwrite(fd, buffer, 0)? as usize;
    let end: u64 = rdtsc();

    if nwritten != buffer.len() {
        return Err(Error::new(ErrorCode::TryAgain, "short pwrite"));
    }

    Ok(end.wrapping_sub(start))
}

/// Performs a `pread()` round-trip using linuxd and returns the TSC delta.
#[inline]
#[cfg(feature = "payload-sweep")]
fn bench_linuxd_pread(fd: i32, buffer: &mut [u8]) -> Result<u64, Error> {
    let start: u64 = rdtsc();
    let nread: usize = unistd::pread(fd, buffer, 0)? as usize;
    let end: u64 = rdtsc();

    if nread != buffer.len() {
        return Err(Error::new(ErrorCode::TryAgain, "short pread"));
    }

    Ok(end.wrapping_sub(start))
}

/// Returns the number of iterations to use for a given payload size.
#[cfg(feature = "payload-sweep")]
fn payload_iterations(size: usize) -> u32 {
    if size > PAYLOAD_LARGE_THRESHOLD {
        PAYLOAD_ITERATIONS_LARGE
    } else {
        PAYLOAD_ITERATIONS_SMALL
    }
}

/// Returns the largest payload size in the sweep.
#[cfg(feature = "payload-sweep")]
fn max_payload_size(sizes: &[usize]) -> usize {
    match sizes.last() {
        Some(size) => *size,
        None => 0,
    }
}

/// Returns the largest payload size across all payload benchmark sweeps.
#[cfg(feature = "payload-sweep")]
fn max_benchmark_payload_size() -> usize {
    core::cmp::max(
        max_payload_size(PAYLOAD_SIZES),
        max_payload_size(POSITIONED_PAYLOAD_SIZES),
    )
}

/// Returns the minimum file capacity required for the sequential read/write sweeps.
#[cfg(feature = "payload-sweep")]
fn payload_file_capacity() -> usize {
    max_benchmark_payload_size() * (PAYLOAD_WARMUP_ITERATIONS + PAYLOAD_ITERATIONS_SMALL) as usize
}

/// Prepares the payload benchmark file with deterministic contents.
#[cfg(feature = "payload-sweep")]
fn prepare_payload_file(fd: i32, size: usize, fill: u8) -> Result<(), Error> {
    let payload: alloc::vec::Vec<u8> = alloc::vec![fill; size];
    let nwritten: usize = unistd::pwrite(fd, &payload, 0)? as usize;

    if nwritten != payload.len() {
        return Err(Error::new(ErrorCode::TryAgain, "short payload file setup"));
    }

    Ok(())
}

#[inline]
fn write_str(buf: &mut [u8], pos: &mut usize, s: &[u8]) {
    for &b in s {
        if *pos < buf.len() {
            buf[*pos] = b;
            *pos += 1;
        }
    }
}

#[inline]
fn write_u64(buf: &mut [u8], pos: &mut usize, val: u64) {
    if val == 0 {
        if *pos < buf.len() {
            buf[*pos] = b'0';
            *pos += 1;
        }
        return;
    }

    let mut digits: [u8; 20] = [0u8; 20];
    let mut n: u64 = val;
    let mut i: usize = 0;
    while n > 0 {
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        if *pos < buf.len() {
            buf[*pos] = digits[i];
            *pos += 1;
        }
    }
}

/// Builds a CloseRequest IPC message targeting linuxd with fd=-1.
///
/// Constructs the payload directly to avoid depending on the syscall crate.
/// LinuxDaemonMessageHeader::CloseRequest has discriminant 4 (u16).
/// The fd field is i32 at offset 2 in the LinuxDaemonMessage payload.
#[cfg(all(feature = "ipc-bench", not(feature = "payload-sweep-only")))]
fn build_close_request(tid: ::sys::pm::ThreadIdentifier) -> ::sys::ipc::Message {
    let mut payload: [u8; ::sys::ipc::Message::PAYLOAD_SIZE] =
        [0u8; ::sys::ipc::Message::PAYLOAD_SIZE];
    // LinuxDaemonMessageHeader::CloseRequest discriminant.
    payload[0..2].copy_from_slice(&4u16.to_ne_bytes());
    // fd = -1, which linuxd rejects with EBADF (fast no-op round-trip).
    payload[2..6].copy_from_slice(&(-1i32).to_ne_bytes());

    ::sys::ipc::Message::new(
        ::sys::ipc::MessageSender::from(tid),
        ::sys::ipc::MessageReceiver::from(::sys::pm::ProcessIdentifier::KERNEL),
        ::sys::ipc::MessageType::Ikc,
        None,
        payload,
    )
}

/// Performs a single IPC round-trip (send CloseRequest + recv response)
/// and returns the TSC delta. Only usable in multi-process mode (linuxd required).
#[cfg(all(feature = "ipc-bench", not(feature = "payload-sweep-only")))]
#[inline]
fn bench_ipc_roundtrip(request: &::sys::ipc::Message) -> u64 {
    let start: u64 = rdtsc();
    let _ = ::sys::kcall::ipc::send(request);
    let _ = ::sys::kcall::ipc::recv();
    let end: u64 = rdtsc();
    end.wrapping_sub(start)
}

/// Writes benchmark results in a machine-readable format.
#[cfg(not(feature = "payload-sweep-only"))]
fn report(name: &str, iterations: u32, total_cycles: u64) {
    let avg_cycles: u64 = total_cycles / (iterations as u64);
    // Approximate nanoseconds assuming ~2 GHz TSC (common for KVM guests).
    let approx_ns: u64 = avg_cycles / 2;

    // Format: "BENCH <name>: <iterations> iterations, <avg_cycles> avg cycles, ~<ns> ns/call\n"
    // We use manual formatting since we're in no_std without format_args! for debug output.
    let mut buf: [u8; 128] = [0u8; 128];
    let mut pos: usize = 0;

    write_str(&mut buf, &mut pos, b"BENCH ");
    write_str(&mut buf, &mut pos, name.as_bytes());
    write_str(&mut buf, &mut pos, b": ");
    write_u64(&mut buf, &mut pos, iterations as u64);
    write_str(&mut buf, &mut pos, b" iters, ");
    write_u64(&mut buf, &mut pos, avg_cycles);
    write_str(&mut buf, &mut pos, b" avg cyc, ~");
    write_u64(&mut buf, &mut pos, approx_ns);
    write_str(&mut buf, &mut pos, b" ns/call\n");

    let _ = ::sys::kcall::debug::debug(buf.as_ptr(), pos);
}

/// Writes payload benchmark results in a machine-readable format.
#[cfg(feature = "payload-sweep")]
fn report_payload(name: &str, size: usize, iterations: u32, total_cycles: u64) {
    let avg_cycles: u64 = total_cycles / (iterations as u64);
    let approx_ns: u64 = avg_cycles / 2;
    let mut buf: [u8; 160] = [0u8; 160];
    let mut pos: usize = 0;

    write_str(&mut buf, &mut pos, b"SIZEBENCH ");
    write_str(&mut buf, &mut pos, name.as_bytes());
    write_str(&mut buf, &mut pos, b": ");
    write_u64(&mut buf, &mut pos, size as u64);
    write_str(&mut buf, &mut pos, b" bytes, ");
    write_u64(&mut buf, &mut pos, iterations as u64);
    write_str(&mut buf, &mut pos, b" iters, ");
    write_u64(&mut buf, &mut pos, avg_cycles);
    write_str(&mut buf, &mut pos, b" avg cyc, ~");
    write_u64(&mut buf, &mut pos, approx_ns);
    write_str(&mut buf, &mut pos, b" ns/call\n");

    let _ = ::sys::kcall::debug::debug(buf.as_ptr(), pos);
}

fn print(msg: &[u8]) {
    let _ = ::sys::kcall::debug::debug(msg.as_ptr(), msg.len());
}

#[cfg(feature = "payload-sweep")]
fn run_payload_sweep() -> Result<(), Error> {
    let flags: i32 = O_CREAT | O_RDWR | O_TRUNC;
    let fd: i32 = fcntl::openat(AT_FDCWD, PAYLOAD_BENCH_FILE, flags, S_IRUSR | S_IWUSR)?;
    let file_capacity: usize = payload_file_capacity();

    prepare_payload_file(fd, file_capacity, 0xa5)?;

    print(b"--- linuxd payload sweep: pwrite() ---\n");

    for &size in POSITIONED_PAYLOAD_SIZES {
        let iterations: u32 = payload_iterations(size);
        let payload: alloc::vec::Vec<u8> = alloc::vec![0x5au8; size];

        for _ in 0..PAYLOAD_WARMUP_ITERATIONS {
            let _ = bench_linuxd_pwrite(fd, &payload)?;
        }

        let mut total_cycles: u64 = 0;
        for _ in 0..iterations {
            total_cycles += bench_linuxd_pwrite(fd, &payload)?;
        }

        report_payload("pwrite", size, iterations, total_cycles);
    }

    prepare_payload_file(fd, file_capacity, 0xa5)?;

    print(b"--- linuxd payload sweep: pread() ---\n");

    for &size in POSITIONED_PAYLOAD_SIZES {
        let iterations: u32 = payload_iterations(size);
        let mut payload: alloc::vec::Vec<u8> = alloc::vec![0u8; size];

        for _ in 0..PAYLOAD_WARMUP_ITERATIONS {
            let _ = bench_linuxd_pread(fd, &mut payload)?;
        }

        let mut total_cycles: u64 = 0;
        for _ in 0..iterations {
            total_cycles += bench_linuxd_pread(fd, &mut payload)?;
        }

        report_payload("pread", size, iterations, total_cycles);
    }

    unistd::close(fd)?;
    unistd::unlink(PAYLOAD_BENCH_FILE)?;

    Ok(())
}

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    print(b"=== Syscall Latency Microbenchmark ===\n");

    #[cfg(not(feature = "payload-sweep-only"))]
    {
        // Phase 1: Warm up the TSC and caches.
        for _ in 0..100 {
            let _ = bench_getpid();
        }

        // Phase 2: Measure getpid() latency (pure kcall, no IPC).
        let mut total_getpid: u64 = 0;
        for _ in 0..ITERATIONS {
            total_getpid += bench_getpid();
        }
        report("getpid", ITERATIONS, total_getpid);

        // Phase 3: Measure gettid() latency (pure kcall, no IPC).
        let mut total_gettid: u64 = 0;
        for _ in 0..ITERATIONS {
            total_gettid += bench_gettid();
        }
        report("gettid", ITERATIONS, total_gettid);

        // Phase 4: Measure a guest -> linuxd -> guest round-trip syscall.
        let mut total_linuxd_fcntl: u64 = 0;
        for _ in 0..ITERATIONS {
            total_linuxd_fcntl += bench_linuxd_fcntl_getfl()?;
        }
        report("fcntl(F_GETFL)", ITERATIONS, total_linuxd_fcntl);

        // Phase 5: Measure gettime() latency (pvclock read, no VM exit).
        let mut total_gettime: u64 = 0;
        let mut time_buf: ::sys::time::SystemTime = ::sys::time::SystemTime::default();
        for _ in 0..ITERATIONS {
            let start: u64 = rdtsc();
            let _ = ::sys::kcall::pm::gettime(&mut time_buf);
            let end: u64 = rdtsc();
            total_gettime += end.wrapping_sub(start);
        }
        report("gettime", ITERATIONS, total_gettime);

        // Phase 6: IPC round-trip latency (multi-process only, requires linuxd).
        #[cfg(feature = "ipc-bench")]
        {
            const IPC_ITERATIONS: u32 = 1_000;
            let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::gettid()?;
            let request: ::sys::ipc::Message = build_close_request(tid);

            print(b"--- IPC round-trip: close(-1) -> linuxd -> EBADF ---\n");

            // Warmup.
            for _ in 0..10 {
                let _ = bench_ipc_roundtrip(&request);
            }

            let mut total_ipc: u64 = 0;
            for _ in 0..IPC_ITERATIONS {
                total_ipc += bench_ipc_roundtrip(&request);
            }
            report("ipc-close", IPC_ITERATIONS, total_ipc);
        }
    }

    #[cfg(feature = "payload-sweep")]
    run_payload_sweep()?;

    print(b"\n=== Benchmark complete ===\n");

    Ok(())
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::core::sync::atomic::Ordering;
use ::sys::error::Error;
use alloc::{
    collections::BTreeMap,
    vec,
    vec::Vec,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Total memory footprint for the sparse data structure in bytes, derived from the VM's physical
/// memory size. We use half of the user heap capacity to leave room for runtime overhead.
const MEMORY_SIZE: usize = config::memory_layout::USER_HEAP_CAPACITY / 2;

/// Size of each value payload in bytes.
const VALUE_SIZE: usize = 512;

/// Number of entries in the sparse index, derived from the target memory size.
const NUM_ENTRIES: usize = MEMORY_SIZE / VALUE_SIZE;

/// Number of random-walk steps executed before the snapshot is taken. These steps dirty heap
/// pages in an irregular pattern so the captured snapshot reflects realistic application state.
const PRE_SNAPSHOT_WALK_STEPS: usize = 4096;

/// Number of random-walk steps executed after the snapshot is restored. These steps exercise both
/// reads and writes against the restored state, simulating workload activity that resumes from the
/// snapshot.
const POST_SNAPSHOT_WALK_STEPS: usize = 1024;

/// Build-time seed used to drive the random walks. Bumping `NANVIX_BENCH_SEED` (or rebuilding
/// without it set) yields a different access pattern, preventing the benchmark from collapsing to
/// a single fixed trace.
const BENCH_SEED: u64 = parse_u64(env!("NANVIX_BENCH_SEED"));

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Parses a decimal `u64` at compile time. Non-digit bytes terminate parsing; overflow wraps.
const fn parse_u64(s: &str) -> u64 {
    let bytes: &[u8] = s.as_bytes();
    let mut i: usize = 0;
    let mut acc: u64 = 0;
    while i < bytes.len() {
        let b: u8 = bytes[i];
        if b < b'0' || b > b'9' {
            break;
        }
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as u64);
        i += 1;
    }
    acc
}

/// Returns the initial PRNG state, ensuring it is non-zero (xorshift requires a non-zero seed).
const fn initial_prng_state() -> u64 {
    let seed: u64 = BENCH_SEED;
    if seed == 0 {
        0x9E3779B97F4A7C15
    } else {
        seed
    }
}

/// Advances a xorshift64 PRNG and returns the next pseudo-random `u64`.
#[inline(always)]
fn xorshift64(state: &mut u64) -> u64 {
    let mut x: u64 = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

///
/// # Description
///
/// Builds a sparse in-memory data structure that simulates realistic application
/// state (e.g., a page-table-like index or cache) before taking a VM snapshot.
/// This ensures the snapshot captures dirty memory pages scattered across the
/// heap, producing a more realistic measurement of snapshot latency.
///
fn build_sparse_state() -> BTreeMap<u64, Vec<u8>> {
    let mut map: BTreeMap<u64, Vec<u8>> = BTreeMap::new();

    for i in 0..NUM_ENTRIES {
        // Keys are spaced by `VALUE_SIZE` so successive entries land in distinct cache lines
        // (and frequently distinct pages) without overlapping.
        let key: u64 = (i as u64) * (VALUE_SIZE as u64);
        // Fill each value with a pattern derived from the key so the compiler
        // cannot optimize the allocation away.
        let value: Vec<u8> = vec![(key & 0xFF) as u8; VALUE_SIZE];
        map.insert(key, value);
    }

    map
}

///
/// # Description
///
/// Executes `steps` random-walk operations against `state`, mixing reads and writes. Each step
/// hashes the PRNG output to select a key, accumulates a byte sampled from the value (read), and
/// then mutates a byte at a PRNG-selected offset (write). The accumulator is returned so callers
/// can feed it into `black_box` and defeat dead-store elimination.
///
fn random_walk(state: &mut BTreeMap<u64, Vec<u8>>, prng: &mut u64, steps: usize) -> u64 {
    let mut acc: u64 = 0;
    for _ in 0..steps {
        let r0: u64 = xorshift64(prng);
        let r1: u64 = xorshift64(prng);
        let key: u64 = (r0 % NUM_ENTRIES as u64) * (VALUE_SIZE as u64);
        if let Some(value) = state.get_mut(&key) {
            // Read: sample one byte from a pseudo-random offset.
            let read_offset: usize = (r0 as usize) % value.len();
            acc = acc.wrapping_add(value[read_offset] as u64);
            // Write: mutate one byte at a different pseudo-random offset.
            let write_offset: usize = (r1 as usize) % value.len();
            value[write_offset] = value[write_offset].wrapping_add((r1 & 0xFF) as u8);
        }
    }
    acc
}

/// Returns `true` if `--snapshot` was passed as a command-line argument.
fn should_snapshot() -> bool {
    let argc: i32 = nvx_crt0::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);

    if argv.is_null() || argc <= 1 {
        return false;
    }

    let flag: &[u8] = b"--snapshot";

    for i in 1..argc {
        let ptr: *const u8 = unsafe { *argv.add(i as usize) };
        if ptr.is_null() {
            continue;
        }
        // Compare the argument byte-by-byte against "--snapshot", stopping at the
        // first null terminator so we never read past the end of a short argument.
        let mut matches: bool = true;
        for (j, &expected) in flag.iter().enumerate() {
            let byte: u8 = unsafe { *ptr.add(j) };
            if byte == 0 || byte != expected {
                matches = false;
                break;
            }
        }
        // Ensure the argument is exactly "--snapshot" (null-terminated after the flag).
        if matches && unsafe { *ptr.add(flag.len()) } == 0 {
            return true;
        }
    }

    false
}

///
/// # Description
///
/// Benchmark entry point. Populates a sparse memory data structure to dirty
/// heap pages across the address space, then exercises that state with a random
/// walk of reads and writes before the snapshot is taken. If `--snapshot` is
/// passed, creates a VM snapshot and, upon restore, performs a post-snapshot
/// random walk against the restored state; otherwise exits immediately after
/// the pre-snapshot workload (for cold-start measurement).
///
/// Note on measurement: `pm::snapshot()` returns on both the snapshot-creation
/// run (after the VMM saves state and resumes) and the restore run (after the
/// VMM loads state and resumes). The post-snapshot walk therefore executes in
/// both paths. To avoid this skewing the reported snapshot-creation latency,
/// the host benchmark records the actual save duration via per-phase VMM
/// timings instead of wall-clock for that phase.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Build sparse state so the snapshot captures meaningful dirty pages.
    let mut state: BTreeMap<u64, Vec<u8>> = build_sparse_state();

    // Pre-snapshot random walk: dirty pages in an irregular pattern.
    let mut prng: u64 = initial_prng_state();
    let pre_acc: u64 = random_walk(&mut state, &mut prng, PRE_SNAPSHOT_WALK_STEPS);
    core::hint::black_box(pre_acc);

    // Only take a snapshot and run post-restore work if explicitly requested.
    if !should_snapshot() {
        return Ok(());
    }

    ::sys::kcall::pm::snapshot()?;

    // Post-snapshot random walk: exercise reads and writes on the restored state. When the VM
    // resumes from a snapshot, execution lands here and this walk runs against the restored heap.
    let post_acc: u64 = random_walk(&mut state, &mut prng, POST_SNAPSHOT_WALK_STEPS);
    core::hint::black_box(post_acc);
    core::hint::black_box(&state);

    Ok(())
}

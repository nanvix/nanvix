// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    CapabilityGuard,
    StressError,
    WorkerStack,
    error_code_from_usize,
    error_code_to_usize,
    exposed_addr_to_mut_u8,
    thread_args,
};
use ::alloc::vec::Vec;
use ::config::{
    constants::KILOBYTE,
    kernel::SCOREBOARD_SLOTS,
};
use ::core::{
    cmp,
    convert::TryFrom,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};
use ::sys::{
    config::memory_layout::USER_MMAP_BASE,
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm::{
            mmap,
            munmap,
        },
        pm::{
            create_thread,
            getpid,
            join_thread,
        },
        sched::sched_yield,
    },
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::{
        Capability,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const SCOREBOARD_PRESSURE_ITERATIONS: usize = 48;
const SCOREBOARD_SLOT_COUNT: usize = SCOREBOARD_SLOTS;
const SCOREBOARD_PRESSURE_PAGE_BYTES: usize = 4 * KILOBYTE;
const SCOREBOARD_PRESSURE_REGION_STRIDE_BYTES: usize =
    SCOREBOARD_PRESSURE_PAGE_BYTES * SCOREBOARD_PRESSURE_ITERATIONS * 2;
const SCOREBOARD_PRESSURE_YIELD_MASK: usize = 0x7;
const SCOREBOARD_PRESSURE_BYTE_MASK: usize = 0xff;
const MIN_SCOREBOARD_PRESSURE_WORKERS: usize = 8;
/// Number of workers beyond the scoreboard slot limit to test backpressure handling.
/// Setting this to 1 ensures at least one worker must wait for a slot, exercising the
/// semaphore-based flow control in the scoreboard dispatcher.
const SCOREBOARD_PRESSURE_OVERSUBSCRIPTION: usize = 1;
const SCOREBOARD_PRESSURE_CONCURRENCY_LIMIT: usize =
    SCOREBOARD_SLOT_COUNT + SCOREBOARD_PRESSURE_OVERSUBSCRIPTION;
/// Base value for encoding worker errors in the return value. A return value at or above this
/// threshold indicates failure, with the error code encoded as
/// `retval - SCOREBOARD_PRESSURE_ERROR_BASE`.
const SCOREBOARD_PRESSURE_ERROR_BASE: usize = usize::MAX - 0xFFFF;

//==================================================================================================
// Globals
//==================================================================================================

static SCOREBOARD_PRESSURE_PROGRESS: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Launches enough mmap/munmap workers to over-subscribe the kernel call scoreboard and verify
/// that concurrent rendezvous slots make forward progress.
///
/// The test uses a batched concurrency model: instead of spawning all workers at once (which could
/// exhaust system resources), it spawns workers in batches limited by `concurrency_limit`. Each
/// batch runs to completion before the next batch starts. This ensures that at least
/// `SCOREBOARD_PRESSURE_OVERSUBSCRIPTION` workers beyond the scoreboard slot limit are active
/// simultaneously, exercising the semaphore-based backpressure mechanism in the dispatcher.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread or kernel call operations fail.
///
pub fn run() -> Result<(), StressError> {
    SCOREBOARD_PRESSURE_PROGRESS.store(0, Ordering::Relaxed);

    let mut capability_guard: CapabilityGuard =
        CapabilityGuard::enable(Capability::MemoryManagement)?;

    let worker_count: usize = cmp::max(SCOREBOARD_SLOT_COUNT * 2, MIN_SCOREBOARD_PRESSURE_WORKERS);
    let desired_concurrency: usize =
        cmp::max(SCOREBOARD_PRESSURE_CONCURRENCY_LIMIT, MIN_SCOREBOARD_PRESSURE_WORKERS);
    let concurrency_limit: usize = cmp::min(desired_concurrency, worker_count);

    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(concurrency_limit);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(concurrency_limit);

    let mut next_worker_id: usize = 0;
    while next_worker_id < worker_count {
        let batch_size: usize = cmp::min(concurrency_limit, worker_count - next_worker_id);

        for worker_offset in 0..batch_size {
            let worker_id: usize = next_worker_id + worker_offset;
            let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_STACK_SIZE)?;
            let mut args: ThreadCreateArgs =
                thread_args(&stack, scoreboard_pressure_worker, worker_id);
            match create_thread(&mut args) {
                Ok(tid) => {
                    tids.push(tid);
                    stacks.push(stack);
                },
                Err(e) => {
                    // Best-effort cleanup: join already-spawned threads before dropping stacks.
                    for (tid, stack) in tids.drain(..).zip(stacks.drain(..)) {
                        let mut retval: usize = 0;
                        let _: Result<(), StressError> = join_thread(tid, &mut retval);
                        drop(stack);
                    }
                    return Err(e);
                },
            }
        }

        for (tid, stack) in tids.drain(..).zip(stacks.drain(..)) {
            let mut retval: usize = 0;
            join_thread(tid, &mut retval)?;
            drop(stack);

            if retval >= SCOREBOARD_PRESSURE_ERROR_BASE {
                let code: ErrorCode =
                    error_code_from_usize(retval.wrapping_sub(SCOREBOARD_PRESSURE_ERROR_BASE));
                return Err(Error::new(code, "scoreboard pressure worker failed"));
            }

            if retval != SCOREBOARD_PRESSURE_ITERATIONS {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "scoreboard pressure worker iterations mismatch",
                ));
            }
        }

        next_worker_id += batch_size;
    }

    let expected_progress: usize = worker_count * SCOREBOARD_PRESSURE_ITERATIONS;
    let observed_progress: usize = SCOREBOARD_PRESSURE_PROGRESS.load(Ordering::Acquire);
    if observed_progress != expected_progress {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "scoreboard pressure progress mismatch",
        ));
    }

    capability_guard.disable()?;
    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for each mmap/munmap worker that contributes to scoreboard contention.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Number of successful iterations performed or an encoded error on failure.
///
extern "C" fn scoreboard_pressure_worker(worker_id: usize) -> usize {
    match scoreboard_pressure_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => SCOREBOARD_PRESSURE_ERROR_BASE.wrapping_add(error_code_to_usize(err.code)),
    }
}

///
/// # Description
///
/// Executes the mmap/munmap loop for a single worker and records progress atomically.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Total iterations completed on success.
///
/// # Errors
///
/// Propagates failures from kernel calls or address calculations.
///
fn scoreboard_pressure_worker_impl(worker_id: usize) -> Result<usize, Error> {
    let pid: ProcessIdentifier = getpid()?;
    let region_base: usize = worker_region_base(worker_id)?;

    for iteration in 0..SCOREBOARD_PRESSURE_ITERATIONS {
        let page_offset: usize = iteration * SCOREBOARD_PRESSURE_PAGE_BYTES;
        let addr_raw: usize = region_base + page_offset;
        let addr: VirtualAddress = VirtualAddress::from_raw_value(addr_raw);

        mmap(pid, addr, AccessPermission::RDWR)?;
        let byte_raw: usize = (worker_id ^ iteration) & SCOREBOARD_PRESSURE_BYTE_MASK;
        let byte: u8 = u8::try_from(byte_raw)
            .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "worker byte overflow"))?;
        unsafe {
            let ptr: *mut u8 = exposed_addr_to_mut_u8(addr_raw);
            ptr.write_volatile(byte);
            // Verify the written value can be read back correctly. This catches potential
            // result corruption if the scoreboard delivers a result to the wrong dispatcher.
            let readback: u8 = ptr.read_volatile();
            if readback != byte {
                return Err(Error::new(ErrorCode::InvalidArgument, "memory readback mismatch"));
            }
        }
        munmap(pid, addr)?;

        SCOREBOARD_PRESSURE_PROGRESS.fetch_add(1, Ordering::AcqRel);

        if (iteration ^ worker_id) & SCOREBOARD_PRESSURE_YIELD_MASK == 0 {
            sched_yield()?;
        }
    }

    Ok(SCOREBOARD_PRESSURE_ITERATIONS)
}

///
/// # Description
///
/// Computes the base virtual address for a worker's mapping region to avoid overlap.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Base address for the worker's mapping range.
///
/// # Errors
///
/// Returns `ValueOutOfRange` when address arithmetic overflows.
///
fn worker_region_base(worker_id: usize) -> Result<usize, Error> {
    let user_base: usize = usize::from(USER_MMAP_BASE);
    let offset: usize = worker_id
        .checked_mul(SCOREBOARD_PRESSURE_REGION_STRIDE_BYTES)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "worker offset overflow"))?;

    user_base
        .checked_add(offset)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "virtual address overflow"))
}

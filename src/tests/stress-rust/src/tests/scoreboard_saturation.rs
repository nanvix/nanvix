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
            gettime,
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
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of rapid serial iterations per worker in the recycling test.
const RAPID_RECYCLE_ITERATIONS: usize = 64;

/// Workers for the rapid serial recycling test (single thread exercises slot reuse).
const RAPID_RECYCLE_WORKERS: usize = 1;

/// Workers for the mixed kcall test -- enough to fill all slots simultaneously.
const MIXED_KCALL_WORKERS: usize = SCOREBOARD_SLOTS;

/// Iterations per worker in the mixed kcall test.
const MIXED_KCALL_ITERATIONS: usize = 32;

/// Workers for the full saturation test -- exceeds scoreboard capacity.
const SATURATION_WORKERS: usize = SCOREBOARD_SLOTS * 3;

/// Iterations per worker in the saturation test.
const SATURATION_ITERATIONS: usize = 16;

/// Page size used for mmap/munmap operations.
const PAGE_BYTES: usize = 4 * KILOBYTE;

/// Byte mask for verification patterns.
const BYTE_MASK: usize = 0xff;

/// Yield cadence mask -- yield every 4th iteration on average.
const YIELD_MASK: usize = 0x3;

/// Base value for encoding worker errors in the return value. A return value at or above this
/// threshold indicates failure, with the error code encoded as `retval - WORKER_ERROR_BASE`.
const WORKER_ERROR_BASE: usize = usize::MAX - 0xFFFF;

/// Region stride to prevent virtual address overlap between workers.
const REGION_STRIDE_BYTES: usize = PAGE_BYTES * 128;

//==================================================================================================
// Globals
//==================================================================================================

/// Tracks total progress across all workers in the current test phase.
static PROGRESS: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Runs three scoreboard saturation scenarios that complement the backpressure test:
///
/// 1. **Rapid serial recycling**: A single thread dispatches many kernel calls in tight succession,
///    verifying that slot allocation and release cycle correctly without leaks.
///
/// 2. **Mixed concurrent kcall types**: Workers issue different kernel call types (`gettime`,
///    `mmap`/`munmap`) concurrently, exercising the scoreboard with varied kcall numbers flowing
///    through the same slot pool.
///
/// 3. **Full saturation**: Three times the scoreboard capacity worth of workers compete for slots,
///    pushing the semaphore-based backpressure mechanism to its limits and verifying no deadlocks
///    or lost wakeups occur under heavy contention.
///
/// # Returns
///
/// `Ok(())` on success or an error if any scenario fails.
///
pub fn run() -> Result<(), StressError> {
    run_rapid_serial_recycling()?;
    run_mixed_concurrent_kcalls()?;
    run_full_saturation()?;
    Ok(())
}

//==================================================================================================
// Scenario 1: Rapid Serial Recycling
//==================================================================================================

///
/// # Description
///
/// Verifies that a single thread can rapidly dispatch many kernel calls without scoreboard slot
/// exhaustion. Each mmap/munmap pair exercises the full dispatch-handle-handled-release cycle,
/// confirming that slot recycling is immediate and correct.
///
/// # Returns
///
/// `Ok(())` on success or an error if the worker fails.
///
fn run_rapid_serial_recycling() -> Result<(), StressError> {
    reset_globals();

    let mut capability_guard: CapabilityGuard =
        CapabilityGuard::enable(Capability::MemoryManagement)?;

    let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = thread_args(&stack, rapid_recycle_worker, 0);
    let tid: ThreadIdentifier = create_thread(&mut args)?;

    let mut retval: usize = 0;
    join_thread(tid, &mut retval)?;
    drop(stack);

    check_worker_result(retval, RAPID_RECYCLE_ITERATIONS)?;
    check_progress(RAPID_RECYCLE_WORKERS * RAPID_RECYCLE_ITERATIONS)?;

    capability_guard.disable()?;
    Ok(())
}

//==================================================================================================
// Scenario 2: Mixed Concurrent Kernel Call Types
//==================================================================================================

///
/// # Description
///
/// Verifies that the scoreboard correctly handles concurrent dispatch of different kernel call
/// types. Even-numbered workers perform `mmap`/`munmap` cycles while odd-numbered workers call
/// `gettime` repeatedly, ensuring the scoreboard does not conflate slot states across kcall types.
///
/// # Returns
///
/// `Ok(())` on success or an error if any worker fails.
///
fn run_mixed_concurrent_kcalls() -> Result<(), StressError> {
    reset_globals();

    let mut capability_guard: CapabilityGuard =
        CapabilityGuard::enable(Capability::MemoryManagement)?;

    // MIXED_KCALL_WORKERS equals SCOREBOARD_SLOTS by definition; the variable is kept for
    // clarity and to allow adjusting the constant independently without breaking this call site.
    let worker_count: usize = MIXED_KCALL_WORKERS;
    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(worker_count);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, mixed_kcall_worker, worker_id);
        match create_thread(&mut args) {
            Ok(tid) => {
                tids.push(tid);
                stacks.push(stack);
            },
            Err(e) => {
                // Best-effort cleanup: join already-spawned threads before dropping their stacks.
                for (tid, stack) in tids.into_iter().zip(stacks.into_iter()) {
                    let mut retval: usize = 0;
                    let _: Result<(), StressError> = join_thread(tid, &mut retval);
                    drop(stack);
                }
                return Err(e);
            },
        }
    }

    for (tid, stack) in tids.into_iter().zip(stacks.into_iter()) {
        let mut retval: usize = 0;
        join_thread(tid, &mut retval)?;
        drop(stack);
        check_worker_result(retval, MIXED_KCALL_ITERATIONS)?;
    }

    check_progress(worker_count * MIXED_KCALL_ITERATIONS)?;

    capability_guard.disable()?;
    Ok(())
}

//==================================================================================================
// Scenario 3: Full Saturation
//==================================================================================================

///
/// # Description
///
/// Spawns workers in excess of the scoreboard slot count to confirm that the semaphore-based
/// backpressure mechanism prevents deadlocks and allows all workers to eventually complete. Workers
/// are batched at a concurrency level of `SCOREBOARD_SLOTS + 4` to stress the flow-control path.
///
/// # Returns
///
/// `Ok(())` on success or an error if any worker fails.
///
fn run_full_saturation() -> Result<(), StressError> {
    reset_globals();

    let mut capability_guard: CapabilityGuard =
        CapabilityGuard::enable(Capability::MemoryManagement)?;

    let worker_count: usize = SATURATION_WORKERS;
    // Use a higher oversubscription factor than the basic backpressure test.
    let concurrency_limit: usize = cmp::min(SCOREBOARD_SLOTS + 4, worker_count);

    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(concurrency_limit);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(concurrency_limit);

    let mut next_worker_id: usize = 0;
    while next_worker_id < worker_count {
        let batch_size: usize = cmp::min(concurrency_limit, worker_count - next_worker_id);

        for worker_offset in 0..batch_size {
            let worker_id: usize = next_worker_id + worker_offset;
            let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_STACK_SIZE)?;
            let mut args: ThreadCreateArgs = thread_args(&stack, saturation_worker, worker_id);
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
            check_worker_result(retval, SATURATION_ITERATIONS)?;
        }

        next_worker_id += batch_size;
    }

    check_progress(worker_count * SATURATION_ITERATIONS)?;

    capability_guard.disable()?;
    Ok(())
}

//==================================================================================================
// Worker Entry Points
//==================================================================================================

///
/// # Description
///
/// Entry point for the rapid serial recycling worker.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Number of successful iterations or an encoded error on failure.
///
extern "C" fn rapid_recycle_worker(worker_id: usize) -> usize {
    match rapid_recycle_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => WORKER_ERROR_BASE.wrapping_add(error_code_to_usize(err.code)),
    }
}

///
/// # Description
///
/// Entry point for mixed kernel call workers.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Number of successful iterations or an encoded error on failure.
///
extern "C" fn mixed_kcall_worker(worker_id: usize) -> usize {
    match mixed_kcall_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => WORKER_ERROR_BASE.wrapping_add(error_code_to_usize(err.code)),
    }
}

///
/// # Description
///
/// Entry point for full saturation workers.
///
/// # Parameters
///
/// - `worker_id`: Unique identifier assigned to the worker thread.
///
/// # Returns
///
/// Number of successful iterations or an encoded error on failure.
///
extern "C" fn saturation_worker(worker_id: usize) -> usize {
    match saturation_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => WORKER_ERROR_BASE.wrapping_add(error_code_to_usize(err.code)),
    }
}

//==================================================================================================
// Worker Implementations
//==================================================================================================

///
/// # Description
///
/// Rapidly maps and unmaps pages in a tight loop to exercise scoreboard slot recycling. Each
/// iteration is a full round-trip through the scoreboard (dispatch, handle, handled, release).
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
fn rapid_recycle_worker_impl(worker_id: usize) -> Result<usize, Error> {
    let pid: ProcessIdentifier = getpid()?;
    let region_base: usize = worker_region_base(worker_id)?;

    for iteration in 0..RAPID_RECYCLE_ITERATIONS {
        let addr_raw: usize = region_base + (iteration * PAGE_BYTES);
        let addr: VirtualAddress = VirtualAddress::from_raw_value(addr_raw);

        // Map, touch, unmap -- three dispatched kcalls per iteration.
        mmap(pid, addr, AccessPermission::RDWR)?;
        let byte: u8 = u8::try_from(iteration & BYTE_MASK)
            .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "byte overflow"))?;
        unsafe {
            let ptr: *mut u8 = exposed_addr_to_mut_u8(addr_raw);
            ptr.write_volatile(byte);
        }
        munmap(pid, addr)?;

        PROGRESS.fetch_add(1, Ordering::AcqRel);
    }

    Ok(RAPID_RECYCLE_ITERATIONS)
}

///
/// # Description
///
/// Performs a mix of kernel call types: even-numbered workers map/unmap pages while odd-numbered
/// workers call `gettime`. This exercises the scoreboard with heterogeneous kcall numbers flowing
/// through the same slot pool.
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
/// Propagates failures from kernel calls.
///
fn mixed_kcall_worker_impl(worker_id: usize) -> Result<usize, Error> {
    let pid: ProcessIdentifier = getpid()?;
    let is_memory_worker: bool = worker_id.is_multiple_of(2);

    if is_memory_worker {
        let region_base: usize = worker_region_base(worker_id)?;
        for iteration in 0..MIXED_KCALL_ITERATIONS {
            let addr_raw: usize = region_base + (iteration * PAGE_BYTES);
            let addr: VirtualAddress = VirtualAddress::from_raw_value(addr_raw);

            mmap(pid, addr, AccessPermission::RDWR)?;
            munmap(pid, addr)?;

            PROGRESS.fetch_add(1, Ordering::AcqRel);

            if (iteration + worker_id) & YIELD_MASK == 0 {
                sched_yield()?;
            }
        }
    } else {
        for iteration in 0..MIXED_KCALL_ITERATIONS {
            let mut now: SystemTime = SystemTime::default();
            gettime(&mut now)?;

            PROGRESS.fetch_add(1, Ordering::AcqRel);

            if (iteration + worker_id) & YIELD_MASK == 0 {
                sched_yield()?;
            }
        }
    }

    Ok(MIXED_KCALL_ITERATIONS)
}

///
/// # Description
///
/// Performs a tight mmap/munmap loop under heavy contention. Workers exceeding the scoreboard
/// capacity must wait for a slot via the semaphore, stressing the backpressure mechanism.
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
fn saturation_worker_impl(worker_id: usize) -> Result<usize, Error> {
    let pid: ProcessIdentifier = getpid()?;
    let region_base: usize = worker_region_base(worker_id)?;

    for iteration in 0..SATURATION_ITERATIONS {
        let addr_raw: usize = region_base + (iteration * PAGE_BYTES);
        let addr: VirtualAddress = VirtualAddress::from_raw_value(addr_raw);

        mmap(pid, addr, AccessPermission::RDWR)?;
        let byte_raw: usize = (worker_id ^ iteration) & BYTE_MASK;
        let byte: u8 = u8::try_from(byte_raw)
            .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "byte overflow"))?;
        unsafe {
            let ptr: *mut u8 = exposed_addr_to_mut_u8(addr_raw);
            ptr.write_volatile(byte);
            let readback: u8 = ptr.read_volatile();
            if readback != byte {
                return Err(Error::new(ErrorCode::InvalidArgument, "memory readback mismatch"));
            }
        }
        munmap(pid, addr)?;

        PROGRESS.fetch_add(1, Ordering::AcqRel);

        if (iteration ^ worker_id) & YIELD_MASK == 0 {
            sched_yield()?;
        }
    }

    Ok(SATURATION_ITERATIONS)
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Resets the shared atomic counters before each test scenario.
///
fn reset_globals() {
    PROGRESS.store(0, Ordering::Relaxed);
}

///
/// # Description
///
/// Validates that a worker returned the expected iteration count.
///
/// # Parameters
///
/// - `retval`: Value returned by the worker thread.
/// - `expected`: Expected number of successful iterations.
///
/// # Returns
///
/// `Ok(())` when validation passes.
///
/// # Errors
///
/// Returns an error if the worker reported a failure or an iteration count mismatch.
///
fn check_worker_result(retval: usize, expected: usize) -> Result<(), StressError> {
    if retval >= WORKER_ERROR_BASE {
        let code: ErrorCode = error_code_from_usize(retval.wrapping_sub(WORKER_ERROR_BASE));
        return Err(Error::new(code, "scoreboard worker failed"));
    }

    if retval != expected {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "scoreboard worker iterations mismatch",
        ));
    }

    Ok(())
}

///
/// # Description
///
/// Validates that the global progress counter matches the expected total.
///
/// # Parameters
///
/// - `expected`: Expected total progress.
///
/// # Returns
///
/// `Ok(())` when validation passes.
///
/// # Errors
///
/// Returns an error if the progress counter does not match.
///
fn check_progress(expected: usize) -> Result<(), StressError> {
    let observed: usize = PROGRESS.load(Ordering::Acquire);
    if observed != expected {
        return Err(Error::new(ErrorCode::InvalidArgument, "scoreboard progress mismatch"));
    }
    Ok(())
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
        .checked_mul(REGION_STRIDE_BYTES)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "worker offset overflow"))?;

    user_base
        .checked_add(offset)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "virtual address overflow"))
}

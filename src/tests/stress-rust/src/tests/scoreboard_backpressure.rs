// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    WorkerStack,
    error_code_from_usize,
    error_code_to_usize,
    exposed_addr_to_mut_u8,
    thread_args,
};
use ::alloc::vec::Vec;
use ::config::constants::KILOBYTE;
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
            capctl,
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
// Structures
//==================================================================================================

struct CapabilityGuard {
    capability: Capability,
    released: bool,
}

impl CapabilityGuard {
    fn enable(capability: Capability) -> Result<Self, StressError> {
        capctl(capability, true)?;
        Ok(Self {
            capability,
            released: false,
        })
    }

    fn disable(&mut self) -> Result<(), StressError> {
        if !self.released {
            capctl(self.capability, false)?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for CapabilityGuard {
    fn drop(&mut self) {
        if !self.released {
            let _ = capctl(self.capability, false);
            self.released = true;
        }
    }
}

//==================================================================================================
// Constants
//==================================================================================================

const SCOREBOARD_PRESSURE_ITERATIONS: usize = 48;
const SCOREBOARD_SLOT_HINT: usize = 1;
const SCOREBOARD_PRESSURE_PAGE_BYTES: usize = 4 * KILOBYTE;
const SCOREBOARD_PRESSURE_REGION_STRIDE_BYTES: usize =
    SCOREBOARD_PRESSURE_PAGE_BYTES * SCOREBOARD_PRESSURE_ITERATIONS * 2;
const SCOREBOARD_PRESSURE_YIELD_MASK: usize = 0x7;
const SCOREBOARD_PRESSURE_BYTE_MASK: usize = 0xff;
const MIN_SCOREBOARD_PRESSURE_WORKERS: usize = 8;
const SCOREBOARD_PRESSURE_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static SCOREBOARD_PRESSURE_PROGRESS: AtomicUsize = AtomicUsize::new(0);
static SCOREBOARD_PRESSURE_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Launches enough mmap/munmap workers to over-subscribe the kernel call scoreboard and verify
/// that concurrent rendezvous slots make forward progress.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread or kernel call operations fail.
///
pub fn run() -> Result<(), StressError> {
    SCOREBOARD_PRESSURE_PROGRESS.store(0, Ordering::Relaxed);
    SCOREBOARD_PRESSURE_ERROR_CODE.store(0, Ordering::Relaxed);

    let mut capability_guard: CapabilityGuard =
        CapabilityGuard::enable(Capability::MemoryManagement)?;

    let worker_count: usize = cmp::max(SCOREBOARD_SLOT_HINT * 2, MIN_SCOREBOARD_PRESSURE_WORKERS);

    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(worker_count);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, scoreboard_pressure_worker, worker_id);
        let tid: ThreadIdentifier = create_thread(&mut args)?;
        tids.push(tid);
        stacks.push(stack);
    }

    for (tid, stack) in tids.into_iter().zip(stacks.into_iter()) {
        let mut retval: usize = 0;
        join_thread(tid, &mut retval)?;
        drop(stack);

        if retval == SCOREBOARD_PRESSURE_FAILURE {
            let encoded: usize = SCOREBOARD_PRESSURE_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code = error_code_from_usize(encoded);
            return Err(Error::new(code, "scoreboard pressure worker failed"));
        }

        if retval != SCOREBOARD_PRESSURE_ITERATIONS {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "scoreboard pressure worker iterations mismatch",
            ));
        }
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
/// Number of successful iterations performed or `SCOREBOARD_PRESSURE_FAILURE` on error.
///
extern "C" fn scoreboard_pressure_worker(worker_id: usize) -> usize {
    match scoreboard_pressure_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => {
            SCOREBOARD_PRESSURE_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            SCOREBOARD_PRESSURE_FAILURE
        },
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

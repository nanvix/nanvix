// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    CapabilityGuard,
    StressError,
    exposed_addr_to_mut_u8,
};
use ::config::constants::KILOBYTE;
use ::core::convert::TryFrom;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm::{
            __kcall_mmap,
            __kcall_mprotect,
            __kcall_munmap,
        },
        pm::getpid_uncached,
    },
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const MMAP_STRESS_PAGES: usize = 24;
const MMAP_STRESS_STRIDE_BYTES: usize = 4 * KILOBYTE;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Thrashes mmap/mprotect/munmap across a sliding window of user VA space to mimic allocator or JIT
/// engines that rapidly map and unmap pages with changing protections.
///
/// # Returns
///
/// `Ok(())` on success or an error if capability or memory mapping calls fail.
///
pub fn run() -> Result<(), StressError> {
    let mut cap_guard: CapabilityGuard = CapabilityGuard::enable(Capability::MemoryManagement)?;

    let pid: ProcessIdentifier = getpid_uncached()?;

    // Reserve address space from the unified bump allocator so we don't conflict with the heap
    // region.
    let region_size: usize = MMAP_STRESS_PAGES * MMAP_STRESS_STRIDE_BYTES;
    let region_base: VirtualAddress = ::syscall::sys::mman::mmap_reserve(region_size)?;
    let mut current_addr: VirtualAddress = region_base;

    for iteration in 0..MMAP_STRESS_PAGES {
        let addr: VirtualAddress = current_addr;
        let perm: AccessPermission = if iteration & 0x1 == 0 {
            AccessPermission::RDWR
        } else {
            AccessPermission::RDONLY
        };

        __kcall_mmap(pid, addr, 1, perm)?;

        if perm.is_writable() {
            __kcall_mprotect(pid, addr, AccessPermission::RDONLY)?;
            __kcall_mprotect(pid, addr, AccessPermission::RDWR)?;

            let raw_addr: usize = usize::from(addr);
            let iteration_byte: u8 = u8::try_from(iteration)
                .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "iteration overflow"))?;
            unsafe {
                let ptr: *mut u8 = exposed_addr_to_mut_u8(raw_addr);
                ptr.write_volatile(iteration_byte ^ 0x5a);
            }
        }

        __kcall_munmap(pid, addr)?;

        let next_raw: usize = usize::from(addr);
        current_addr = VirtualAddress::from_raw_value(next_raw + MMAP_STRESS_STRIDE_BYTES);
    }

    cap_guard.disable()?;
    Ok(())
}

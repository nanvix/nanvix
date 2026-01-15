// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    exposed_addr_to_mut_u8,
};
use ::config::constants::KILOBYTE;
use ::core::convert::TryFrom;
use ::sys::{
    config::memory_layout::USER_MMAP_BASE,
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm::{
            mmap,
            mprotect,
            munmap,
        },
        pm::{
            capctl,
            getpid,
        },
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

    let pid: ProcessIdentifier = getpid()?;
    let mut current_addr: VirtualAddress = USER_MMAP_BASE;

    for iteration in 0..MMAP_STRESS_PAGES {
        let addr: VirtualAddress = current_addr;
        let perm: AccessPermission = if iteration & 0x1 == 0 {
            AccessPermission::RDWR
        } else {
            AccessPermission::RDONLY
        };

        mmap(pid, addr, perm)?;

        if perm.is_writable() {
            mprotect(pid, addr, AccessPermission::RDONLY)?;
            mprotect(pid, addr, AccessPermission::RDWR)?;

            let raw_addr: usize = usize::from(addr);
            let iteration_byte: u8 = u8::try_from(iteration)
                .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "iteration overflow"))?;
            unsafe {
                let ptr: *mut u8 = exposed_addr_to_mut_u8(raw_addr);
                ptr.write_volatile(iteration_byte ^ 0x5a);
            }
        }

        munmap(pid, addr)?;

        let next_raw: usize = usize::from(addr);
        current_addr = VirtualAddress::from_raw_value(next_raw + MMAP_STRESS_STRIDE_BYTES);
    }

    cap_guard.disable()?;
    Ok(())
}

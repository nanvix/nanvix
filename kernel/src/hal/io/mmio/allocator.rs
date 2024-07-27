// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    io::IoMemoryRegion,
    mem::{
        TruncatedMemoryRegion,
        VirtualAddress,
    },
};
use ::alloc::collections::linked_list::LinkedList;
use ::kcall::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct IoMemoryAllocator {
    regions: LinkedList<IoMemoryRegion>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoMemoryAllocator {
    pub fn new() -> Self {
        Self {
            regions: LinkedList::new(),
        }
    }

    pub fn register(&mut self, region: TruncatedMemoryRegion<VirtualAddress>) -> Result<(), Error> {
        trace!("register(): region={:?}", region);

        // TODO: Check regions overlap.

        // TODO: Keep the list sorted.

        // Check if address is already registered.
        for reg in self.regions.iter() {
            if reg.base() == region.start() {
                let reason: &str = "address already registered";
                error!("register(): {}", reason);
                return Err(Error::new(ErrorCode::EntryExists, &reason));
            }
        }

        self.regions.push_back(IoMemoryRegion::new(region));

        Ok(())
    }

    /// Allocates an I/O address from the memory allocator.
    pub fn allocate(&mut self, addr: VirtualAddress) -> Result<IoMemoryRegion, Error> {
        for region in self.regions.iter() {
            if region.base().into_inner() == addr {
                if region.ref_count() > 1 {
                    let reason: &str = "region already allocated";
                    error!("allocate(): {}", reason);
                    return Err(Error::new(ErrorCode::EntryExists, &reason));
                }

                return Ok(region.clone());
            }
        }

        let reason: &str = "region not registered";
        error!("allocate(): {}", reason);
        Err(Error::new(ErrorCode::NoSuchEntry, &reason))
    }
}

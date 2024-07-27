// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod x86;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::{
        cpu::{
            pic,
            pit,
        },
        mem,
    },
    error::Error,
    hal::{
        arch::x86::{
            cpu::madt::madt::MadtInfo,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            AccessPermission,
            Address,
            MemoryRegionType,
            PageAligned,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
    },
};
use ::alloc::collections::linked_list::LinkedList;

//==================================================================================================
// Exports
//==================================================================================================

pub use x86::{
    forge_user_stack,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: Option<MadtInfo>,
) -> Result<Arch, Error> {
    // Register I/O ports for 8259 PIC.
    ioports.register_read_write(pic::PIC_CTRL_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_DATA_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_CTRL_SLAVE as u16)?;
    ioports.register_read_write(pic::PIC_DATA_SLAVE as u16)?;

    // Register I/O ports from 0x3f8 to 0x3fc as read/write.
    for base in [0x3F8, 0x2F8, 0x3E8, 0x2E8, 0x3E0, 0x2E0, 0x3F0, 0x2F0].iter() {
        for p in [0, 1, 2, 3, 4, 7].iter() {
            ioports.register_read_write(base + p)?;
        }

        // Register read-only ports.
        for p in [5, 6].iter() {
            ioports.register_read_only(base + p)?;
        }
    }

    // Register ports for the PIT.
    ioports.register_read_write(pit::PIT_CTRL)?;
    ioports.register_read_write(pit::PIT_DATA)?;

    // Register memory mapped I/O regions.
    for region in mmio_regions.iter() {
        ioaddresses.register(region.clone())?;
    }

    // Register video display memory.
    let video_display_memory: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
        "video display memory",
        PageAligned::from_raw_value(0xb8000)?,
        mem::PAGE_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    ioaddresses.register(video_display_memory.clone())?;
    mmio_regions.push_back(video_display_memory);

    x86::init(ioports, ioaddresses, madt)
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu,
        mem::{
            AccessPermission,
            Address,
            MemoryRegion,
            MemoryRegionType,
            VirtualAddress,
        },
    },
    klib,
};
use ::sys::{
    config,
    error::Error,
};

extern "C" {
    static __TEXT_START: u8;
    static __TEXT_END: u8;
    static __RODATA_START: u8;
    static __RODATA_END: u8;
    static __DATA_START: u8;
    static __DATA_END: u8;
    static __BSS_START: u8;
    static __BSS_END: u8;
    static __KERNEL_END: u8;
}

//==================================================================================================
// Structures
//==================================================================================================

pub struct KernelImage {
    text: MemoryRegion<VirtualAddress>,
    data: Option<MemoryRegion<VirtualAddress>>,
    rodata: MemoryRegion<VirtualAddress>,
    bss: MemoryRegion<VirtualAddress>,
    kpool: MemoryRegion<VirtualAddress>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl KernelImage {
    pub fn new() -> Result<Self, Error> {
        // Text section.
        let text_start: usize = (unsafe { &__TEXT_START } as *const u8 as usize);
        let text_end: usize = (unsafe { &__TEXT_END } as *const u8 as usize);
        let text_size: usize = text_end - text_start;
        info!("{:>6}: start={:#010x}, size={:#010x}", "text", text_start, text_size);
        let text = MemoryRegion::new(
            "kernel text",
            VirtualAddress::from_raw_value(text_start)?,
            text_size,
            MemoryRegionType::Reserved,
            AccessPermission::EXEC,
        )?;

        // Read-only data section.
        let rodata_start: usize = (unsafe { &__RODATA_START } as *const u8 as usize);
        let rodata_end: usize = (unsafe { &__RODATA_END } as *const u8 as usize);
        let rodata_size: usize = rodata_end - rodata_start;
        info!("{:>6}: start={:#010x}, size={:#010x}", "rodata", rodata_start, rodata_size);
        let rodata = MemoryRegion::new(
            "kernel read-only data",
            VirtualAddress::from_raw_value(rodata_start)?,
            rodata_size,
            MemoryRegionType::Reserved,
            AccessPermission::RDONLY,
        )?;

        // Data section.
        let data_start: usize = (unsafe { &__DATA_START } as *const u8 as usize);
        let data_end: usize = (unsafe { &__DATA_END } as *const u8 as usize);
        let data_size: usize = data_end - data_start;
        let data: Option<MemoryRegion<VirtualAddress>> = if data_size > 0 {
            info!("{:>6}: start={:#010x}, size={:#010x}", "data", data_start, data_size);
            Some(MemoryRegion::new(
                "kernel data",
                VirtualAddress::from_raw_value(data_start)?,
                data_size,
                MemoryRegionType::Reserved,
                AccessPermission::RDWR,
            )?)
        } else {
            None
        };

        // BSS section.
        let bss_start: usize = (unsafe { &__BSS_START } as *const u8 as usize);
        let bss_end: usize = (unsafe { &__BSS_END } as *const u8 as usize);
        let bss_size: usize = bss_end - bss_start;
        info!("{:>6}: start={:#010x}, size={:#010x}", "bss", bss_start, bss_size);
        let bss = MemoryRegion::new(
            "kernel bss",
            VirtualAddress::from_raw_value(bss_start)?,
            bss_size,
            MemoryRegionType::Reserved,
            AccessPermission::RDWR,
        )?;

        let kernel_end: usize = 0x04000000;
        let kpool_start: usize = klib::align_up(kernel_end, mmu::PGTAB_ALIGNMENT);
        let kpool_size: usize = config::kernel::KPOOL_SIZE;
        let kpool = MemoryRegion::new(
            "kernel page pool",
            VirtualAddress::new(kpool_start),
            kpool_size,
            MemoryRegionType::Reserved,
            AccessPermission::RDWR,
        )?;

        Ok(Self {
            text,
            data,
            rodata,
            bss,
            kpool,
        })
    }

    pub fn text(&self) -> MemoryRegion<VirtualAddress> {
        self.text.clone()
    }

    pub fn data(&self) -> Option<MemoryRegion<VirtualAddress>> {
        self.data.clone()
    }

    pub fn rodata(&self) -> MemoryRegion<VirtualAddress> {
        self.rodata.clone()
    }

    pub fn bss(&self) -> MemoryRegion<VirtualAddress> {
        self.bss.clone()
    }

    pub fn kpool(&self) -> MemoryRegion<VirtualAddress> {
        self.kpool.clone()
    }
}

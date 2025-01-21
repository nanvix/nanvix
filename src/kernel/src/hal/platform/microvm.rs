// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::{
            self,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            MemoryRegion,
            PhysicalAddress,
            TruncatedMemoryRegion,
        },
        platform::{
            bootinfo::BootInfo,
            madt::MadtInfo,
        },
    },
    kmod::KernelModule,
};
use ::alloc::{
    collections::linked_list::LinkedList,
    string::ToString,
};
use ::sys::{
    arch::mem,
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Platform {
    pub arch: Arch,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the 8-bit value `b` to the platform's standard output device.
///
/// # Parameters
///
/// - `b`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn putb(b: u8) {
    ::sys::arch::io::out8(::config::hal::DEFAULT_STDOUT_PORT, b);
}

///
/// # Description
///
/// Places a write request to the platform's standard output device.
///
/// # Parameters
///
/// - `addr`: Address where data should be written from.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
#[cfg(feature = "stdio")]
pub unsafe fn vmbus_write(addr: *const u8) {
    use core::hint;

    #[allow(clippy::unit_arg)]
    hint::black_box(::sys::arch::io::out32(::config::hal::DEFAULT_STDOUT_PORT, addr as u32));
}

///
/// # Description
///
/// Places a read request to the platform's standard input device.
///
/// # Parameters
///
/// - `addr`: Address where data should be read into.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard input device is present.
/// - It assumes that the standard input device was properly initialized.
/// - It does not prevent concurrent access to the standard input device.
///
#[cfg(feature = "stdio")]
pub unsafe fn vmbus_read(addr: *mut u8) {
    use core::hint;

    #[allow(clippy::unit_arg)]
    hint::black_box(::sys::arch::io::out32(::config::hal::DEFAULT_STDIN_PORT, addr as u32))
}

///
/// # Description
///
/// Shutdowns the machine.
///
/// # Return
///
/// This function never returns.
///
pub fn shutdown() -> ! {
    unsafe { ::sys::arch::io::out16(::config::hal::DEFAULT_VMM_PORT, ::config::hal::DEFAULT_VMM_SHUTDOWN_CMD) };
    loop {
        core::hint::spin_loop();
    }
}

///
/// # Description
///
/// Parses boot information.
///
/// # Parameters
///
/// - `magic`: Magic number.
/// - `info`:  Address of the boot information.
///
/// # Returns
///
/// A new boot information structure.
///
pub fn parse_bootinfo(magic: u32, info: usize) -> Result<BootInfo, Error> {
    // Check if magic number matches what we expect.
    if magic != ::config::hal::DEFAULT_BOOT_MAGIC {
        let reason: &str = "invalid boot magic number";
        error!("parse_bootinfo(): magic={:#010x}, info={:#010x} (error={})", magic, info, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    trace!("parse_bootinfo(): magic={:#010x}, info={:#010x}", magic, info);

    // Retrieve initrd information.
    // - Lower 12 bits encode the size of the initrd.
    // - Higher bits encode the base address of the initrd.
    let initrd_size: usize = info & 0xfff;
    let initrd_base: usize = info & !0xfff;

    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();

    // Register initrd as a kernel module.
    if initrd_size != 0 {
        info!(
            "parse_bootinfo(): initrd_base={:#010x}, initrd_size={:#010x}",
            initrd_base, initrd_size
        );

        // Add kernel module to the list of kernel modules.
        let module: KernelModule = KernelModule::new(
            PhysicalAddress::from_raw_value(initrd_base)?,
            initrd_size * mem::PAGE_SIZE,
            "initrd".to_string(),
        );
        kernel_modules.push_back(module);
    }

    Ok(BootInfo::new(None, None, LinkedList::new(), LinkedList::new(), kernel_modules))
}

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    _memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    _mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error> {
    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
    })
}

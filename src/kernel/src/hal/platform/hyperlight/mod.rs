// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod peb;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::{
            self,
            mem::mmu,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            MemoryRegion,
            MemoryRegionType,
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
    arch::{
        mem,
        mem::PAGE_ALIGNMENT,
    },
    config::memory_layout,
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
};
use peb::{
    HyperlightPEB,
    ProcessEnvironmentBlock,
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
/// Writes the string `s` to the platform's standard debug device.
///
/// # Parameters
///
/// - `s`: String to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
///
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn puts(message: &str) {
    let _ = ProcessEnvironmentBlock::puts(message);
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
    let data = core::slice::from_raw_parts(addr, config::kernel::IPC_MESSAGE_SIZE);
    let _ = ProcessEnvironmentBlock::vmbus_write(data);
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
    let data = core::slice::from_raw_parts_mut(addr, config::kernel::IPC_MESSAGE_SIZE);
    let bytes = ProcessEnvironmentBlock::vmbus_read();
    if let Ok(bytes) = bytes {
        data.copy_from_slice(&bytes);
    }
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
    unsafe {
        ::sys::arch::cpu::halt();
    };
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
    if magic != ::config::hyperlight::DEFAULT_BOOT_MAGIC {
        let reason: &str = "invalid boot magic number";
        error!("parse_bootinfo(): magic={:#010x}, info={:#010x} (error={})", magic, info, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    trace!("parse_bootinfo(): magic={:#010x}, info={:#010x}", magic, info);

    // Retrieve initrd information.
    // - Lower 12 bits encode the size of the initrd.
    // - Higher bits encode the base address of the initrd.
    let nzeros: usize = 12; // TODO: change this to INITRD_BASE.trailing_zeros()
    let initrd_size: usize = info & ((1 << nzeros) - 1);
    let initrd_base: usize = info & !((1 << nzeros) - 1);

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
    memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    _mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error> {
    extern "C" {
        static __KERNEL_END: u8;
    }
    // Register PEB structure.
    let peb_base: usize =
        ::sys::mm::align_up(unsafe { &__KERNEL_END } as *const u8 as usize, PAGE_ALIGNMENT);
    const PEB_SIZE: usize = mem::PAGE_SIZE;
    let peb: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "peb",
        VirtualAddress::from_raw_value(peb_base),
        PEB_SIZE,
        MemoryRegionType::Reserved,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(peb);

    // Register host function definitions.
    let host_functions_base: usize = peb_base + PEB_SIZE;
    const HOST_FUNCTIONS_SIZE: usize = mem::PAGE_SIZE;
    let host_functions: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "host functions",
        VirtualAddress::from_raw_value(host_functions_base),
        HOST_FUNCTIONS_SIZE,
        MemoryRegionType::Reserved,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(host_functions);

    // Register host exception handlers.
    let host_exceptions_base: usize = host_functions_base + HOST_FUNCTIONS_SIZE;
    const HOST_EXCEPTIONS_SIZE: usize = 4 * mem::PAGE_SIZE;
    let host_exceptions: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "host exceptions",
        VirtualAddress::from_raw_value(host_exceptions_base),
        HOST_EXCEPTIONS_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(host_exceptions);

    // Register guest error log.
    let guest_error_log_base: usize = host_exceptions_base + HOST_EXCEPTIONS_SIZE;
    const GUEST_ERROR_LOG_SIZE: usize = mem::PAGE_SIZE;
    let guest_error_log: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest error log",
        VirtualAddress::from_raw_value(guest_error_log_base),
        GUEST_ERROR_LOG_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(guest_error_log);

    // Register input data buffer.
    let input_data_base: usize = guest_error_log_base + GUEST_ERROR_LOG_SIZE;
    const INPUT_DATA_BUFFER_SIZE: usize = 4 * mem::PAGE_SIZE;
    let input_data_buffer: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "input data buffer",
        VirtualAddress::from_raw_value(input_data_base),
        INPUT_DATA_BUFFER_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(input_data_buffer);

    // Register output data buffer.
    let output_data_base: usize = input_data_base + INPUT_DATA_BUFFER_SIZE;
    const OUTPUT_DATA_BUFFER_SIZE: usize = 4 * mem::PAGE_SIZE;
    let output_data_buffer: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "output data buffer",
        VirtualAddress::from_raw_value(output_data_base),
        OUTPUT_DATA_BUFFER_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(output_data_buffer);

    // Register guest panic context.
    let guest_panic_context_base: usize = output_data_base + OUTPUT_DATA_BUFFER_SIZE;
    const GUEST_PANIC_CONTEXT_SIZE: usize = mem::PAGE_SIZE;
    let guest_panic_context: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest panic context",
        VirtualAddress::from_raw_value(guest_panic_context_base),
        GUEST_PANIC_CONTEXT_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(guest_panic_context);

    // Register reserved area for heap padding.
    let heap_padding_base: usize = guest_panic_context_base + GUEST_PANIC_CONTEXT_SIZE;
    debug!("heap_padding_base={:#010x}", heap_padding_base);
    let heap_padding_size: usize = memory_layout::KPOOL_BASE.into_raw_value() - heap_padding_base;
    let heap_padding: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "heap padding",
        VirtualAddress::from_raw_value(heap_padding_base),
        heap_padding_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(heap_padding);

    // Register kpool guard page.
    let kpool_guard_base: usize =
        memory_layout::KPOOL_BASE.into_raw_value() + config::kernel::KPOOL_SIZE;
    let kpool_guard_size: usize = mem::PAGE_SIZE;
    let kpool_guard: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "kpool guard",
        VirtualAddress::from_raw_value(kpool_guard_base),
        kpool_guard_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(kpool_guard);

    // Register hyperlight guest user stack.
    let guest_user_stack_base: usize = kpool_guard_base + kpool_guard_size;
    let guest_user_stack_size: usize = mem::PAGE_SIZE;
    let guest_user_stack: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest user stack",
        VirtualAddress::from_raw_value(guest_user_stack_base),
        guest_user_stack_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_user_stack);

    // Register hyperlight guest user stack guard.
    let guest_user_stack_guard_base: usize = guest_user_stack_base + guest_user_stack_size;
    let guest_user_stack_guard_size: usize = mem::PAGE_SIZE;
    let guest_user_stack_guard: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest user stack guard",
        VirtualAddress::from_raw_value(guest_user_stack_guard_base),
        guest_user_stack_guard_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_user_stack_guard);

    // Register hyperlight kernel stack.
    let guest_kernel_stack_base: usize = guest_user_stack_guard_base + guest_user_stack_guard_size;
    let guest_kernel_stack_size: usize = mem::PAGE_SIZE;
    let guest_kernel_stack: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest kernel stack",
        VirtualAddress::from_raw_value(guest_kernel_stack_base),
        guest_kernel_stack_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_kernel_stack);

    // Register hyperlight kernel stack guard.
    let guest_kernel_stack_guard_base: usize = guest_kernel_stack_base + guest_kernel_stack_size;
    let guest_kernel_stack_guard_size: usize = mem::PAGE_SIZE;
    let guest_kernel_stack_guard: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest kernel stack guard",
        VirtualAddress::from_raw_value(guest_kernel_stack_guard_base),
        guest_kernel_stack_guard_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_kernel_stack_guard);

    // Register hyperlight boot stack.
    let guest_boot_stack_base: usize =
        guest_kernel_stack_guard_base + guest_kernel_stack_guard_size;
    let guest_boot_stack_size: usize = mem::PAGE_SIZE;
    let guest_boot_stack: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest boot stack",
        VirtualAddress::from_raw_value(guest_boot_stack_base),
        guest_boot_stack_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_boot_stack);

    unsafe {
        ProcessEnvironmentBlock::init(peb_base as *mut HyperlightPEB)?;
        ProcessEnvironmentBlock::set_guest_function_dispatch_ptr(0xdeadbeef)?;
    };

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
    })
}

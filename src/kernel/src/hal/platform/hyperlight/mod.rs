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
use ::arch::{
    mem,
    mem::PAGE_ALIGNMENT,
};
use ::hyperlight_common::mem::HyperlightPEB;
use ::sys::{
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
    ProcessEnvironmentBlock,
    GUEST_HANDLE,
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
/// Disables all interrupts on the calling core.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is in a state where interrupts can be
/// disabled.
///
pub(super) unsafe fn disable_interrupts() {
    ::arch::cpu::cli();
}

///
/// # Description
///
/// Enables all interrupts on the calling core.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is in a state where interrupts can be
/// enabled.
///
pub(super) unsafe fn enable_interrupts() {
    // Hyperlight does not have an interrupt chip. Enabling interrupts in this context
    // could lead to undefined behavior or other unintended side effects. Therefore, this
    // function is intentionally left as a no-op.
}

///
/// # Description
///
/// Waits for an interrupt to happen.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is able to receive interrupts.
///
pub(super) unsafe fn wait_for_interrupt() {
    // Hyperlight does not have an interrupt chip. Waiting for interrupts (halt) in this context
    // could lead to undefined behavior or other unintended side effects. Therefore, this function
    // is intentionally left as a no-op.
}

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
    use crate::PERF_VMBUS_WRITE;

    PERF_VMBUS_WRITE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

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
    use crate::PERF_VMBUS_READ;

    PERF_VMBUS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

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
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Return
///
/// This function never returns.
///
pub fn shutdown(_status: usize) -> ! {
    unsafe {
        ::arch::cpu::halt();
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
pub fn parse_bootinfo(_: u32, _: usize) -> Result<BootInfo, Error> {
    trace!("parse_bootinfo()");

    extern "C" {
        static __KERNEL_END: u8;
    }

    let peb_base: usize =
        unsafe { ::sys::mm::align_up(&__KERNEL_END as *const u8 as usize, PAGE_ALIGNMENT) };

    unsafe {
        ProcessEnvironmentBlock::init(peb_base as *mut HyperlightPEB)?;
        ProcessEnvironmentBlock::set_guest_function_dispatch_ptr(0xdeadbeef)?;
    };

    // Read actual size and relocate only that amount
    let (initrd_base, initrd_size) = unsafe {
        match GUEST_HANDLE.peb() {
            Some(peb_ptr) => {
                let current_data_start = (*peb_ptr).init_data.ptr as usize;
                let total_size = (*peb_ptr).init_data.size as usize;

                // Read the actual initrd size from the first INITRD_SIZE_BYTES bytes
                let size_bytes = core::slice::from_raw_parts(
                    current_data_start as *const u8,
                    ::config::hyperlight::INITRD_SIZE_BYTES,
                );
                let actual_initrd_size = u64::from_le_bytes([
                    size_bytes[0],
                    size_bytes[1],
                    size_bytes[2],
                    size_bytes[3],
                    size_bytes[4],
                    size_bytes[5],
                    size_bytes[6],
                    size_bytes[7],
                ]) as usize;

                // The actual initrd data starts after the INITRD_SIZE_BYTES-byte header
                let current_initrd_start =
                    current_data_start + ::config::hyperlight::INITRD_SIZE_BYTES;

                debug!(
                    "initrd: found at 0x{current_initrd_start:08x}, actual size: \
                     {actual_initrd_size} bytes (total allocation: {total_size} bytes)"
                );

                if current_initrd_start != ::config::hyperlight::DEFAULT_INITRD_BASE {
                    let src_ptr = current_initrd_start as *const u8;
                    let dst_ptr = ::config::hyperlight::DEFAULT_INITRD_BASE as *mut u8;

                    core::ptr::copy(src_ptr, dst_ptr, actual_initrd_size);

                    debug!(
                        "parse_bootinfo(): initrd relocated from {current_initrd_start:#010x} to \
                         {:#010x}",
                        ::config::hyperlight::DEFAULT_INITRD_BASE
                    );

                    (::config::hyperlight::DEFAULT_INITRD_BASE, actual_initrd_size)
                } else {
                    (current_initrd_start, actual_initrd_size)
                }
            },
            None => {
                error!("parse_bootinfo(): PEB not initialized");
                return Err(Error::new(ErrorCode::NoSuchDevice, "PEB not initialized"));
            },
        }
    };

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
            initrd_size,
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

    // Register host function definitions
    let host_function_definitions_base: usize = peb_base + PEB_SIZE;
    const HOST_FUNCTION_DEFINITIONS_SIZE: usize = mem::PAGE_SIZE;
    let host_function_definitions: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "host function definitions",
        VirtualAddress::from_raw_value(host_function_definitions_base),
        HOST_FUNCTION_DEFINITIONS_SIZE,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(host_function_definitions);

    // Register input data buffer.
    let input_data_base: usize = host_function_definitions_base + HOST_FUNCTION_DEFINITIONS_SIZE;
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

    // Register reserved area for heap padding.
    let heap_padding_base: usize = output_data_base + OUTPUT_DATA_BUFFER_SIZE;
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

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
    })
}

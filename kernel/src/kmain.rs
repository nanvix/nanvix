// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![feature(allocator_api)] // kheap uses this.
#![feature(panic_info_message)] // kpanic uses this.
#![feature(ptr_sub_ptr)] // slab uses this.
#![feature(pointer_is_aligned)] // mboot uses this.
#![feature(asm_const)] // gdt uses this.
#![feature(const_mut_refs)] // tss uses this.
#![feature(linked_list_remove)] // vmem uses this.
#![feature(linked_list_retain)] // vmem uses this.
#![feature(never_type)] // exit() uses this.
#![no_std]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use crate::{
    hal::{
        arch::x86::cpu::madt::MadtInfo,
        mem::{
            AccessPermission,
            Address,
            MemoryRegion,
            MemoryRegionType,
            VirtualAddress,
        },
        Hal,
    },
    kargs::KernelArguments,
    kimage::KernelImage,
    kmod::KernelModule,
    mm::{
        elf::Elf32Fhdr,
        kheap,
        VirtMemoryManager,
        Vmem,
    },
    pm::ProcessManager,
};
use ::alloc::{
    collections::LinkedList,
    string::String,
};
use ::sys::pm::ProcessIdentifier;
use hal::mem::TruncatedMemoryRegion;

//==================================================================================================
// Modules
//==================================================================================================

#[macro_use]
extern crate arch;

#[macro_use]
mod macros;

mod debug;
mod event;
mod hal;
mod io;
mod ipc;
mod kargs;
mod kcall;
mod kimage;
mod klib;
mod klog;
mod kmod;
mod kpanic;
mod mboot;
mod mm;
mod pm;
mod stdout;
mod uart;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn test() {
    if !crate::hal::mem::test() {
        panic!("memory tests failed");
    }
}

pub fn kernel_magic_string() -> ! {
    let magic_string = "PANIC: Hello World!\n";
    unsafe { crate::stdout::puts(magic_string) }
    loop {
        core::hint::spin_loop();
    }
}

///
/// # Description
///
/// Spawn bootstrap servers.
///
/// # Parameters
///
/// - `mm`: A reference to the virtual memory manager to use.
/// - `pm`: A reference to the process manager to use.
/// - `kmods`: A reference to the list of kernel modules to spawn.
///
/// # Returns
///
/// The number of servers that were successfully spawned.
///
fn spawn_servers(
    mm: &mut VirtMemoryManager,
    pm: &mut ProcessManager,
    kmods: &LinkedList<KernelModule>,
) -> usize {
    let mut count: usize = 0;
    // Spawn all servers.
    for kmod in kmods.iter() {
        let elf: &Elf32Fhdr = Elf32Fhdr::from_address(kmod.start().into_raw_value());
        let pid: ProcessIdentifier = {
            match pm.create_process(mm) {
                Ok(pid) => pid,
                Err(err) => {
                    warn!("failed to create server process: {:?}", err);
                    continue;
                },
            }
        };
        match pm.exec(mm, pid, elf) {
            Ok(_) => {
                count += 1;
            },
            Err(err) => {
                warn!("failed to load server image: {:?}", err);
                unimplemented!("kill server process");
            },
        }

        info!("server {} spawned, pid={:?}", kmod.cmdline(), pid);
    }

    count
}

#[no_mangle]
pub extern "C" fn kmain(kargs: &KernelArguments) {
    assert!(crate::klib::raw_array::test_unmanaged());
    assert!(crate::klib::bitmap::test());

    info!("initializing the kernel...");

    // Initialize the kernel heap.
    if let Err(e) = unsafe { kheap::init() } {
        panic!("failed to initialize kernel heap: {:?}", e);
    }

    test();

    // Parse kernel arguments.
    info!("parsing kernel arguments...");
    let (madt, mut memory_regions, mut mmio_regions, kernel_modules): (
        Option<MadtInfo>,
        LinkedList<MemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        LinkedList<KernelModule>,
    ) = match kargs.parse() {
        Ok(bootinfo) => {
            (bootinfo.madt, bootinfo.memory_regions, bootinfo.mmio_regions, bootinfo.kernel_modules)
        },
        Err(err) => {
            panic!("failed to parse kernel arguments: {:?}", err);
        },
    };

    info!("parsing kernel image...");
    let kimage: KernelImage = match KernelImage::new() {
        Ok(kimage) => kimage,
        Err(err) => {
            panic!("failed to initialize kernel image: {:?}", err);
        },
    };

    // Add kernel image to list of memory regions.
    memory_regions.push_back(kimage.text());
    memory_regions.push_back(kimage.rodata());
    if let Some(data) = kimage.data() {
        memory_regions.push_back(data);
    }
    memory_regions.push_back(kimage.bss());
    memory_regions.push_back(kimage.kpool());

    // Add kernel modules to list of memory regions.
    for module in kernel_modules.iter() {
        let name: String = module.cmdline();
        let start: VirtualAddress = module.start().into_virtual_address();
        let size: usize = module.size();
        let typ: MemoryRegionType = MemoryRegionType::Reserved;
        if let Ok(region) = MemoryRegion::new(&name, start, size, typ, AccessPermission::RDONLY) {
            memory_regions.push_back(region);
        }
    }

    let mut hal: Hal = match hal::init(&mut mmio_regions, madt) {
        Ok(hal) => hal,
        Err(err) => {
            panic!("failed to initialize hardware abstraction layer: {:?}", err);
        },
    };

    // Initialize the memory manager.
    let (root, mut mm): (Vmem, VirtMemoryManager) =
        match mm::init(&kimage, memory_regions, mmio_regions) {
            Ok((root, mm)) => (root, mm),
            Err(err) => {
                panic!("failed to initialize memory manager: {:?}", err);
            },
        };

    let mut pm: ProcessManager = match pm::init(&mut hal, root) {
        Ok(pm) => pm,
        Err(err) => {
            panic!("failed to initialize process manager: {:?}", err);
        },
    };

    if spawn_servers(&mut mm, &mut pm, &kernel_modules) > 0 {
        // Initialize kernel call dispatcher.
        kcall::init();

        // Enable timer interrupts.
        if let Err(e) = hal.intman.unmask(hal::arch::InterruptNumber::Timer) {
            panic!("failed to mask timer interrupt: {:?}", e);
        }

        kcall::handler(hal, mm, pm)
    }

    kernel_magic_string();
}

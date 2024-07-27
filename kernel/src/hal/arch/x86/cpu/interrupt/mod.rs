// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod controller;
mod ioapic;
mod map;
mod number;
mod pic;
mod xapic;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::cpu::{
        eflags::{
            self,
            EflagsRegister,
        },
        madt::{
            MadtEntryIoApicSourceOverride,
            MadtEntryLocalApic,
        },
    },
    error::Error,
    hal::{
        arch::x86::{
            cpu::{
                idt,
                interrupt::{
                    ioapic::UninitIoapic,
                    map::InterruptMap,
                    pic::UninitPic,
                    xapic::UninitXapic,
                },
                madt::madt::MadtInfo,
            },
            mem::gdt,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
            IoMemoryRegion,
        },
        mem::VirtualAddress,
    },
};
use ::alloc::collections::LinkedList;

//==================================================================================================
// Exports
//==================================================================================================

pub use controller::{
    InterruptController,
    InterruptHandler,
};
pub use number::InterruptNumber;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Forges a stack frame that is suitable for returning from an interrupt with privilege transition.
///
/// # Parameters
///
/// - `kernel_stack_top`: Pointer to the top of the kernel stack.
/// - `user_stack_top`: Top address of user stack.
/// - `user_func`: User function.
/// - `kernel_func`: Kernel function.
///
/// # Returns
///
/// A pointer to the forged stack frame.
///
/// # Safety
///
/// Behavior is undefined if any of the following conditions are violated:
///
/// - `stackp` must point to a valid location in memory.
///
pub unsafe fn forge_user_stack(
    kernel_stack_top: *mut u8,
    user_stack_top: usize,
    user_func: usize,
    kernel_func: usize,
) -> *mut u8 {
    let mut stackp: *mut u32 = kernel_stack_top as *mut u32;

    // User DS
    stackp = stackp.offset(-1);
    *stackp = gdt::SegmentSelector::UserData as u32;

    // User ESP
    stackp = stackp.offset(-1);
    *stackp = user_stack_top as u32;

    // EFLAGS
    let mut eflags: EflagsRegister = eflags::EflagsRegister::default();
    eflags.interrupt = eflags::InterruptFlag::Set;
    stackp = stackp.offset(-1);
    *stackp = eflags.into_raw_value();

    // User CS
    stackp = stackp.offset(-1);
    *stackp = gdt::SegmentSelector::UserCode as u32;

    // User EIP
    stackp = stackp.offset(-1);
    *stackp = user_func as u32;

    // Kernel EIP
    stackp = stackp.offset(-1);
    *stackp = kernel_func as u32;

    stackp as *mut u8
}

fn build_interrupt_map(madt: &MadtInfo) -> InterruptMap {
    let interrupt_override: LinkedList<&MadtEntryIoApicSourceOverride> =
        madt.get_ioapic_source_override();
    let mut intmap: InterruptMap = InterruptMap::new();

    // Build the interrupt map.
    for entry in interrupt_override {
        intmap.remap(entry.source, entry.global_sys_int as u8);
    }

    intmap
}

/// Initializes the interrupt controller.
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: Option<MadtInfo>,
) -> Result<InterruptController, Error> {
    info!("initializing interrupt controller...");
    match madt {
        // MADT is present.
        Some(madt) => {
            info!("retriving information from madt");

            // Check if the 8259 PIC is present.
            let pic: Option<UninitPic> = match madt.has_8259_pic() {
                true => {
                    info!("8259 pic found");
                    Some(UninitPic::new(ioports, idt::INT_OFF)?)
                },
                false => {
                    info!("8259 pic not found");
                    None
                },
            };

            // Check if the I/O APIC is present.
            let ioapic: Option<UninitIoapic> = match madt.get_ioapic_info() {
                Some(ioapic_info) => {
                    info!("ioapic found");

                    let id: u8 = ioapic_info.io_apic_id;
                    let addr: u32 = ioapic_info.io_apic_addr;
                    let gsi: u32 = ioapic_info.global_sys_int_base;
                    let base: IoMemoryRegion =
                        ioaddresses.allocate(VirtualAddress::new(addr as usize))?;
                    Some(UninitIoapic::new(idt::INT_OFF, id, base, gsi))
                },
                None => {
                    info!("ioapic not found");
                    None
                },
            };

            // Check if local APIC is present.
            let xapic: Option<UninitXapic> = match madt.get_lapic_info() {
                Some(local_apic_info) => {
                    info!("xapic found");

                    if (local_apic_info.flags & MadtEntryLocalApic::ENABLED) != 0 {
                        if (local_apic_info.flags & MadtEntryLocalApic::ONLINE_CAPABLE) == 0 {
                            info!("cpu is enabled")
                        } else {
                            // This should not happen if MADT is consistent to the spec.
                            unreachable!("xapic is malfunctioning")
                        }
                    } else {
                        if (local_apic_info.flags & MadtEntryLocalApic::ONLINE_CAPABLE) != 0 {
                            info!("cpu is online capable")
                        } else {
                            info!("cpu is disabled")
                        }
                    }

                    // TODO: remove the following assert when we handle multiple local APICs.
                    // CPU 0 must be enabled.
                    assert!(
                        local_apic_info.apic_id == 0
                            || (local_apic_info.flags & MadtEntryLocalApic::ENABLED) != 0
                    );

                    let base: IoMemoryRegion =
                        ioaddresses.allocate(VirtualAddress::new(madt.local_apic_addr as usize))?;
                    Some(UninitXapic::new(local_apic_info.apic_id, base))
                },
                None => {
                    info!("xapic not found");
                    None
                },
            };

            let intmap: InterruptMap = build_interrupt_map(&madt);
            InterruptController::new(pic, xapic, ioapic, intmap)
        },

        // MADT is not present.
        None => {
            info!("madt not present, falling back to 8259 pic");
            let pic: UninitPic = UninitPic::new(ioports, idt::INT_OFF)?;
            let intmap: InterruptMap = InterruptMap::new();
            Ok(InterruptController::new(Some(pic), None, None, intmap)?)
        },
    }
}

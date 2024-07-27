// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::cpu::{
        idt::{
            DescriptorPrivilegeLevel,
            Flags,
            GateType,
            Idte,
            PresentBit,
        },
        idtr::Idtr,
    },
    hal::arch::x86::mem::gdt::SegmentSelector,
};
use core::mem;

extern "C" {
    /// Division-by-Zero Error.
    fn _do_excp0();
    /// Debug Exception.
    fn _do_excp1();
    /// Non-Maskable Interrupt.
    fn _do_excp2();
    /// Breakpoint Exception.
    fn _do_excp3();
    /// Overflow Exception.
    fn _do_excp4();
    /// Bounds Check Exception.
    fn _do_excp5();
    /// Invalid Opcode Exception.
    fn _do_excp6();
    /// Coprocessor Not Available.
    fn _do_excp7();
    /// Double Fault.
    fn _do_excp8();
    /// Coprocessor Segment Overrun.
    fn _do_excp9();
    /// Invalid TSS.
    fn _do_excp10();
    /// Segment Not Present.
    fn _do_excp11();
    /// Stack Segment Fault.
    fn _do_excp12();
    /// General Protection Fault.
    fn _do_excp13();
    /// Page Fault.
    fn _do_excp14();
    /// Reserved.
    fn _do_excp15();
    /// Floating Point Exception.
    fn _do_excp16();
    /// Alignment Check Exception.
    fn _do_excp17();
    /// Machine Check Exception.
    fn _do_excp18();
    /// SIMD Floating Point Exception.
    fn _do_excp19();
    /// Virtualization Exception.
    fn _do_excp20();
    /// Security Exception.
    fn _do_excp30();
    fn _do_hwint0();
    fn _do_hwint1();
    fn _do_hwint2();
    fn _do_hwint3();
    fn _do_hwint4();
    fn _do_hwint5();
    fn _do_hwint6();
    fn _do_hwint7();
    fn _do_hwint8();
    fn _do_hwint9();
    fn _do_hwint10();
    fn _do_hwint11();
    fn _do_hwint12();
    fn _do_hwint13();
    fn _do_hwint14();
    fn _do_hwint15();
    fn _do_kcall();
}

//==================================================================================================
// Macros
//==================================================================================================

macro_rules! idt_entry {
    ( $handler:expr, $dpl:expr, $type:expr) => {
        Idte::new(
            $handler as u32,
            SegmentSelector::KernelCode as u16,
            Flags::new(PresentBit::Present, $dpl, $type),
        )
    };
}

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Offset of the exceptions in the IDT.
///
pub const EXCP_OFF: u8 = 0;

///
/// # Description
///
/// Offset of the hardware interrupts in the IDT.
///
pub const INT_OFF: u8 = 32;

//==================================================================================================
// Global Variables
//==================================================================================================

static mut IDT: [Idte; 256] = unsafe { mem::zeroed() };

static mut IDTR: Idtr = unsafe { mem::zeroed() };

pub unsafe fn init() {
    info!("initializing idt...");

    // Set exception hooks.
    IDT[EXCP_OFF as usize] =
        idt_entry!(_do_excp0, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 1] =
        idt_entry!(_do_excp1, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 2] =
        idt_entry!(_do_excp2, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 3] =
        idt_entry!(_do_excp3, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 4] =
        idt_entry!(_do_excp4, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 5] =
        idt_entry!(_do_excp5, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 6] =
        idt_entry!(_do_excp6, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 7] =
        idt_entry!(_do_excp7, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 8] =
        idt_entry!(_do_excp8, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 9] =
        idt_entry!(_do_excp9, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 10] =
        idt_entry!(_do_excp10, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 11] =
        idt_entry!(_do_excp11, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 12] =
        idt_entry!(_do_excp12, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 13] =
        idt_entry!(_do_excp13, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[EXCP_OFF as usize + 14] =
        idt_entry!(_do_excp14, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    for (_, entry) in IDT
        .iter_mut()
        .enumerate()
        .skip(EXCP_OFF as usize + 15)
        .take(17)
    {
        *entry = idt_entry!(_do_excp15, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    }

    // Set hardware interrupt hooks.
    IDT[INT_OFF as usize] =
        idt_entry!(_do_hwint0, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 1] =
        idt_entry!(_do_hwint1, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 2] =
        idt_entry!(_do_hwint2, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 3] =
        idt_entry!(_do_hwint3, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 4] =
        idt_entry!(_do_hwint4, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 5] =
        idt_entry!(_do_hwint5, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 6] =
        idt_entry!(_do_hwint6, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 7] =
        idt_entry!(_do_hwint7, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 8] =
        idt_entry!(_do_hwint8, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 9] =
        idt_entry!(_do_hwint9, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 10] =
        idt_entry!(_do_hwint10, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 11] =
        idt_entry!(_do_hwint11, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 12] =
        idt_entry!(_do_hwint12, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 13] =
        idt_entry!(_do_hwint13, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 14] =
        idt_entry!(_do_hwint14, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    IDT[INT_OFF as usize + 15] =
        idt_entry!(_do_hwint15, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    // Set system call hook.
    IDT[128] = idt_entry!(_do_kcall, DescriptorPrivilegeLevel::Ring3, GateType::Int32);

    // Load IDT.
    info!("loading idt (base={:p}, size={})", IDT.as_ptr(), mem::size_of_val(&IDT));
    IDTR.init(IDT.as_ptr() as u32, (mem::size_of_val(&IDT)) as u16);
    IDTR.load();
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::mem::gdt::SegmentSelector;
pub use ::arch::cpu::idt::IDTE_ALIGNMENT;
use ::arch::cpu::{
    idt::{
        DescriptorPrivilegeLevel,
        Flags,
        GateType,
        Idte,
        PresentBit,
    },
    idtr::Idtr,
};
use ::core::mem;
use ::sys::error::{
    Error,
    ErrorCode,
};

unsafe extern "C" {
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
            $handler as *const () as usize as u32,
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

///
/// # Description
///
/// Length of the IDT.
///
pub const IDT_LEN: usize = 256;

///
/// # Description
///
/// Size of the IDT in bytes.
///
pub const IDT_SIZE: usize = IDT_LEN * mem::size_of::<Idte>();

///
/// # Description
///
/// Size of the IDTR in bytes.
///
#[allow(dead_code)]
pub const IDTR_SIZE: usize = mem::size_of::<Idtr>();

//==================================================================================================
// Structures
//==================================================================================================

/// Interrupt descriptor table.
pub struct Idt;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Pointer to the platform-provided IDT backing storage.
///
/// Initialized by [`Idt::set_backing_storage()`] before [`init()`]. On microvm the storage
/// is a BSS-allocated static array.
static mut IDT: *mut Idte = core::ptr::null_mut();

/// Pointer to the platform-provided IDTR backing storage.
///
/// Initialized by [`Idt::set_backing_storage()`] before [`init()`]. On microvm the storage
/// is a BSS-allocated static.
static mut IDTR: *mut Idtr = core::ptr::null_mut();

//==================================================================================================
// Implementations
//==================================================================================================

impl Idt {
    ///
    /// # Description
    ///
    /// Installs platform-provided backing storage for the IDT and IDTR.
    ///
    /// Must be called exactly once before [`init()`].
    ///
    /// # Parameters
    ///
    /// - `idt_storage`: Pointer to at least [`IDT_LEN`] contiguous [`Idte`] slots whose
    ///   lifetime exceeds all subsequent IDT operations. Must be aligned to [`IDTE_ALIGNMENT`].
    /// - `idtr_storage`: Pointer to a single [`Idtr`] whose lifetime exceeds all subsequent IDT
    ///   operations.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the backing storage was successfully installed.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::InvalidArgument`] if `idt_storage` is not aligned to [`IDTE_ALIGNMENT`].
    ///
    /// # Safety
    ///
    /// This function is unsafe because it sets global raw pointers that all IDT operations
    /// depend on. The caller must ensure:
    /// - `idt_storage` is non-null and points to at least [`IDT_LEN`] entries.
    /// - `idtr_storage` is non-null and points to a valid [`Idtr`].
    /// - The backing memory outlives all IDT usage.
    /// - This function is called at most once.
    ///
    pub unsafe fn set_backing_storage(
        idt_storage: *mut Idte,
        idtr_storage: *mut Idtr,
    ) -> Result<(), Error> {
        if !::sys::mm::is_aligned(idt_storage as usize, IDTE_ALIGNMENT) {
            let reason: &str = "IDT backing storage pointer is not properly aligned";
            error!("{}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        IDT = idt_storage;
        IDTR = idtr_storage;
        Ok(())
    }
}

pub unsafe fn init() {
    debug_assert!(
        !IDT.is_null(),
        "IDT backing storage not installed; call Idt::set_backing_storage() first"
    );
    debug_assert!(
        !IDTR.is_null(),
        "IDTR backing storage not installed; call Idt::set_backing_storage() first"
    );
    info!("initializing idt...");

    let idt: *mut Idte = IDT;

    // Set exception hooks.
    *idt.add(EXCP_OFF as usize) =
        idt_entry!(_do_excp0, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 1) =
        idt_entry!(_do_excp1, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 2) =
        idt_entry!(_do_excp2, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 3) =
        idt_entry!(_do_excp3, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 4) =
        idt_entry!(_do_excp4, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 5) =
        idt_entry!(_do_excp5, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 6) =
        idt_entry!(_do_excp6, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 7) =
        idt_entry!(_do_excp7, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 8) =
        idt_entry!(_do_excp8, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 9) =
        idt_entry!(_do_excp9, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 10) =
        idt_entry!(_do_excp10, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 11) =
        idt_entry!(_do_excp11, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 12) =
        idt_entry!(_do_excp12, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 13) =
        idt_entry!(_do_excp13, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(EXCP_OFF as usize + 14) =
        idt_entry!(_do_excp14, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    for i in 0..17 {
        *idt.add(EXCP_OFF as usize + 15 + i) =
            idt_entry!(_do_excp15, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    }

    // Set hardware interrupt hooks.
    *idt.add(INT_OFF as usize) =
        idt_entry!(_do_hwint0, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 1) =
        idt_entry!(_do_hwint1, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 2) =
        idt_entry!(_do_hwint2, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 3) =
        idt_entry!(_do_hwint3, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 4) =
        idt_entry!(_do_hwint4, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 5) =
        idt_entry!(_do_hwint5, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 6) =
        idt_entry!(_do_hwint6, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 7) =
        idt_entry!(_do_hwint7, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 8) =
        idt_entry!(_do_hwint8, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 9) =
        idt_entry!(_do_hwint9, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 10) =
        idt_entry!(_do_hwint10, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 11) =
        idt_entry!(_do_hwint11, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 12) =
        idt_entry!(_do_hwint12, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 13) =
        idt_entry!(_do_hwint13, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 14) =
        idt_entry!(_do_hwint14, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    *idt.add(INT_OFF as usize + 15) =
        idt_entry!(_do_hwint15, DescriptorPrivilegeLevel::Ring0, GateType::Int32);
    // Set system call hook.
    *idt.add(::sys::number::KCALL_VECTOR as usize) =
        idt_entry!(_do_kcall, DescriptorPrivilegeLevel::Ring3, GateType::Int32);

    // Load IDT.
    (*IDTR).init(idt as u32, u16::try_from(IDT_SIZE).expect("wrong idt size, is it corrupted?"));
    load()
}

///
/// # Description
///
/// Loads the IDT.
pub unsafe fn load() {
    debug_assert!(
        !IDT.is_null(),
        "IDT backing storage not installed; call Idt::set_backing_storage() first"
    );
    debug_assert!(
        !IDTR.is_null(),
        "IDTR backing storage not installed; call Idt::set_backing_storage() first"
    );
    info!("loading idt (base={:p}, size={})", IDT, IDT_SIZE);
    (*IDTR).load();
}

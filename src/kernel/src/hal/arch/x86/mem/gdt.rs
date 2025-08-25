// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::tss::TssRef;
use ::alloc::boxed::Box;
use ::arch::mem::gdt::{
    AccessAccessed,
    AccessConforming,
    AccessDescriptorType,
    AccessDirection,
    AccessDirectionConforming,
    AccessExecutable,
    AccessPresent,
    AccessReadWrite,
    AccessReadable,
    AccessWritable,
    DescriptorPrivilegeLevel,
    Gdte,
    GdteAccessByte,
    GdteFlags,
    GdteGranularity,
    GdteLongMode,
    GdteProtectedMode,
};
use ::core::{
    arch,
    mem,
    pin::Pin,
};
use ::sys::error::Error;

//==================================================================================================
// Structures
//==================================================================================================

/// Global descriptor table.
pub struct Gdt;

/// Global descriptor table pointer.
pub struct GdtPtr(Pin<Box<::arch::mem::gdtr::Gdtr>>);

/// Global descriptor table entries.
#[derive(Debug)]
#[repr(u8)]
enum GdtEntries {
    Null = 0,
    KernelCode = 1,
    KernelData = 2,
    UserCode = 3,
    UserData = 4,
    UserThreadDataArea = 5,
    Tss = 6,
}

/// Segment selector.
#[repr(u8)]
pub enum SegmentSelector {
    Null = (GdtEntries::Null as u8) << 3,
    KernelCode = (GdtEntries::KernelCode as u8) << 3,
    KernelData = (GdtEntries::KernelData as u8) << 3,
    UserCode = ((GdtEntries::UserCode as u8) << 3) | 3,
    UserData = ((GdtEntries::UserData as u8) << 3) | 3,
    UserThreadDataArea = ((GdtEntries::UserThreadDataArea as u8) << 3) | 3,
    Tss = (GdtEntries::Tss as u8) << 3,
}

//===================================================================================================
// Global Variables
//===================================================================================================

/// Global descriptor table.
static mut GDT: [Gdte; 7] = [
    // Null entry.
    Gdte::new(
        0x0,
        0x0,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::DataSegment(AccessWritable::Readonly),
            AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
            AccessExecutable::Data,
            AccessDescriptorType::System,
            DescriptorPrivilegeLevel::Ring0,
            AccessPresent::NotPresent,
        ),
        GdteFlags::new(
            GdteGranularity::ByteGranularity,
            GdteProtectedMode::ProtectedMode16,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // Kernel code entry.
    Gdte::new(
        0x0,
        0xfffff,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::CodeSegment(AccessReadable::Readable),
            AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
            AccessExecutable::Code,
            AccessDescriptorType::CodeData,
            DescriptorPrivilegeLevel::Ring0,
            AccessPresent::Present,
        ),
        GdteFlags::new(
            GdteGranularity::PageGranularity,
            GdteProtectedMode::ProtectedMode32,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // Kernel data entry.
    Gdte::new(
        0x0,
        0xfffff,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::DataSegment(AccessWritable::ReadWrite),
            AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
            AccessExecutable::Data,
            AccessDescriptorType::CodeData,
            DescriptorPrivilegeLevel::Ring0,
            AccessPresent::Present,
        ),
        GdteFlags::new(
            GdteGranularity::PageGranularity,
            GdteProtectedMode::ProtectedMode32,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // User code entry.
    Gdte::new(
        0x0,
        0xfffff,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::CodeSegment(AccessReadable::Readable),
            AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
            AccessExecutable::Code,
            AccessDescriptorType::CodeData,
            DescriptorPrivilegeLevel::Ring3,
            AccessPresent::Present,
        ),
        GdteFlags::new(
            GdteGranularity::PageGranularity,
            GdteProtectedMode::ProtectedMode32,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // User data entry.
    Gdte::new(
        0x0,
        0xfffff,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::DataSegment(AccessWritable::ReadWrite),
            AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
            AccessExecutable::Data,
            AccessDescriptorType::CodeData,
            DescriptorPrivilegeLevel::Ring3,
            AccessPresent::Present,
        ),
        GdteFlags::new(
            GdteGranularity::PageGranularity,
            GdteProtectedMode::ProtectedMode32,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // Entry for user-space thread data area.
    Gdte::new(
        0x0,
        0xfffff,
        GdteAccessByte::new(
            AccessAccessed::NotAccessed,
            AccessReadWrite::DataSegment(AccessWritable::ReadWrite),
            AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
            AccessExecutable::Data,
            AccessDescriptorType::CodeData,
            DescriptorPrivilegeLevel::Ring3,
            AccessPresent::Present,
        ),
        GdteFlags::new(
            GdteGranularity::PageGranularity,
            GdteProtectedMode::ProtectedMode32,
            GdteLongMode::CompatibilityMode,
        ),
    ),
    // Task segment selector entry (overwritten at system initialization).
    Gdte::default(),
];

//==================================================================================================
// Implementations
//==================================================================================================

impl Gdt {
    #[inline(never)]
    pub unsafe fn load(ptr: *const ::arch::mem::gdtr::Gdtr) {
        // No data is pushed the stack, or write to the stack red-zone
        arch::asm!(
            "movl {ptr}, %eax",
            "lgdt (%eax)",
            "ljmp ${KERNEL_CS}, $2f",
            "2:",
            "movw ${NULL}, %ax",
            "movw %ax, %fs",
            "movw %ax, %gs",
            "movw ${KERNEL_DS}, %ax",
            "movw %ax, %ds",
            "movw %ax, %es",
            "movw %ax, %ss",
            ptr = in(reg) ptr,
            NULL = const SegmentSelector::Null as u16,
            KERNEL_CS = const SegmentSelector::KernelCode as u16,
            KERNEL_DS = const SegmentSelector::KernelData as u16,
            options(nostack, att_syntax)
        );
    }

    pub unsafe fn init(kstack: *const u8) -> Result<(GdtPtr, TssRef), Error> {
        trace!("initializing gdt...");
        let ss0: u32 = SegmentSelector::KernelData as u32;
        let esp0: u32 = kstack as u32;
        trace!("ss0={:x}, esp0={:x}", ss0, esp0);
        let tss: TssRef = TssRef::new(ss0, esp0)?;

        // Overwrite task segment selector entry.
        GDT[GdtEntries::Tss as usize] = Gdte::new(
            tss.address() as u32,
            tss.size() as u32 - 1,
            GdteAccessByte::new(
                AccessAccessed::Accessed,
                AccessReadWrite::DataSegment(AccessWritable::Readonly),
                AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
                AccessExecutable::Code,
                AccessDescriptorType::System,
                DescriptorPrivilegeLevel::Ring0,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::ByteGranularity,
                GdteProtectedMode::ProtectedMode16,
                GdteLongMode::CompatibilityMode,
            ),
        );

        // Set the GDTPTR.
        let gdtr = GdtPtr(Pin::new(Box::new(::arch::mem::gdtr::Gdtr::new(
            GDT.as_ptr() as u32,
            (mem::size_of_val(&GDT)) as u16,
        ))));

        info!("loading the GDT...");
        Self::load(gdtr.0.as_ref().get_ref() as *const ::arch::mem::gdtr::Gdtr);

        // Load the TSS.
        tss.load(SegmentSelector::Tss as u16);

        Ok((gdtr, tss))
    }

    ///
    /// # Description
    ///
    /// Sets the base address for the user-space thread data area segment.
    ///
    /// # Parameters
    ///
    /// `tda_base`: The base address of the user-space thread data area segment.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it modifies the global descriptor table.
    ///
    /// It is safe to use this function if and only if the processor is running in privileged mode.
    ///
    pub unsafe fn set_thread_data_area(tda_base: u32) {
        GDT[GdtEntries::UserThreadDataArea as usize].set_base(tda_base);
    }
}

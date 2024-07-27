// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::boxed::Box;

use crate::{
    arch::mem::gdt::{
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
        DescriptorPrivilegeLegel,
        Gdte,
        GdteAccessByte,
        GdteFlags,
        GdteGranularity,
        GdteLongMode,
        GdteProtectedMode,
    },
    error::Error,
    hal::arch::x86::cpu::tss::TssRef,
};
use ::core::{
    arch,
    mem,
    pin::Pin,
};

//==================================================================================================
// Global Variables
//==================================================================================================

pub struct Gdt(Pin<Box<[Gdte; 6]>>);

pub struct GdtPtr(Pin<Box<::arch::mem::gdtr::Gdtr>>);

pub unsafe fn load(ptr: *const ::arch::mem::gdtr::Gdtr) {
    // No data is pushed the stack, or write to the stack red-zone
    arch::asm!(
        "movl {ptr}, %eax",
        "lgdt (%eax)",
        "ljmp ${KERNEL_CS}, $2f",
        "2:",
        "movw ${KERNEL_DS}, %ax",
        "movw %ax, %ds",
        "movw %ax, %es",
        "movw %ax, %fs",
        "movw %ax, %gs",
        "movw %ax, %ss",
        ptr = in(reg) ptr,
        KERNEL_CS = const SegmentSelector::KernelCode as u16,
        KERNEL_DS = const SegmentSelector::KernelData as u16,
        options(nostack, att_syntax)
    );
}

/// Global descriptor table entries.
#[derive(Debug)]
#[repr(u8)]
enum GdtEntries {
    Null = 0,
    KernelCode = 1,
    KernelData = 2,
    UserCode = 3,
    UserData = 4,
    Tss = 5,
}

/// Segment selector.
#[repr(u8)]
pub enum SegmentSelector {
    _Null = (GdtEntries::Null as u8) << 3,
    KernelCode = (GdtEntries::KernelCode as u8) << 3,
    KernelData = (GdtEntries::KernelData as u8) << 3,
    UserCode = ((GdtEntries::UserCode as u8) << 3) | 3,
    UserData = ((GdtEntries::UserData as u8) << 3) | 3,
    Tss = (GdtEntries::Tss as u8) << 3,
}

pub unsafe fn init(kstack: *const u8) -> Result<(Gdt, GdtPtr, TssRef), Error> {
    trace!("initializing gdt...");
    let ss0: u32 = SegmentSelector::KernelData as u32;
    let esp0: u32 = kstack as u32;
    trace!("ss0={:x}, esp0={:x}", ss0, esp0);
    let tss: TssRef = TssRef::new(ss0, esp0)?;

    let gdt = Gdt(Pin::new(Box::new([
        // Null GDTE.
        Gdte::new(
            0x0,
            0x0,
            GdteAccessByte::new(
                AccessAccessed::NotAccessed,
                AccessReadWrite::DataSegment(AccessWritable::Readonly),
                AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
                AccessExecutable::Data,
                AccessDescriptorType::System,
                DescriptorPrivilegeLegel::Ring0,
                AccessPresent::NotPresent,
            ),
            GdteFlags::new(
                GdteGranularity::ByteGranularity,
                GdteProtectedMode::ProtectedMode16,
                GdteLongMode::CompatibilityMode,
            ),
        ),
        // Kernel code GDTE.
        Gdte::new(
            0x0,
            0xfffff,
            GdteAccessByte::new(
                AccessAccessed::NotAccessed,
                AccessReadWrite::CodeSegment(AccessReadable::Readable),
                AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
                AccessExecutable::Code,
                AccessDescriptorType::CodeData,
                DescriptorPrivilegeLegel::Ring0,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::PageGranularity,
                GdteProtectedMode::ProtectedMode32,
                GdteLongMode::CompatibilityMode,
            ),
        ),
        // Kernel data GDTE.
        Gdte::new(
            0x0,
            0xfffff,
            GdteAccessByte::new(
                AccessAccessed::NotAccessed,
                AccessReadWrite::DataSegment(AccessWritable::ReadWrite),
                AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
                AccessExecutable::Data,
                AccessDescriptorType::CodeData,
                DescriptorPrivilegeLegel::Ring0,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::PageGranularity,
                GdteProtectedMode::ProtectedMode32,
                GdteLongMode::CompatibilityMode,
            ),
        ),
        // User code GDTE.
        Gdte::new(
            0x0,
            0xfffff,
            GdteAccessByte::new(
                AccessAccessed::NotAccessed,
                AccessReadWrite::CodeSegment(AccessReadable::Readable),
                AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
                AccessExecutable::Code,
                AccessDescriptorType::CodeData,
                DescriptorPrivilegeLegel::Ring3,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::PageGranularity,
                GdteProtectedMode::ProtectedMode32,
                GdteLongMode::CompatibilityMode,
            ),
        ),
        // User data GDTE.
        Gdte::new(
            0x0,
            0xfffff,
            GdteAccessByte::new(
                AccessAccessed::NotAccessed,
                AccessReadWrite::DataSegment(AccessWritable::ReadWrite),
                AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
                AccessExecutable::Data,
                AccessDescriptorType::CodeData,
                DescriptorPrivilegeLegel::Ring3,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::PageGranularity,
                GdteProtectedMode::ProtectedMode32,
                GdteLongMode::CompatibilityMode,
            ),
        ),
        // TSS GDTE.
        Gdte::new(
            tss.address() as u32,
            tss.size() as u32 - 1,
            GdteAccessByte::new(
                AccessAccessed::Accessed,
                AccessReadWrite::DataSegment(AccessWritable::Readonly),
                AccessDirectionConforming::Conforming(AccessConforming::NonConforming),
                AccessExecutable::Code,
                AccessDescriptorType::System,
                DescriptorPrivilegeLegel::Ring0,
                AccessPresent::Present,
            ),
            GdteFlags::new(
                GdteGranularity::ByteGranularity,
                GdteProtectedMode::ProtectedMode16,
                GdteLongMode::CompatibilityMode,
            ),
        ),
    ])));

    // Set the GDTPTR.
    let gdtr = GdtPtr(Pin::new(Box::new(::arch::mem::gdtr::Gdtr::new(
        gdt.0.as_ptr() as u32,
        (mem::size_of_val(&*gdt.0)) as u16,
    ))));

    info!("loading the GDT...");
    load(gdtr.0.as_ref().get_ref() as *const ::arch::mem::gdtr::Gdtr);

    // Load the TSS.
    tss.load(SegmentSelector::Tss as u16);

    Ok((gdt, gdtr, tss))
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86_64::cpu::tss::TssRef;
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
    SystemSegmentDescriptor,
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
///
/// In x86_64, the TSS descriptor is 16 bytes (occupies 2 GDT slots).
/// We store the GDT as raw bytes and overlay entries manually for the TSS.
#[derive(Debug)]
#[repr(u8)]
enum GdtEntries {
    Null = 0,
    KernelCode = 1,
    KernelData = 2,
    UserCode = 3,
    UserData = 4,
    Tss = 5, // TSS occupies slots 5 and 6 (16-byte system segment descriptor)
}

/// Number of 8-byte GDT slots (TSS uses 2 slots).
const GDT_SLOT_COUNT: usize = 7;

/// Segment selector.
#[repr(u16)]
#[allow(dead_code)]
pub enum SegmentSelector {
    Null = (GdtEntries::Null as u16) << 3,
    KernelCode = (GdtEntries::KernelCode as u16) << 3,
    KernelData = (GdtEntries::KernelData as u16) << 3,
    UserCode = ((GdtEntries::UserCode as u16) << 3) | 3,
    UserData = ((GdtEntries::UserData as u16) << 3) | 3,
    Tss = (GdtEntries::Tss as u16) << 3,
}

//===================================================================================================
// Global Variables
//===================================================================================================

/// Global descriptor table (stored as raw 8-byte entries).
/// Slots 0-4 are standard 8-byte descriptors. Slots 5-6 form the 16-byte TSS descriptor.
static mut GDT: [Gdte; GDT_SLOT_COUNT] = [
    // Null entry (slot 0).
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
    // Kernel code entry (slot 1) — 64-bit code segment.
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
            GdteProtectedMode::ProtectedMode16, // Must be 0 for 64-bit code segment
            GdteLongMode::LongMode,
        ),
    ),
    // Kernel data entry (slot 2).
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
    // User code entry (slot 3) — 64-bit code segment.
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
            GdteProtectedMode::ProtectedMode16, // Must be 0 for 64-bit code segment
            GdteLongMode::LongMode,
        ),
    ),
    // User data entry (slot 4).
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
    // TSS descriptor (slots 5-6): placeholder, overwritten at initialization.
    // Slot 5: lower 8 bytes of 16-byte system segment descriptor.
    Gdte::default(),
    // Slot 6: upper 8 bytes of 16-byte system segment descriptor.
    Gdte::default(),
];

//==================================================================================================
// Implementations
//==================================================================================================

impl Gdt {
    #[inline(never)]
    pub unsafe fn load(ptr: *const ::arch::mem::gdtr::Gdtr) {
        arch::asm!(
            "lgdt [{ptr}]",
            // Reload CS via a far return.
            "push {KERNEL_CS}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            // Reload data segment registers.
            "mov ax, {KERNEL_DS}",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "xor ax, ax",
            "mov fs, ax",
            "mov gs, ax",
            ptr = in(reg) ptr,
            KERNEL_CS = const SegmentSelector::KernelCode as u64,
            KERNEL_DS = const SegmentSelector::KernelData as u16,
            tmp = out(reg) _,
            options(nostack)
        );
    }

    pub unsafe fn init(kstack: *const u8) -> Result<(GdtPtr, TssRef), Error> {
        trace!("initializing gdt...");
        let rsp0: u64 = kstack as u64;
        trace!("rsp0={:#018x}", rsp0);
        let tss: TssRef = TssRef::new(rsp0)?;

        // Write the 16-byte TSS system segment descriptor into GDT slots 5-6.
        let tss_base: u64 = tss.address() as u64;
        let tss_limit: u32 = tss.size() as u32 - 1;
        let ssd: SystemSegmentDescriptor = SystemSegmentDescriptor::new(
            tss_base,
            tss_limit,
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

        // Copy the 16-byte SystemSegmentDescriptor into GDT slots 5-6.
        let ssd_ptr: *const u8 = &ssd as *const SystemSegmentDescriptor as *const u8;
        let gdt_tss_ptr: *mut u8 =
            &mut GDT[GdtEntries::Tss as usize] as *mut Gdte as *mut u8;
        core::ptr::copy_nonoverlapping(ssd_ptr, gdt_tss_ptr, mem::size_of::<SystemSegmentDescriptor>());

        // Set the GDTPTR.
        let gdtr = GdtPtr(Pin::new(Box::new(::arch::mem::gdtr::Gdtr::new(
            GDT.as_ptr() as u64,
            (mem::size_of_val(&GDT)) as u16,
        ))));

        info!("loading the GDT...");
        Self::load(gdtr.0.as_ref().get_ref() as *const ::arch::mem::gdtr::Gdtr);

        // Load the TSS.
        tss.load(SegmentSelector::Tss as u16);

        Ok((gdtr, tss))
    }
}

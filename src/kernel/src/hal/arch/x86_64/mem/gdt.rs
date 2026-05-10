// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::tss::TssRef;
use ::alloc::boxed::Box;
pub use ::arch::mem::gdt::GDTE_ALIGNMENT;
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
use ::sys::error::{
    Error,
    ErrorCode,
};

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
    TssLow = 5,
    TssHigh = 6,
}

/// Segment selector.
#[repr(u16)]
pub enum SegmentSelector {
    Null = (GdtEntries::Null as u16) << 3,
    KernelCode = (GdtEntries::KernelCode as u16) << 3,
    KernelData = (GdtEntries::KernelData as u16) << 3,
    UserCode = ((GdtEntries::UserCode as u16) << 3) | 3,
    UserData = ((GdtEntries::UserData as u16) << 3) | 3,
    Tss = (GdtEntries::TssLow as u16) << 3,
}

//===================================================================================================
// Constants
//===================================================================================================

/// Number of entries in the GDT.
pub const GDT_NUM_ENTRIES: usize = 7;

/// Default GDT entries used to populate platform-provided backing storage.
///
/// In x86_64 long mode:
/// - Code segments must have L=1, D=0.
/// - Data segments ignore L and D bits.
/// - The TSS descriptor is 16 bytes (two GDT entries): TssLow + TssHigh.
pub const DEFAULT_ENTRIES: [Gdte; GDT_NUM_ENTRIES] = [
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
    // Kernel code entry (64-bit long mode: L=1, D=0).
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
            GdteProtectedMode::ProtectedMode16,
            GdteLongMode::LongMode,
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
    // User code entry (64-bit long mode: L=1, D=0).
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
            GdteProtectedMode::ProtectedMode16,
            GdteLongMode::LongMode,
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
    // TSS low entry (overwritten at system initialization).
    Gdte::default(),
    // TSS high entry (overwritten at system initialization with upper base bits).
    Gdte::default(),
];

//===================================================================================================
// Global Variables
//===================================================================================================

/// Pointer to the platform-provided GDT backing storage.
///
/// Initialized by [`Gdt::set_backing_storage()`] before [`Gdt::init()`]. On microvm the storage
/// is a BSS-allocated static array.
static mut GDT: *mut Gdte = core::ptr::null_mut();

//==================================================================================================
// Implementations
//==================================================================================================

impl Gdt {
    ///
    /// # Description
    ///
    /// Installs platform-provided backing storage for the GDT.
    ///
    /// The caller is responsible for populating the storage with the correct descriptor entries
    /// (e.g., by copying [`DEFAULT_ENTRIES`]) before this function is called. This function only
    /// records the pointer; it does not write any entries.
    ///
    /// Must be called exactly once before [`Gdt::init()`].
    ///
    /// # Parameters
    ///
    /// - `storage`: Pointer to at least [`GDT_NUM_ENTRIES`] contiguous [`Gdte`] slots whose
    ///   lifetime exceeds all subsequent GDT operations. Must be aligned to [`GDTE_ALIGNMENT`].
    ///
    /// # Returns
    ///
    /// `Ok(())` if the backing storage was successfully installed.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::InvalidArgument`] if `storage` is not aligned to [`GDTE_ALIGNMENT`].
    ///
    /// # Safety
    ///
    /// This function is unsafe because it sets a global raw pointer that all GDT operations
    /// depend on. The caller must ensure:
    /// - `storage` is non-null and points to at least [`GDT_NUM_ENTRIES`] entries.
    /// - The backing memory outlives all GDT usage.
    /// - This function is called at most once.
    ///
    pub unsafe fn set_backing_storage(storage: *mut Gdte) -> Result<(), Error> {
        if !::sys::mm::is_aligned(storage as usize, GDTE_ALIGNMENT) {
            let reason: &str = "GDT backing storage pointer is not properly aligned";
            error!("{}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        GDT = storage;
        Ok(())
    }

    #[inline(never)]
    pub unsafe fn load(ptr: *const ::arch::mem::gdtr::Gdtr) {
        // In x86_64 long mode, we use a far return (lretq) to reload CS because
        // ljmp with 64-bit absolute addresses is not directly encodable.
        arch::asm!(
            "lgdt ({ptr})",
            "pushq ${KERNEL_CS}",
            "leaq 2f(%rip), %rax",
            "pushq %rax",
            "lretq",
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
            out("rax") _,
            options(att_syntax)
        );
    }

    pub unsafe fn init(kstack: *const u8) -> Result<(GdtPtr, TssRef), Error> {
        debug_assert!(
            !GDT.is_null(),
            "GDT backing storage not installed; call set_backing_storage() first"
        );
        trace!("initializing gdt...");
        let rsp0: u64 = kstack as u64;
        trace!("rsp0={:x}", rsp0);
        let tss: TssRef = TssRef::new(rsp0)?;

        // Overwrite TSS low entry (first 8 bytes of the 16-byte TSS descriptor).
        let tss_addr: usize = tss.address();
        let tss_size: usize = tss.size();
        *GDT.add(GdtEntries::TssLow as usize) = Gdte::new(
            tss_addr as u32,
            tss_size as u32 - 1,
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

        // Overwrite TSS high entry (second 8 bytes: upper 32 bits of TSS base address).
        let tss_high_ptr: *mut u32 = GDT.add(GdtEntries::TssHigh as usize) as *mut u32;
        *tss_high_ptr = (tss_addr >> 32) as u32;
        *tss_high_ptr.add(1) = 0;

        // Set the GDTPTR.
        let gdtr = GdtPtr(Pin::new(Box::new(::arch::mem::gdtr::Gdtr::new(
            GDT as u64,
            (GDT_NUM_ENTRIES * mem::size_of::<Gdte>()) as u16,
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
    /// Sets the FS_BASE MSR for user-space thread data area without reloading segment registers.
    /// In x86_64, the FS base is set via MSR rather than GDT segment descriptors.
    ///
    /// # Parameters
    ///
    /// - `tda_base`: The base address of the user-space thread data area segment.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it writes to a model-specific register.
    ///
    /// It is safe to use this function if and only if the processor is running in privileged mode.
    ///
    pub unsafe fn set_thread_data_area_base(tda_base: u64) {
        const IA32_FS_BASE: u32 = 0xC000_0100;
        let eax: u32 = tda_base as u32;
        let edx: u32 = (tda_base >> 32) as u32;
        arch::asm!(
            "wrmsr",
            in("ecx") IA32_FS_BASE,
            in("eax") eax,
            in("edx") edx,
            options(nomem, preserves_flags, nostack),
        );
    }

    ///
    /// # Description
    ///
    /// Sets the FS_BASE MSR for user-space thread data area. In x86_64, this is equivalent to
    /// [`Self::set_thread_data_area_base()`] since there are no segment registers to reload.
    ///
    /// # Parameters
    ///
    /// - `tda_base`: The base address of the user-space thread data area segment.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it writes to a model-specific register.
    ///
    /// It is safe to use this function if and only if the processor is running in privileged mode.
    ///
    pub unsafe fn set_thread_data_area(tda_base: u64) {
        Self::set_thread_data_area_base(tda_base);
    }

    ///
    /// # Description
    ///
    /// Clears the user-space thread data area by zeroing the FS_BASE MSR.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it writes to a model-specific register.
    ///
    /// It is safe to use this function if and only if the processor is running in privileged mode.
    ///
    pub unsafe fn clear_thread_data_area_segments() {
        Self::set_thread_data_area_base(0);
    }
}

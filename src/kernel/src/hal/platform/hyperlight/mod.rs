// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(feature = "microvm", feature = "hyperlight"))]
compile_error!("features \"microvm\" and \"hyperlight\" are mutually exclusive");

pub(crate) mod peb;
mod start;
// `#[macro_use]` is required so that the `scratch_layout!` macro defined
// in this submodule is visible in the parent module without a path prefix.
#[macro_use]
mod scratch_layout;

//==================================================================================================
// Re-exports
//==================================================================================================

/// Returns `true` if the given GVA falls within the scratch region.
pub fn is_scratch_address(gva: usize) -> bool {
    let base: usize = load_scratch_base();
    let end: usize = load_scratch_end();
    if base == 0 {
        return false;
    }
    // Handles wrapping: scratch_end may be 0 when MAX_GVA == usize::MAX.
    if end == 0 {
        gva >= base
    } else {
        gva >= base && gva < end
    }
}

/// Translates a guest virtual address to a guest physical address.
///
/// On i686-guest, some writable snapshot pages are remapped through the scratch
/// region, so the resulting GPA may differ from the original GVA. This function
/// walks the page tables via `pte_walk_gva_to_gpa()` to resolve the actual GPA and
/// falls back to returning `vaddr` unchanged if translation is unavailable.
///
/// This is intended to be used only after the page tables have been set up.
#[inline(always)]
pub fn virt_to_phys(vaddr: usize) -> usize {
    // After eager CoW pre-faulting, writable snapshot pages (kernel pool, BSS, data)
    // are backed by scratch frames whose GPA ≠ GVA. Walk the page tables to resolve
    // the actual GPA. This is essential for values used as CR3 (page directory physical
    // address), where the CPU needs the true GPA, not the identity-mapped GVA.
    //
    // Safety: called after page tables are set up (post-boot).
    unsafe { pte_walk_gva_to_gpa(vaddr) }.unwrap_or(vaddr)
}

/// Returns a pointer to the root (host-built) page directory entries.
///
/// On Hyperlight the host PD is active in CR3. Its GPA is translated to a GVA so the
/// kernel can read the entries through the host page tables.
pub fn get_root_pd_ptr() -> *const PteWord {
    let cr3: u32;
    unsafe {
        core::arch::asm!("mov {:e}, cr3", out(reg) cr3, options(nostack, nomem));
    }
    let host_pd_gpa: usize = (cr3 & PTE_ADDR_MASK_U32) as usize;
    gpa_to_gva(host_pd_gpa) as *const PteWord
}

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "pit")]
use crate::hal::platform::pit::Pit;
use crate::{
    collections::{
        Bitmap,
        RawArray,
    },
    hal::{
        arch::x86::{
            self,
            cpu::idt,
            mem::gdt,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            MemoryRegion,
            MemoryRegionType,
            MmioCachePolicy,
            PageAligned,
            PhysicalAddress,
            TruncatedMemoryRegion,
        },
        platform::{
            bootinfo::BootInfo,
            madt::MadtInfo,
            peb::ProcessEnvironmentBlock,
            region_names::RAMFS_REGION_NAME,
            region_tags::{
                INPUT_BUF_MMIO_TAG,
                OUTPUT_BUF_MMIO_TAG,
                PEB_MMIO_TAG,
                RAMFS_MMIO_TAG,
                SCRATCH_IO_MMIO_TAG,
            },
        },
    },
    kmod::KernelModule,
};
use ::alloc::{
    collections::linked_list::LinkedList,
    format,
    string::{
        String,
        ToString,
    },
    vec,
};
use ::arch::{
    cpu::{
        idt::Idte,
        idtr::Idtr,
    },
    mem,
    mem::{
        gdt::Gdte,
        paging::PteWord,
        PAGE_ALIGNMENT,
        WORD_ALIGNMENT,
    },
};
use ::config::{
    constants::KILOBYTE,
    hyperlight::{
        INITRD_SIZE_BYTES,
        INPUT_DATA_BUFFER_SIZE,
        OUTPUT_DATA_BUFFER_SIZE,
        PEB_SIZE,
    },
    kernel::MEMORY_SIZE,
    memory_layout::KERNEL_BASE_RAW,
};
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use ::hyperlight_common::{
    flatbuffer_wrappers::{
        function_call::FunctionCall,
        function_types::{
            FunctionCallResult,
            ReturnValue,
        },
        guest_error::{
            ErrorCode as GuestErrorCode,
            GuestError,
        },
    },
    layout::{
        MAX_GPA,
        MAX_GVA,
    },
    mem::HyperlightPEB,
    outb::VmAction,
};
use ::hyperlight_guest::guest_handle::handle::GuestHandle;
use ::sparse_bitmap::SparseBitmap;
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

// =================================================================================================
// _nanvix_dispatch — Entry point for guest function calls
// =================================================================================================
//
// Entry point used by guest function calls.
//
// How Hyperlight captures and restores the entry point:
//
// 1. During `evolve()` (Hyperlight's `initialise`), the VM runs until a HLT.  When
//    `hyperlight_pre_kmain()` halts via VmAction::Halt, Hyperlight reads the vCPU registers and
//    stores:
//      - `self.entrypoint = NextAction::Call(regs.rax)` — the address of
//        `_nanvix_dispatch`, which the kernel placed in EAX before halting.
//      - `self.rsp_gva = regs.rsp` — the stack pointer at halt time.
//
// 2. When the host calls `sandbox.call("kmain", ())`, Hyperlight serialises the function name and
//    arguments into the PEB input buffer, then `dispatch_call_from_host()` sets RIP to the stored
//    dispatch address and resumes the VM.
//
// 3. `_nanvix_dispatch` restores the boot stack, reconstructs a `&KernelArguments`
//    pointer, and calls `nanvix_dispatch_function`. That Rust function reads the
//    `FunctionCall` from the PEB input buffer, validates the function name, and
//    dispatches to `kmain`.
//
::core::arch::global_asm!(
    r#".section .text,"ax",@progbits"#,
    ".code32",
    ".extern nanvix_dispatch_function",
    ".globl _nanvix_dispatch",
    "_nanvix_dispatch:",
    // When Hyperlight's `dispatch_call_from_host()` resumes the VM here, all general-purpose
    // registers are zeroed except RSP (restored to the value saved during evolve) and RFLAGS (RES1
    // set; ZF may be set to signal a pending TLB flush). Segment registers are not touched — they
    // retain whatever state was left after `evolve()`.
    //
    // Restore the boot stack from the compile-time scratch address.  The
    // KernelArguments (magic, info) were pushed to (BOOT_STACK_TOP - 8)
    // during the evolve phase.
    "    movl ${BOOT_STACK_TOP}, %esp",
    "    movl %esp, %ebp",
    "    subl $8, %esp",
    // Push the KernelArguments pointer and call the high-level dispatcher.
    "    push %esp",
    "    call nanvix_dispatch_function",
    "    addl $4, %esp",
    "    addl $8, %esp",
    // nanvix_dispatch_function should never return; halt as a safety net.
    "2:  hlt",
    "    jmp 2b",
    BOOT_STACK_TOP = const ::config::memory_layout::HYPERLIGHT_BOOT_STACK_TOP,
    options(att_syntax),
);

//==================================================================================================
// Global Variables
//==================================================================================================

/// Static array to force the kernel image to grow beyond 4 MB.
/// FIXME (#1310): Remove this once memory layout of Hyperlight is fixed.
#[unsafe(no_mangle)]
#[used]
static KERNEL_PADDING: [u8; 2 * 1024 * 1024] = [0u8; 2 * 1024 * 1024];

//==================================================================================================
// Constants
//==================================================================================================

/// Number of page tables needed for identity-mapping physical memory regions.
///
/// On Hyperlight the host page tables are used directly; only a small number of
/// page table slots are needed for process creation.
pub const NUM_PAGE_TABLES: usize = 4;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Platform {
    pub arch: Arch,
    #[cfg(feature = "pit")]
    pub _pit: Pit,
    /// A sparse bitmap representing the physical memory layout, owned by the platform and consumed
    /// by the memory manager during system initialization.
    pub physical_memory_layout: Option<SparseBitmap>,
    /// A bitmap for the kernel page pool, owned by the platform and consumed by the memory manager
    /// during system initialization.
    pub kpool_bitmap: Option<Bitmap>,
}

//==================================================================================================
// Constants
//==================================================================================================

// Ensure the number of kpool pages is a multiple of 8 so the bitmap has no padding bits.
::static_assert::assert_eq!(
    (::config::kernel::KPOOL_SIZE / mem::PAGE_SIZE).is_multiple_of(u8::BITS as usize)
);

// Ensure the config crate's GPA ceiling matches the upstream MAX_GPA from hyperlight-common.
::static_assert::assert_eq!(memory_layout::HYPERLIGHT_GPA_CEILING == MAX_GPA + 1);

//==================================================================================================
// GPA ↔ GVA Translation
//==================================================================================================

/// Translates a guest physical address (GPA) to the corresponding guest virtual address (GVA).
///
/// On i686-guest, the host maps two regions:
///   - **Snapshot** (kernel image): identity-mapped at `BASE_ADDRESS` (0x1000), so GVA == GPA.
///   - **Scratch**: mapped at `GVA = MAX_GVA - scratch_size + 1`, but backed at
///     `GPA = MAX_GPA - scratch_size + 1`. The difference (GVA − GPA) is constant:
///     `MAX_GVA − MAX_GPA` (= 0x0120_0000 for the current layout).
///
/// Page table entries (PDEs, PTEs) store GPAs. Software that walks page tables must convert
/// those GPAs to GVAs before dereferencing them.
///
/// # Parameters
///
/// - `gpa`: A guest physical address (e.g., from a PDE/PTE or CR3).
///
/// # Returns
///
/// The corresponding guest virtual address.
pub fn gpa_to_gva(gpa: usize) -> usize {
    let scratch_base_gpa: usize = MAX_GPA + 1 - cached_scratch_size();
    if gpa >= scratch_base_gpa {
        gpa.wrapping_add(MAX_GVA.wrapping_sub(MAX_GPA))
    } else {
        gpa
    }
}

/// Translates a guest virtual address (GVA) to the corresponding guest physical address (GPA).
///
/// Inverse of [`gpa_to_gva`]. Identity for low memory; subtracts the GVA−GPA delta for scratch.
pub fn gva_to_gpa(gva: usize) -> usize {
    let delta: usize = MAX_GVA.wrapping_sub(MAX_GPA);
    let scratch_base_gpa: usize = MAX_GPA + 1 - cached_scratch_size();
    let scratch_base_gva: usize = scratch_base_gpa.wrapping_add(delta);
    if gva >= scratch_base_gva {
        gva.wrapping_sub(delta)
    } else {
        gva
    }
}

/// Walks the current page tables (from CR3) to resolve a GVA to its actual GPA.
///
/// After eager pre-faulting, PTEs for CoW-resolved pages point to scratch GPAs rather
/// than their original snapshot GPAs. This function reads the PTE to determine the
/// actual physical frame backing the given virtual address.
///
/// # Returns
///
/// The GPA that the page table maps the given GVA to, or `None` if the page is not present.
///
/// # Safety
///
/// Must be called after page tables have been set up (i.e., after boot).
unsafe fn pte_walk_gva_to_gpa(gva: usize) -> Option<usize> {
    use ::arch::mem::paging::PresentFlag;

    let cr3: u32;
    core::arch::asm!("mov {:e}, cr3", out(reg) cr3, options(nostack, nomem));
    let pd_gpa: usize = (cr3 & PTE_ADDR_MASK_U32) as usize;
    let pd_base: *const u32 = gpa_to_gva(pd_gpa) as *const u32;

    let pd_idx: usize = gva >> mem::PGTAB_SHIFT;
    let pt_idx: usize = (gva >> mem::PAGE_SHIFT) & (mem::PAGE_TABLE_LENGTH - 1);
    let page_offset: usize = gva & (mem::PAGE_SIZE - 1);

    let pde: u32 = pd_base.add(pd_idx).read_volatile();
    if !PresentFlag::is_set(pde) {
        return None;
    }

    let pt_gpa: usize = (pde & PTE_ADDR_MASK_U32) as usize;
    let pt_base: *const u32 = gpa_to_gva(pt_gpa) as *const u32;

    let pte: u32 = pt_base.add(pt_idx).read_volatile();
    if !PresentFlag::is_set(pte) {
        return None;
    }

    Some((pte & PTE_ADDR_MASK_U32) as usize + page_offset)
}

/// Returns the total scratch region size by reading the scratch metadata slot.
///
/// This reads the `SCRATCH_TOP_SIZE_OFFSET` u64 at the top of scratch, which the host
/// writes during `update_scratch_bookkeeping()`.
fn scratch_size() -> usize {
    let ptr = ::hyperlight_guest::layout::scratch_size_gva();
    unsafe { core::ptr::read_volatile(ptr) as usize }
}

/// Returns the page table base GPA by reading the scratch metadata slot.
///
/// The host stores this at `SCRATCH_TOP_SNAPSHOT_PT_GPA_BASE_OFFSET` from the
/// top of scratch.
pub(super) fn pt_base_gpa() -> usize {
    let ptr = ::hyperlight_guest::layout::snapshot_pt_gpa_base_gva();
    unsafe { core::ptr::read_volatile(ptr) as usize }
}

/// Returns the first free GPA (bump allocator pointer) from the scratch metadata slot.
///
/// This is `pt_base_gpa + pt_size`, so `first_free_gpa - pt_base_gpa` gives the
/// page table size.
pub fn first_free_scratch_gpa() -> usize {
    let ptr = ::hyperlight_guest::layout::allocator_gva();
    unsafe { core::ptr::read_volatile(ptr) as usize }
}

/// Boot-time first-free-GPA cached before any bump allocation advances the pointer.
static BOOT_FIRST_FREE_GPA: AtomicUsize = AtomicUsize::new(0);

/// Returns the boot-time first-free-GPA value.
///
/// This is set once by `hyperlight_pre_kmain()` before any bump allocation can advance
/// the live allocator pointer.
fn load_boot_first_free_gpa() -> usize {
    BOOT_FIRST_FREE_GPA.load(Ordering::Relaxed)
}

/// Saves the boot-time first-free-GPA value.
pub(super) fn save_boot_first_free_gpa(val: usize) {
    BOOT_FIRST_FREE_GPA.store(val, Ordering::Relaxed);
}

// Scratch-reserved layout.
//
// These structures are allocated in the scratch region (outside the CoW snapshot) so that
// runtime writes do not trigger copy-on-write faults.  The `scratch_layout!` macro generates
// *_OFFSET, *_SIZE constants, per-entry _ptr() accessors, and the page-aligned
// SCRATCH_RESERVED_SIZE constant from the entries below.
scratch_layout! {
    page_align = PAGE_ALIGNMENT;

    /// One bit per frame for the entire `MEMORY_SIZE` address range, plus extra bytes
    /// to account for byte-alignment padding in each sparse bitmap chunk.
    FRAME_ALLOC_BITMAP : size = MEMORY_SIZE / (mem::FRAME_SIZE * u8::BITS as usize) + 8,
                         align = WORD_ALIGNMENT;
    /// Kernel page pool bitmap storage.
    KPOOL_BITMAP       : size = ::config::kernel::KPOOL_SIZE / (mem::PAGE_SIZE * u8::BITS as usize),
                         align = WORD_ALIGNMENT;
    /// GDT backing storage.
    GDT                : size = gdt::GDT_NUM_ENTRIES * core::mem::size_of::<Gdte>(),
                         align = gdt::GDTE_ALIGNMENT;
    /// IDT backing storage.
    IDT                : size = idt::IDT_SIZE,
                         align = idt::IDTE_ALIGNMENT;
    /// IDTR backing storage.
    IDTR               : size = idt::IDTR_SIZE,
                         align = WORD_ALIGNMENT;
    /// Kernel heap backing storage (relocated from BSS to avoid dirtying CoW snapshot pages).
    HEAP_STORAGE       : size = crate::mm::kheap::MIN_HEAP_SIZE,
                         align = PAGE_ALIGNMENT;
    /// Kernel log buffer backing storage (relocated from BSS to avoid dirtying CoW snapshot pages).
    KLOG_BUFFER        : size = crate::klog::KLOG_BUFFER_STORAGE_SIZE,
                         align = crate::klog::KLOG_BUFFER_ALIGNMENT;
}

//==================================================================================================
// Global Variables for Memory Layout
//==================================================================================================

/// Snapshot region base address.
const SNAPSHOT_BASE: usize = ::config::hyperlight::PLATFORM_BASE_ADDR;
/// Snapshot region end address.
static SNAPSHOT_END: AtomicUsize = AtomicUsize::new(0);

/// RAMFS base address.
static RAMFS_BASE: AtomicUsize = AtomicUsize::new(0);
/// RAMFS end address.
static RAMFS_END: AtomicUsize = AtomicUsize::new(0);

/// Scratch region base address.
static SCRATCH_BASE: AtomicUsize = AtomicUsize::new(0);
/// Scratch region end address.
static SCRATCH_END: AtomicUsize = AtomicUsize::new(0);

/// Cached scratch size (set once in `init()`, read by hot-path validators).
/// Zero before `init()` — callers fall back to the volatile read in that case.
static SCRATCH_SIZE_CACHED: AtomicUsize = AtomicUsize::new(0);

/// Returns the snapshot end address, or the default if not yet initialized.
fn load_snapshot_end() -> usize {
    let val: usize = SNAPSHOT_END.load(Ordering::Relaxed);
    if val == 0 {
        KERNEL_BASE_RAW + MEMORY_SIZE
    } else {
        val
    }
}

fn store_snapshot_end(val: usize) {
    SNAPSHOT_END.store(val, Ordering::Relaxed);
}

fn load_ramfs_base() -> usize {
    RAMFS_BASE.load(Ordering::Relaxed)
}

fn store_ramfs_base(val: usize) {
    RAMFS_BASE.store(val, Ordering::Relaxed);
}

fn load_ramfs_end() -> usize {
    RAMFS_END.load(Ordering::Relaxed)
}

fn store_ramfs_end(val: usize) {
    RAMFS_END.store(val, Ordering::Relaxed);
}

fn load_scratch_base() -> usize {
    SCRATCH_BASE.load(Ordering::Relaxed)
}

fn store_scratch_base(val: usize) {
    SCRATCH_BASE.store(val, Ordering::Relaxed);
}

fn load_scratch_end() -> usize {
    SCRATCH_END.load(Ordering::Relaxed)
}

fn store_scratch_end(val: usize) {
    SCRATCH_END.store(val, Ordering::Relaxed);
}

/// Returns the cached scratch size, falling back to the volatile metadata read
/// if `init()` has not yet stored the value.
fn cached_scratch_size() -> usize {
    let cached: usize = SCRATCH_SIZE_CACHED.load(Ordering::Relaxed);
    if cached != 0 {
        cached
    } else {
        scratch_size()
    }
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
    ::arch::cpu::sti();
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
    ::arch::cpu::halt();
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
    use ::sys::ipc::VmBusMessage;

    PERF_VMBUS_WRITE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let vmbus_msg: VmBusMessage = core::ptr::read_unaligned(addr as *const VmBusMessage);

    if vmbus_msg.is_ikc() {
        let message_data: &[u8] = core::slice::from_raw_parts(
            vmbus_msg.message_addr() as *const u8,
            vmbus_msg.size() as usize,
        );
        let _ = ProcessEnvironmentBlock::vmbus_write(message_data);
    } else {
        // Bulk: read header, then copy payload from the user GPA.
        // The GPA points to a user frame that may not be identity-mapped in the
        // kernel's page tables, so use the lazy identity mapper to ensure mappings.
        let header: ::sys::ipc::DataChunkHeader = core::ptr::read_unaligned(
            vmbus_msg.message_addr() as *const ::sys::ipc::DataChunkHeader,
        );
        let header_bytes: [u8; ::sys::ipc::DataChunkHeader::SIZE] = header.to_bytes();

        let data_gpa: usize = header.data_addr() as usize;
        let data_len: usize = header.data_len() as usize;

        // Send header + payload in a single VmbusBulkWrite call.
        // The host-side bulk_output_fn expects [DataChunkHeader][payload] combined.
        let total_len: usize = ::sys::ipc::DataChunkHeader::SIZE + data_len;
        let mut buf: alloc::vec::Vec<u8> = alloc::vec![0u8; total_len];
        buf[..::sys::ipc::DataChunkHeader::SIZE].copy_from_slice(&header_bytes);

        if data_len > 0 {
            let payload_dst: *mut u8 =
                unsafe { buf.as_mut_ptr().add(::sys::ipc::DataChunkHeader::SIZE) };
            // Translate the user data GPA to a GVA the CPU can dereference.
            // Do NOT use crate::mm::memcpy here: it assumes both pointers are GPAs and
            // would double-translate payload_dst (a heap GVA in the scratch region).
            let data_gva: *const u8 = gpa_to_gva(data_gpa) as *const u8;
            unsafe { core::ptr::copy_nonoverlapping(data_gva, payload_dst, data_len) };
        }

        let _ = ProcessEnvironmentBlock::vmbus_bulk_write(&buf);
    }
}

///
/// # Description
///
/// Places a read request to the platform's standard input device.
///
/// If the VmbusRead response is a PullResponse message (containing a [`DataChunkHeader`]),
/// the bulk data payload is fetched in small chunks via the `VmbusBulkRead` host function
/// and copied directly into guest physical memory at the GPA encoded in the header.
/// Each chunk is kept under 400 bytes to fit within the kernel's 512-byte slab allocator
/// after FlatBuffer serialization overhead.
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
    use ::sys::ipc::VmBusMessage;

    PERF_VMBUS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    // Read the vmbus message from the given address.
    let vmbus_msg: VmBusMessage = core::ptr::read_unaligned(addr as *const VmBusMessage);

    // Read the response from the host (just the IPC message, no bulk data concatenated).
    let bytes: Result<alloc::vec::Vec<u8>, _> = ProcessEnvironmentBlock::vmbus_read();
    if let Ok(bytes) = bytes {
        let msg_size: usize = vmbus_msg.size() as usize;
        let copy_len: usize = bytes.len().min(msg_size);
        if copy_len > 0 {
            let dest: &mut [u8] =
                core::slice::from_raw_parts_mut(vmbus_msg.message_addr() as *mut u8, copy_len);
            dest.copy_from_slice(&bytes[..copy_len]);
        }

        // Check if this is a PullResponse that has bulk data to fetch.
        // Parse the DataChunkHeader from the message payload to determine the destination GPA
        // and total data length, then read the data in chunks via VmbusBulkRead.
        let header_offset: usize = ::sys::ipc::Message::HEADER_SIZE;
        if msg_size >= header_offset + ::sys::ipc::DataChunkHeader::SIZE && bytes.len() >= msg_size
        {
            let mut header_bytes: [u8; ::sys::ipc::DataChunkHeader::SIZE] =
                [0u8; ::sys::ipc::DataChunkHeader::SIZE];
            header_bytes.copy_from_slice(
                &bytes[header_offset..header_offset + ::sys::ipc::DataChunkHeader::SIZE],
            );
            if let Ok(header) = ::sys::ipc::DataChunkHeader::try_from_bytes(header_bytes) {
                let dest_gpa: usize = header.data_addr() as usize;
                let total_len: usize = header.data_len() as usize;

                if total_len > 0 {
                    let mut offset: usize = 0;
                    while offset < total_len {
                        match ProcessEnvironmentBlock::vmbus_bulk_read() {
                            Ok(chunk) => {
                                if chunk.is_empty() {
                                    break;
                                }
                                let chunk_len: usize = chunk.len();
                                trace!(
                                    "vmbus_read(): bulk chunk {} bytes to GPA {:#x}+{:#x}",
                                    chunk_len,
                                    dest_gpa,
                                    offset
                                );
                                // Translate the destination GPA to a GVA the CPU can
                                // dereference. Do NOT use crate::mm::memcpy: it would
                                // double-translate the chunk source pointer (a heap GVA).
                                let dest_gva: *mut u8 = gpa_to_gva(dest_gpa + offset) as *mut u8;
                                unsafe {
                                    core::ptr::copy_nonoverlapping(
                                        chunk.as_ptr(),
                                        dest_gva,
                                        chunk_len,
                                    );
                                }
                                offset += chunk_len;
                            },
                            Err(e) => {
                                error!("vmbus_read(): VmbusBulkRead failed: {:?}", e);
                                break;
                            },
                        }
                    }
                }
            }
        }
    }
}

///
/// # Description
///
/// Shuts down the machine.
///
/// Writes a [`FunctionCallResult`] with the exit code to the PEB output buffer so the host can
/// read it through Hyperlight's normal guest function return convention, then halts the VM via
/// [`VmAction::Halt`].
///
/// # Parameters
///
/// - `status`: The shutdown status code (low 8 bits are passed to the VMM).
///
/// # Returns
///
/// This function never returns.
///
pub(in crate::hal::platform) fn do_shutdown(status: usize) -> ! {
    let code: i32 = (status & 0xFF) as i32;

    // Write a proper FunctionCallResult so the host reads the exit code from the PEB output buffer
    // via Hyperlight's standard guest return convention.
    //
    // Safety of `guest_handle()`: The PEB is mapped by the host before guest entry and remains
    // valid for the entire guest lifetime. `do_shutdown()` can only be reached after
    // `hyperlight_pre_kmain()` (evolve phase) has completed — at which point the PEB is fully
    // initialized and the snapshot region is writable (post CoW pre-fault).
    let handle: GuestHandle = guest_handle();
    let fcr = FunctionCallResult::new(Ok(ReturnValue::Int(code)));
    let mut builder = ::flatbuffers::FlatBufferBuilder::new();
    let data = fcr.encode(&mut builder);
    if handle.push_shared_output_data(data).is_err() {
        // PEB output write failed — fall back to abort so the host gets a definite error
        // instead of reading stale data from the output buffer.
        ::hyperlight_guest::exit::abort_with_code(&[code as u8]);
    }

    // Halt the VM cleanly via VmAction::Halt so sandbox.call() returns Ok(exit_code).
    // SAFETY: Port I/O write targets Hyperlight's VmAction::Halt port. This halts the VM and
    // causes sandbox.call() to return on the host. The hlt loop is a safety net in case the
    // out instruction does not immediately stop execution.
    unsafe {
        core::arch::asm!(
            "mov dx, {HALT_PORT}",
            "out dx, al",
            "cli",
            "2: hlt",
            "jmp 2b",
            HALT_PORT = const VmAction::Halt as u16,
            options(noreturn),
        );
    }
}

///
/// # Description
///
/// Signals the VMM that kernel startup is complete and user-space applications are about to start.
///
/// On Hyperlight this is a no-op because the evolve/run lifecycle already
/// communicates boot readiness to the host.
///
pub fn signal_startup_complete() {}

///
/// # Description
///
/// Returns the boot kernel stack top pointer.
///
/// On Hyperlight the boot stack lives in the scratch region at a compile-time
/// constant address derived from [`HYPERLIGHT_MAX_GVA`].
pub fn get_kstack_top() -> *const u8 {
    ::config::memory_layout::HYPERLIGHT_BOOT_STACK_TOP as *const u8
}

///
/// # Description
///
/// Computes the scratch-reserved base address from the PEB and host page table metadata.
///
/// The scratch-reserved region is placed after the IO buffers AND after the host-built
/// page tables (whichever ends later). This ensures our scratch-resident data structures
/// never overlap the host page tables.
///
/// This is safe to call at any point after the PEB has been mapped by the host (i.e., from
/// `init_scratch_kstack` onwards). Only reads from the snapshot region; never writes.
///
fn scratch_reserved_base() -> usize {
    unsafe extern "C" {
        static __KERNEL_END: u8;
    }
    let kernel_end: usize = unsafe { &__KERNEL_END as *const u8 as usize };
    let peb_base: usize = ::sys::mm::align_up(kernel_end, PAGE_ALIGNMENT)
        .expect("scratch_reserved_base(): PEB align_up overflow");
    let peb_ptr: *const HyperlightPEB = peb_base as *const HyperlightPEB;
    let scratch_base: usize = unsafe { (*peb_ptr).input_stack.ptr } as usize;
    let io_buffers_end: usize = scratch_base + INPUT_DATA_BUFFER_SIZE + OUTPUT_DATA_BUFFER_SIZE;
    // Read the boot-time first-free GPA cached by hyperlight_pre_kmain().
    // This value is saved before any CoW bump allocation can modify the live
    // allocator pointer, so it correctly represents the boundary between the
    // host page tables and free scratch space.
    let boot_first_free_gpa: usize = load_boot_first_free_gpa();
    let host_pt_end_gva: usize = if boot_first_free_gpa != 0 {
        gpa_to_gva(boot_first_free_gpa)
    } else {
        // Fallback: read the (possibly stale) live value.
        gpa_to_gva(first_free_scratch_gpa())
    };
    let base: usize = core::cmp::max(io_buffers_end, host_pt_end_gva);
    // Page-align upward.
    ::sys::mm::align_up(base, PAGE_ALIGNMENT)
        .expect("scratch_reserved_base(): page alignment overflow")
}

///
/// # Description
///
/// Returns a raw pointer to the PEB.
///
/// Computed from the `__KERNEL_END` linker symbol, page-aligned upwards.
///
fn peb_ptr() -> *mut HyperlightPEB {
    unsafe extern "C" {
        static __KERNEL_END: u8;
    }
    let kernel_end: usize = unsafe { &__KERNEL_END as *const u8 as usize };
    let peb_base: usize =
        ::sys::mm::align_up(kernel_end, PAGE_ALIGNMENT).expect("peb_ptr(): PEB align_up overflow");
    peb_base as *mut HyperlightPEB
}

///
/// # Description
///
/// Constructs a [`GuestHandle`] from the PEB pointer.
///
/// This is lightweight (no heap allocation) and avoids storing mutable global
/// state that would dirty CoW snapshot pages during early boot.
///
fn guest_handle() -> GuestHandle {
    GuestHandle::init(peb_ptr())
}

/// Allocates a single frame from the scratch bump allocator.
///
/// Reads the current bump pointer from the scratch metadata slot and advances it by one page.
fn bump_alloc_frame() -> u32 {
    let alloc_ptr = ::hyperlight_guest::layout::allocator_gva();
    let gpa: u64 = unsafe { core::ptr::read_volatile(alloc_ptr) };
    // Guard against bump pointer exceeding the scratch region (would corrupt host memory).
    debug_assert!(
        (gpa as usize) < MAX_GPA + 1,
        "bump_alloc_frame(): bump pointer {:#x} exceeds MAX_GPA {:#x}",
        gpa,
        MAX_GPA
    );
    unsafe {
        core::ptr::write_volatile(alloc_ptr, gpa + mem::PAGE_SIZE as u64);
    }
    gpa as u32
}

/// Software-defined AVL/CoW bit in x86 PTEs (bit 9).
///
/// The Hyperlight host sets this bit on page table entries that point to snapshot GPAs and require
/// Copy-on-Write resolution. The guest must copy the page to a scratch frame and clear this bit.
const PAGE_AVL_COW: u32 = 1 << 9;

/// Mask for extracting the 4 KiB-aligned physical address from a PTE/PDE.
pub(crate) const PTE_ADDR_MASK_U32: u32 = 0xFFFFF000;

///
/// # Description
///
/// Eagerly pre-faults every CoW-marked page in the host-built page tables.
///
/// Walks the page directory (from CR3), and for each present PDE walks the page table.
/// For each PTE with the `PAGE_AVL_COW` bit set:
///   1. Allocates a fresh scratch frame via the bump allocator.
///   2. Copies 4 KiB from the snapshot-backed GVA to the new scratch frame.
///   3. Patches the PTE in place: scratch GPA, PAGE_PRESENT | PAGE_RW, clear PAGE_AVL_COW.
///
/// After this function returns, every writable page is backed by scratch memory and the
/// kernel runs fault-free.
///
/// # Safety
///
/// Must be called only after the bump allocator has been advanced past the
/// scratch-reserved region, so newly allocated scratch frames cannot overlap
/// memory reserved during early boot. It must also run before any writes to
/// snapshot-backed memory that depend on CoW resolution, so those writes do
/// not fault while CoW mappings are still in place.
///
/// The page tables themselves reside in scratch (already writable), so patching PTEs
/// does not trigger faults.
///
unsafe fn eager_prefault_cow_pages() {
    use ::arch::mem::paging::{
        PresentFlag,
        PteWord,
    };

    // Read CR3 to find the host page directory.
    let cr3: u32;
    core::arch::asm!("mov {:e}, cr3", out(reg) cr3, options(nostack, nomem));
    let pd_gpa: usize = (cr3 & PTE_ADDR_MASK_U32) as usize;
    let pd_base: *const u32 = gpa_to_gva(pd_gpa) as *const u32;

    let page_table_length: usize = mem::PAGE_TABLE_LENGTH;

    for pd_idx in 0..page_table_length {
        let pde: u32 = pd_base.add(pd_idx).read_volatile();
        if !PresentFlag::is_set(pde) {
            continue;
        }

        let pt_gpa: usize = (pde & PTE_ADDR_MASK_U32) as usize;
        let pt_base: *mut u32 = gpa_to_gva(pt_gpa) as *mut u32;

        for pt_idx in 0..page_table_length {
            let pte: u32 = pt_base.add(pt_idx).read_volatile();
            if !PresentFlag::is_set(pte) {
                continue;
            }
            // Check for the AVL/CoW software bit.
            if pte & PAGE_AVL_COW == 0 {
                continue;
            }

            // Allocate a scratch frame via the bump allocator.
            let new_frame_gpa: u32 = bump_alloc_frame();
            let new_frame_gva: usize = gpa_to_gva(new_frame_gpa as usize);

            // The source GVA is the identity-mapped virtual address of this page.
            let src_gva: usize = pd_idx * mem::PGTAB_SIZE + pt_idx * mem::PAGE_SIZE;

            // Copy 4 KiB from the snapshot-backed GVA to the new scratch frame.
            core::ptr::copy_nonoverlapping(
                src_gva as *const u8,
                new_frame_gva as *mut u8,
                mem::PAGE_SIZE,
            );

            // Build the new PTE: scratch GPA, present + RW + accessed, clear AVL_COW.
            // Preserve the user-mode flag if set.
            let mut new_pte: PteWord = (new_frame_gpa & PTE_ADDR_MASK_U32)
                | PresentFlag::Present as PteWord
                | ::arch::mem::paging::ReadWriteFlag::ReadWrite as PteWord
                | ::arch::mem::paging::AccessedFlag::Accessed as PteWord;
            // Preserve USER bit from the original PTE.
            new_pte |= pte & (::arch::mem::paging::UserSupervisorFlag::User as PteWord);

            pt_base.add(pt_idx).write_volatile(new_pte);
        }
    }

    // Flush TLB by reloading CR3.
    core::arch::asm!(
        "mov {tmp:e}, cr3",
        "mov cr3, {tmp:e}",
        tmp = out(reg) _,
        options(nostack),
    );
}

///
/// # Description
///
/// Hyperlight evolve-phase entry point, called from `_do_start` after the boot stack has been
/// switched to scratch memory.
///
/// Performs one-time initialization that subsequent `sandbox.call()` invocations depend on:
///
/// 1. Initializes the kernel heap (needed for FunctionCall deserialisation).
///
/// On return, the caller (`_do_start` in `start.rs`) halts the VM so that `evolve()` returns
/// on the host.
///
#[unsafe(no_mangle)]
extern "C" fn hyperlight_pre_kmain() {
    extern "C" {
        static __KERNEL_END: u8;
    }

    // Compute the PEB base address — needed to derive the scratch region for the heap backing
    // storage and to initialize the GuestHandle. The GVA→GPA patch was already applied by
    // init_scratch_kstack().
    let kernel_end: usize = unsafe { &__KERNEL_END as *const u8 as usize };
    let peb_base: usize = ::sys::mm::align_up(kernel_end, PAGE_ALIGNMENT)
        .expect("hyperlight_pre_kmain(): PEB align_up overflow");
    let peb_ptr: *mut HyperlightPEB = peb_base as *mut HyperlightPEB;

    // Capture the boot-time first-free-GPA before any bump allocations occur.
    // The live allocator pointer will be advanced below, so we must read it now.
    let boot_first_free_val: usize = first_free_scratch_gpa();

    // Derive the scratch-reserved base from the PEB and host page table metadata.
    // This accounts for the host page tables that are placed between the IO buffers
    // and our reserved structures.
    // NOTE: load_boot_first_free_gpa() returns 0 here (static not yet written),
    // so scratch_reserved_base() falls back to first_free_scratch_gpa() which
    // still holds the correct value (bump hasn't happened yet).
    let scratch_reserved_base: usize = scratch_reserved_base();

    // Advance the bump allocator pointer past our scratch-reserved region so that
    // frame allocations never overlap our data structures. This must be done on
    // every entry (evolve and call) because the host resets the bump pointer on
    // snapshot restore.
    {
        let reserved_end_gpa: usize = boot_first_free_val + SCRATCH_RESERVED_SIZE;
        let alloc_ptr = ::hyperlight_guest::layout::allocator_gva();
        let current: usize = unsafe { core::ptr::read_volatile(alloc_ptr) as usize };
        if current < reserved_end_gpa {
            unsafe { core::ptr::write_volatile(alloc_ptr, reserved_end_gpa as u64) };
        }
    }

    // Eagerly resolve all CoW pages: walk the host-built page tables and copy every
    // CoW-marked page to a scratch frame, patching the PTE in place. After this call,
    // every writable page in the guest address space is backed by scratch memory.
    unsafe {
        eager_prefault_cow_pages();
    }

    // Now that CoW is resolved, the kernel BSS is writable. Persist the boot-time
    // first-free-GPA so later code (e.g., platform::init) can determine the host
    // page table boundary.
    save_boot_first_free_gpa(boot_first_free_val);

    // Set a conservative snapshot_end so downstream code can determine the
    // snapshot boundary.  The exact value is refined later in platform::init()
    // once the host memory layout is known.  Using the scratch base (from PEB)
    // as the upper bound is safe.
    let peb_scratch_base: usize = unsafe { (*peb_ptr).input_stack.ptr } as usize;
    store_snapshot_end(peb_scratch_base);

    let klog_backing_ptr: *mut u8 = unsafe { klog_buffer_ptr(scratch_reserved_base) };
    if let Err(_e) = unsafe { crate::klog::set_backing_storage(klog_backing_ptr) } {
        unsafe {
            core::arch::asm!("cli", "2: hlt", "jmp 2b", options(noreturn));
        }
    }

    let heap_backing_ptr: *mut u8 = (scratch_reserved_base + HEAP_STORAGE_OFFSET) as *mut u8;
    if let Err(_e) =
        unsafe { crate::mm::kheap::set_backing_storage(heap_backing_ptr, HEAP_STORAGE_SIZE) }
    {
        unsafe {
            core::arch::asm!("cli", "2: hlt", "jmp 2b", options(noreturn));
        }
    }

    if let Err(_e) = unsafe { crate::mm::kheap::init() } {
        unsafe {
            core::arch::asm!("cli", "2: hlt", "jmp 2b", options(noreturn));
        }
    }
}

///
/// # Description
///
/// High-level guest function dispatcher called by `_nanvix_dispatch`.
///
/// Reads the [`FunctionCall`] that the host serialised into the PEB input
/// buffer, validates the function name, and dispatches to the corresponding
/// handler.  Currently the only recognised function is `"kmain"`.
///
/// For unrecognised functions a [`GuestError`] is written back to the PEB
/// output buffer so the host receives a proper error instead of a raw abort.
///
/// # Parameters
///
/// - `kargs`: Kernel arguments reconstructed by the `_nanvix_dispatch` stub.
///
#[unsafe(no_mangle)]
extern "C" fn nanvix_dispatch_function(kargs: &crate::kargs::KernelArguments) {
    let handle: GuestHandle = guest_handle();

    let function_call = handle
        .try_pop_shared_input_data_into::<FunctionCall>()
        .expect("function call deserialization failed");

    match function_call.function_name.as_str() {
        "kmain" => {
            drop(function_call);
            crate::kmain(kargs);
        },
        other => {
            let msg = format!("unknown guest function: {other}");
            drop(function_call);
            let guest_error = GuestError::new(GuestErrorCode::GuestError, msg);
            let fcr = FunctionCallResult::new(Err(guest_error));
            let mut builder = ::flatbuffers::FlatBufferBuilder::new();
            let data = fcr.encode(&mut builder);
            handle
                .push_shared_output_data(data)
                .expect("failed to serialize function call error result");
        },
    }
}

///
/// # Description
///
/// Returns the TSC base frequency in MHz. On hyperlight the VMM does not
/// provide this value, so `0` is returned (callers should use a fallback
/// calibration path).
///
#[allow(dead_code)]
pub fn tsc_base_frequency_mhz() -> u32 {
    0
}

///
/// # Description
///
/// Checks whether the given virtual address corresponds to a valid physical address on the
/// Hyperlight platform.
///
/// # Parameters
///
/// - `addr`: The virtual address to validate.
///
/// # Returns
///
/// `true` if `addr` falls within a known physical memory region, `false` otherwise.
///
#[inline(always)]
pub fn is_valid_physical_address(addr: VirtualAddress) -> bool {
    let raw: usize = addr.into_raw_value();
    let snapshot: (usize, usize) = (KERNEL_BASE_RAW, load_snapshot_end());
    let ramfs: (usize, usize) = (load_ramfs_base(), load_ramfs_end());
    let scratch: (usize, usize) = (load_scratch_base(), load_scratch_end());
    // The frame allocator hands out scratch GPAs, which differ from scratch GVAs.
    // Accept both ranges so physical addresses from the frame allocator pass validation.
    // Only check scratch_gpa after init() has set up the scratch bounds (scratch.0 != 0),
    // because the frame allocator is not operational before that point.
    let scratch_gpa: (usize, usize) = {
        if scratch.0 != 0 {
            let sz: usize = cached_scratch_size();
            if sz > 0 {
                (MAX_GPA + 1 - sz, MAX_GPA + 1)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        }
    };
    // An address is valid when the half-open interval [raw, raw+1) lies inside a region.
    region_contains(snapshot.0, snapshot.1, raw, raw + 1)
        || region_contains(ramfs.0, ramfs.1, raw, raw + 1)
        || region_contains(scratch.0, scratch.1, raw, raw + 1)
        || region_contains(scratch_gpa.0, scratch_gpa.1, raw, raw + 1)
}

///
/// # Description
///
/// Checks whether the given physical memory region lies entirely within a single contiguous
/// physical memory region on the Hyperlight platform.
///
/// # Parameters
///
/// - `start`: Starting physical address of the region.
/// - `size`: Size of the region in bytes.
///
/// # Returns
///
/// `true` if the entire region lies within a single physical memory region, `false` otherwise.
///
#[inline(always)]
pub fn is_valid_physical_region(start: usize, size: usize) -> bool {
    // Reject zero-length regions.
    if size == 0 {
        return false;
    }

    // Compute the exclusive end, guarding against overflow.
    let end: usize = match start.checked_add(size) {
        Some(end) => end,
        None => return false,
    };

    let snapshot: (usize, usize) = (KERNEL_BASE_RAW, load_snapshot_end());
    let ramfs: (usize, usize) = (load_ramfs_base(), load_ramfs_end());
    let scratch: (usize, usize) = (load_scratch_base(), load_scratch_end());
    // Only check scratch_gpa after init() has set up the scratch bounds (scratch.0 != 0),
    // because the frame allocator is not operational before that point.
    let scratch_gpa: (usize, usize) = {
        if scratch.0 != 0 {
            let sz: usize = cached_scratch_size();
            if sz > 0 {
                (MAX_GPA + 1 - sz, MAX_GPA + 1)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        }
    };

    region_contains(snapshot.0, snapshot.1, start, end)
        || region_contains(ramfs.0, ramfs.1, start, end)
        || region_contains(scratch.0, scratch.1, start, end)
        || region_contains(scratch_gpa.0, scratch_gpa.1, start, end)
}

///
/// # Description
///
/// Checks whether the half-open interval `[start, end)` is entirely contained within the region
/// `[region_base, region_end)`. Absent regions (base == end == 0) never contain any interval.
///
/// All arithmetic uses wrapping subtraction so that regions and intervals whose exclusive end
/// overflows past `usize::MAX` (i.e., wraps to 0) are handled correctly. This is relevant on
/// 32-bit targets where the scratch region may abut the 4 GiB ceiling.
///
/// # Parameters
///
/// - `region_base`: Inclusive start address of the region.
/// - `region_end`: Exclusive end address of the region.
/// - `start`: Inclusive start address of the interval to test.
/// - `end`: Exclusive end address of the interval to test.
///
/// # Returns
///
/// `true` if the interval lies within the region, `false` otherwise.
///
#[inline(always)]
fn region_contains(region_base: usize, region_end: usize, start: usize, end: usize) -> bool {
    // Empty region.
    if region_base == region_end {
        return false;
    }
    // Normalize addresses relative to region_base using wrapping subtraction.
    let region_size: usize = region_end.wrapping_sub(region_base);
    let rel_start: usize = start.wrapping_sub(region_base);
    let rel_end: usize = end.wrapping_sub(region_base);
    // The interval is contained when both endpoints fall within [0, region_size] and the
    // interval itself does not wrap around (rel_start <= rel_end).
    rel_start < region_size && rel_end <= region_size && rel_start <= rel_end
}

///
/// # Description
///
/// Returns the highest valid physical address on the Hyperlight platform.
///
/// The physical memory is sparse and spans from low addresses (snapshot) to near the top of
/// the 32-bit address space (scratch). The highest valid physical address is the last address
/// before `SCRATCH_END`, since addresses greater than or equal to `SCRATCH_END` are not
/// considered valid by the platform.
///
/// Before `init()` discovers the scratch region, only the snapshot region is known, so the
/// highest valid address is `SNAPSHOT_END - 1`.
///
/// # Returns
///
/// The highest valid physical address value.
///
#[inline(always)]
pub fn max_physical_address() -> usize {
    let scratch_base: usize = load_scratch_base();
    if scratch_base != 0 {
        // Post-init: the highest valid address is SCRATCH_END - 1.
        // Use wrapping_sub because SCRATCH_END may be 0 when the scratch region
        // reaches the top of the 32-bit address space (base + size wraps).
        load_scratch_end().wrapping_sub(1)
    } else {
        // Pre-init: only the snapshot region is known.
        load_snapshot_end().saturating_sub(1)
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
    trace!("{magic:?}, {info:?}");

    extern "C" {
        static __KERNEL_END: u8;
    }

    let kernel_end: usize = unsafe { &__KERNEL_END as *const u8 as usize };
    let peb_base: usize = ::sys::mm::align_up(kernel_end, PAGE_ALIGNMENT).ok_or_else(|| {
        let reason: &str = "align_up overflow";
        error!("parse_bootinfo(): {reason} (kernel_end={kernel_end:#x})");
        Error::new(ErrorCode::BadAddress, reason)
    })?;
    let peb_ptr: *mut HyperlightPEB = peb_base as *mut HyperlightPEB;

    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();

    // Read the init_data blob from the PEB.
    let (current_data_start, total_size) = unsafe {
        let start: usize = (*peb_ptr).init_data.ptr as usize;
        let size: usize = (*peb_ptr).init_data.size as usize;
        (start, size)
    };

    if total_size == 0 {
        info!("parse_bootinfo(): no init_data provided, booting without kernel modules");
        return Ok(BootInfo::new(
            None,
            None,
            LinkedList::new(),
            LinkedList::new(),
            IoMemoryAllocator::new(),
            kernel_modules,
        ));
    }

    // Detect initrd format by checking for multibinary magic.
    let init_data: &[u8] =
        unsafe { core::slice::from_raw_parts(current_data_start as *const u8, total_size) };

    if init_data.len() >= multibin::MAGIC.len()
        && init_data[..multibin::MAGIC.len()] == multibin::MAGIC
    {
        // Multibinary NVMB format: relocate whole blob, then parse entries.
        info!("parse_bootinfo(): multibinary initrd detected");

        let initrd_base: usize = current_data_start;

        kernel_modules.extend(crate::multibin::parse(
            unsafe { core::slice::from_raw_parts(initrd_base as *const u8, total_size) },
            initrd_base,
        )?);
    } else {
        // Single-binary format: parse old size-header + args layout.
        info!("parse_bootinfo(): single-binary initrd detected");

        let (initrd_base, initrd_size, (_cmdline_len, cmdline)) = unsafe {
            let (base, size, cmdline) = parse_initrd_image(current_data_start, total_size)?;
            (base, size, cmdline)
        };

        info!(
            "initrd_base={:#010x}, initrd_size={:#010x}, cmdline={:?}",
            initrd_base,
            initrd_size,
            cmdline.as_str()
        );

        let module: KernelModule =
            KernelModule::new(PhysicalAddress::from_raw_value(initrd_base)?, initrd_size, cmdline);
        kernel_modules.push_back(module);
    }

    Ok(BootInfo::new(
        None,
        None,
        LinkedList::new(),
        LinkedList::new(),
        IoMemoryAllocator::new(),
        kernel_modules,
    ))
}

fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error> {
    // Register I/O ports for 8259 PIC.
    ioports.register_read_write(::arch::cpu::pic::PIC_CTRL_MASTER as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_DATA_MASTER as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_CTRL_SLAVE as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_DATA_SLAVE as u16)?;
    Ok(())
}

#[cfg(feature = "pit")]
fn register_pit(ioports: &mut IoPortAllocator) -> Result<Pit, Error> {
    // Register ports for the PIT.

    ioports.register_read_write(::arch::cpu::pit::PIT_CTRL)?;
    ioports.register_read_write(::arch::cpu::pit::PIT_DATA)?;

    // Configure the PV timer: writing the period (in microseconds) to the
    // PvTimerConfig port tells the hypervisor to inject periodic timer interrupts at that rate.
    // Use a direct I/O port write instead of the allocator-managed port to avoid registering
    // the PV timer port for general-purpose allocation.
    //
    // Safety: PV_TIMER_PORT is the Hyperlight PvTimerConfig port, and writing the period
    // value is the expected interface to configure the paravirtual timer.
    unsafe {
        ::arch::io::out32(
            ::config::hyperlight::PV_TIMER_PORT,
            ::config::hyperlight::TIMER_PERIOD_US,
        );
    }

    Pit::new(ioports, ::config::kernel::TIMER_FREQ)
}

///
/// # Description
///
/// Initializes the hyperlight platform.
///
/// # Parameters
///
/// - `ioports`: I/O port allocator.
/// - `ioaddresses`: I/O memory allocator.
/// - `memory_regions`: Memory regions.
/// - `mmio_regions`: MMIO regions.
/// - `madt`: MADT information.
/// - `_mem_lower`: Lower memory size.
///
/// # Returns
///
/// Upon success, the initialized platform is returned. Upon failure, an error is returned instead.
///
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error> {
    register_pic_ioports(ioports)?;

    extern "C" {
        static __KERNEL_START: u8;
        static __KERNEL_END: u8;
    }

    // Register the gap between physical address 0 and __KERNEL_START as reserved so the
    // frame allocator never hands out frames below the kernel text section.
    // On Hyperlight, KERNEL_BASE_RAW is 0 and __KERNEL_START is at BOOT_ADDR
    // (0x100000 + PLATFORM_BASE_ADDR), leaving a gap that includes the trampoline
    // and zero-padding sections from the linker script.
    {
        let kernel_start_addr: usize = core::ptr::addr_of!(__KERNEL_START) as usize;
        if let Some(gap_size) = kernel_start_addr.checked_sub(KERNEL_BASE_RAW) {
            if gap_size > 0 {
                let pre_kernel_gap: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                    "pre-kernel gap",
                    VirtualAddress::from_raw_value(KERNEL_BASE_RAW),
                    gap_size,
                    MemoryRegionType::Reserved,
                    AccessPermission::RDONLY,
                )?;
                memory_regions.push_back(pre_kernel_gap);
                info!("pre-kernel gap: [{:#010x}, {:#010x})", KERNEL_BASE_RAW, kernel_start_addr);
            }
        }
    }

    // Query the host for the authoritative physical memory layout.
    let (snapshot_budget_size, pt_overhead, ramfs_base, ramfs_size, scratch_size) =
        unsafe { ProcessEnvironmentBlock::get_memory_layout() }?;
    info!(
        "host memory layout: snapshot_budget_size={} KB, pt_overhead={} KB, ramfs_base={:#x}, \
         ramfs_size={} KB, scratch_size={} KB",
        snapshot_budget_size / KILOBYTE,
        pt_overhead / KILOBYTE,
        ramfs_base,
        ramfs_size / KILOBYTE,
        scratch_size / KILOBYTE
    );

    // Sanity check: the three regions must cover exactly MEMORY_SIZE.
    check_memory_size(snapshot_budget_size, ramfs_size, scratch_size)?;

    // Register PEB structure.
    let kernel_end_addr: usize = unsafe { &__KERNEL_END } as *const u8 as usize;
    let peb_base: usize =
        ::sys::mm::align_up(kernel_end_addr, PAGE_ALIGNMENT).ok_or_else(|| {
            let reason: &str = "align_up overflow";
            error!("init(): {reason} (kernel_end_addr={kernel_end_addr:#x})");
            Error::new(ErrorCode::BadAddress, reason)
        })?;
    let peb: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
        "peb",
        PageAligned::from_raw_value(peb_base)?,
        PEB_SIZE,
        AccessPermission::RDWR,
        MmioCachePolicy::UNCACHEABLE,
    )?;
    ioaddresses.register(PEB_MMIO_TAG, peb.clone())?;
    mmio_regions.push_back(peb);

    // Compute padding between PEB and kernel pool.
    let heap_padding_base: usize = peb_base.checked_add(PEB_SIZE).ok_or_else(|| {
        let reason: &str = "heap padding base address overflow";
        error!("init(): {}", reason);
        Error::new(ErrorCode::OutOfMemory, reason)
    })?;
    debug!("heap_padding_base={:#010x}", heap_padding_base);
    let kpool_base: usize = memory_layout::KPOOL_BASE.into_raw_value();
    match kpool_base.checked_sub(heap_padding_base) {
        None => {
            let reason: &str = "kernel image exceeds KPOOL_BASE, memory regions overlap";
            error!(
                "init(): {} (heap_padding_base={:#010x}, kpool_base={:#010x})",
                reason, heap_padding_base, kpool_base
            );
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        },
        Some(0) => {
            debug!("init(): no heap padding needed");
        },
        Some(heap_padding_size) => {
            let heap_padding: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                "heap padding",
                VirtualAddress::from_raw_value(heap_padding_base),
                heap_padding_size,
                MemoryRegionType::Reserved,
                AccessPermission::RDONLY,
            )?;
            memory_regions.push_back(heap_padding);
        },
    }

    // Register RAMFS region as MMIO for identity mapping, if present.
    if ramfs_base != 0 && ramfs_size != 0 {
        let mut ramfs_mr: MemoryRegion<VirtualAddress> = MemoryRegion::new(
            RAMFS_REGION_NAME,
            VirtualAddress::from_raw_value(ramfs_base),
            ramfs_size,
            MemoryRegionType::Mmio,
            AccessPermission::RDWR,
        )?;
        ramfs_mr.set_cache_policy(MmioCachePolicy::WRITE_BACK);
        let ramfs_region: TruncatedMemoryRegion<VirtualAddress> =
            TruncatedMemoryRegion::from_memory_region(ramfs_mr)?;
        ioaddresses.register(RAMFS_MMIO_TAG, ramfs_region.clone())?;
        mmio_regions.push_back(ramfs_region);
    }

    // Record RAMFS region bounds for is_valid_physical_address.
    if ramfs_size > 0 {
        let ramfs_end: usize = match ramfs_base.checked_add(ramfs_size) {
            Some(ramfs_end) => ramfs_end,
            None => {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "host-reported RAMFS bounds overflow",
                ));
            },
        };

        // Check that the RAMFS identity-mapped region does not overlap the user mmap region.
        if ramfs_end > memory_layout::USER_MMAP_BASE_RAW {
            let reason: &str = "RAMFS region overlaps user mmap region";
            error!(
                "init(): {} (ramfs_end={:#010x}, mmap_base={:#010x})",
                reason,
                ramfs_end,
                memory_layout::USER_MMAP_BASE_RAW
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        store_ramfs_base(ramfs_base);
        store_ramfs_end(ramfs_end);
        info!("ramfs region: [{:#010x}, {:#010x})", ramfs_base, ramfs_end);
    }

    // Derive scratch region addresses from the host-provided scratch_size.
    // scratch_size covers the full range [scratch_base, scratch_end) and includes the last
    // page that Hyperlight reserves for bookkeeping.  That page is included in the bitmap
    // but marked as reserved (used) so the frame allocator never hands it out.
    // Validate that scratch_size is non-zero and large enough to contain the required scratch
    // regions (input buffer + output buffer + allocator storage pages + boot stack + one I/O
    // page + one reserved last page).
    let min_scratch_size: usize = INPUT_DATA_BUFFER_SIZE
        + OUTPUT_DATA_BUFFER_SIZE
        + SCRATCH_RESERVED_SIZE
        + ::config::kernel::KSTACK_SIZE
        + 2 * mem::PAGE_SIZE;
    if scratch_size == 0 || scratch_size < min_scratch_size {
        let reason: &str = "scratch_size is too small for required scratch regions";
        error!(
            "init(): {} (scratch_size={:#x}, min={:#x})",
            reason, scratch_size, min_scratch_size
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    // The host maps the scratch region at GVA = MAX_GVA - scratch_size + 1.
    // Use MAX_GVA (not MAX_GPA) because the CPU accesses memory through the
    // host-built page tables, which map scratch at GVA addresses.
    let scratch_base_address: usize = MAX_GVA - scratch_size + 1;

    // Sanity check: the scratch region must not descend into the user virtual address space.
    // The config crate already computes USER_END_RAW to avoid this overlap, but verify at
    // runtime in case the memory layout parameters diverge from compile-time assumptions.
    // When they do, the reserved MMIO structures (input/output buffers) at the bottom of the
    // scratch region would collide with user stack pages.
    if scratch_base_address < memory_layout::USER_END_RAW {
        let reason: &str = "scratch region overlaps user address space";
        error!(
            "init(): {} (scratch_base={:#010x} < USER_END_RAW={:#010x}, scratch_size={:#x})",
            reason,
            scratch_base_address,
            memory_layout::USER_END_RAW,
            scratch_size
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // scratch_end is the exclusive end of the full scratch range, including the last page
    // reserved for Hyperlight bookkeeping metadata.  When MAX_GVA == usize::MAX
    // (i686-guest), the addition wraps to 0 — this is expected and handled
    // correctly by region_contains() and max_physical_address().
    let scratch_end_address: usize = scratch_base_address.wrapping_add(scratch_size);

    // Record scratch region bounds for is_valid_physical_address.
    store_scratch_base(scratch_base_address);
    store_scratch_end(scratch_end_address);
    // Cache the scratch size for hot-path validators so they avoid volatile reads.
    SCRATCH_SIZE_CACHED.store(scratch_size, Ordering::Relaxed);
    info!(
        "scratch region: [{:#010x}, {:#010x}) (size={:#x})",
        scratch_base_address, scratch_end_address, scratch_size
    );

    // Register only the MMIO-critical portions of the scratch region:
    //  1. Input data buffer  [scratch_base, scratch_base + INPUT_DATA_BUFFER_SIZE)
    //  2. Output data buffer [scratch_base + INPUT_DATA_BUFFER_SIZE, + OUTPUT_DATA_BUFFER_SIZE)
    //  3. Scratch-reserved structures [output_end, output_end + SCRATCH_RESERVED_SIZE)
    //  4. Scratch I/O page: guest counter page (second-to-last page).
    //  5. Scratch reserved page: Hyperlight bookkeeping metadata (last page, not for use).
    // Everything between the allocator storage and the scratch I/O page is
    // intentionally left unregistered and is available for user-page allocation.
    {
        let scratch_input_base: usize = scratch_base_address;
        let scratch_input: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
            "scratch input",
            PageAligned::from_raw_value(scratch_input_base)?,
            INPUT_DATA_BUFFER_SIZE,
            AccessPermission::RDWR,
            MmioCachePolicy::UNCACHEABLE,
        )?;
        ioaddresses.register(INPUT_BUF_MMIO_TAG, scratch_input.clone())?;
        mmio_regions.push_back(scratch_input);
    }
    {
        let scratch_output_base: usize = scratch_base_address + INPUT_DATA_BUFFER_SIZE;
        let scratch_output: TruncatedMemoryRegion<VirtualAddress> =
            TruncatedMemoryRegion::new_mmio(
                "scratch output",
                PageAligned::from_raw_value(scratch_output_base)?,
                OUTPUT_DATA_BUFFER_SIZE,
                AccessPermission::RDWR,
                MmioCachePolicy::UNCACHEABLE,
            )?;
        ioaddresses.register(OUTPUT_BUF_MMIO_TAG, scratch_output.clone())?;
        mmio_regions.push_back(scratch_output);
    }

    // Reserve pages in the free scratch area for the frame allocator bitmap,
    // kpool bitmap, GDT backing stores, heap, etc.  This memory is in scratch (never
    // snapshot/CoW) and is registered as a reserved memory region so the frame allocator
    // never hands it out.
    //
    // The host places its page tables at the start of the free scratch area (right after
    // the IO buffers), so our reserved structures must be placed AFTER the host page
    // tables to avoid overlap.
    let io_buffers_end: usize =
        scratch_base_address + INPUT_DATA_BUFFER_SIZE + OUTPUT_DATA_BUFFER_SIZE;
    let boot_first_free: usize = load_boot_first_free_gpa();
    let host_pt_gva_end: usize = gpa_to_gva(if boot_first_free != 0 {
        boot_first_free
    } else {
        first_free_scratch_gpa()
    });
    // Reuse the helper that computes the scratch-reserved base from PEB + host PT metadata.
    // This avoids duplicating the max(io_buffers_end, host_pt_end) + page-align logic.
    let scratch_reserved_base: usize = scratch_reserved_base();
    {
        // No need to zero the storage pages: the scratch region is guaranteed to be
        // zeroed out by Hyperlight before the guest starts.

        let scratch_reserved_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
            "scratch reserved structures",
            VirtualAddress::from_raw_value(scratch_reserved_base),
            SCRATCH_RESERVED_SIZE,
            MemoryRegionType::Reserved,
            AccessPermission::RDWR,
        )?;
        memory_regions.push_back(scratch_reserved_region);
        info!(
            "scratch reserved: [{:#010x}, {:#010x}) (frame_alloc={} B, kpool={} B, gdt={} B, \
             heap_storage={} B, size={:#x})",
            scratch_reserved_base,
            scratch_reserved_base + SCRATCH_RESERVED_SIZE,
            FRAME_ALLOC_BITMAP_SIZE,
            KPOOL_BITMAP_SIZE,
            GDT_SIZE,
            HEAP_STORAGE_SIZE,
            SCRATCH_RESERVED_SIZE,
        );
    }

    // Validate that our scratch-reserved region does not overlap the host-built page tables.
    // The host places page tables in scratch at a GPA readable from the scratch metadata
    // slot at SCRATCH_TOP_SNAPSHOT_PT_GPA_BASE_OFFSET. The page table size is derived as
    // first_free_gpa − pt_base_gpa.
    {
        let pt_gpa_base: usize = pt_base_gpa();
        // Use the cached boot-time first-free-GPA (before any CoW bump allocations
        // advanced the pointer) to determine the actual PT end boundary.
        let boot_first_free: usize = load_boot_first_free_gpa();
        let pt_gpa_end: usize = if boot_first_free != 0 {
            boot_first_free
        } else {
            first_free_scratch_gpa()
        };
        // Convert PT GPA range to GVA for comparison with our scratch_reserved_base (a GVA).
        let pt_gva_base: usize = gpa_to_gva(pt_gpa_base);
        let pt_gva_end: usize = gpa_to_gva(pt_gpa_end);
        let reserved_end: usize = scratch_reserved_base + SCRATCH_RESERVED_SIZE;

        info!(
            "host page tables: GPA [{:#010x}, {:#010x}) => GVA [{:#010x}, {:#010x}), size={:#x}",
            pt_gpa_base,
            pt_gpa_end,
            pt_gva_base,
            pt_gva_end,
            pt_gpa_end - pt_gpa_base
        );

        // Check for overlap: two ranges [a, b) and [c, d) overlap iff a < d && c < b.
        if scratch_reserved_base < pt_gva_end && pt_gva_base < reserved_end {
            let reason: &str = "scratch reserved region overlaps host page tables";
            error!(
                "init(): {} (reserved=[{:#010x}, {:#010x}), pt=[{:#010x}, {:#010x}))",
                reason, scratch_reserved_base, reserved_end, pt_gva_base, pt_gva_end,
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Reserve the boot stack area at the top of the scratch region, just below
    // the I/O and bookkeeping pages.  The address is a compile-time constant
    // derived from HYPERLIGHT_GPA_CEILING, which falls within the GVA scratch range.
    {
        let boot_stack_guard: usize =
            memory_layout::HYPERLIGHT_BOOT_STACK_TOP - ::config::kernel::KSTACK_SIZE;
        let boot_stack_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
            "boot stack",
            VirtualAddress::from_raw_value(boot_stack_guard),
            ::config::kernel::KSTACK_SIZE,
            MemoryRegionType::Reserved,
            AccessPermission::RDWR,
        )?;
        memory_regions.push_back(boot_stack_region);
    }
    {
        // scratch_end_address wraps to 0 when MAX_GVA == usize::MAX, so use wrapping_sub.
        let scratch_io_page: usize = scratch_end_address.wrapping_sub(2 * mem::PAGE_SIZE);
        let scratch_io: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
            "scratch-io",
            PageAligned::from_raw_value(scratch_io_page)?,
            mem::PAGE_SIZE,
            AccessPermission::RDWR,
            MmioCachePolicy::UNCACHEABLE,
        )?;
        ioaddresses.register(SCRATCH_IO_MMIO_TAG, scratch_io.clone())?;
        mmio_regions.push_back(scratch_io);
    }
    {
        let scratch_reserved_page: usize = scratch_end_address.wrapping_sub(mem::PAGE_SIZE);
        let scratch_reserved: MemoryRegion<VirtualAddress> = MemoryRegion::new(
            "scratch reserved",
            VirtualAddress::from_raw_value(scratch_reserved_page),
            mem::PAGE_SIZE,
            MemoryRegionType::Reserved,
            AccessPermission::RDONLY,
        )?;
        memory_regions.push_back(scratch_reserved);
    }

    // Set snapshot region end from the host-provided snapshot budget.
    // With i686-guest, the host builds guest page tables which adds pt_overhead bytes on
    // top of the snapshot budget. Include pt_overhead so the snapshot region covers all
    // host-placed data (kernel code, PEB, heap, init data, and page tables).
    // The snapshot_end for CoW purposes covers the full budget (all pages may be CoW-protected).
    // But the frame allocator bitmap only needs to cover actually-usable frames.
    // On Hyperlight, pages beyond the last used section (kernel + initrd) may not have
    // host-created PTEs, so they must not be handed out by the frame allocator.
    // Keep the budget end distinct from the actual snapshot end so users that only care
    // about budget-backed guest frames can do so explicitly.
    let snapshot_budget_end_address: usize = SNAPSHOT_BASE + snapshot_budget_size;
    let snapshot_end_for_cow: usize = snapshot_budget_end_address + pt_overhead;
    store_snapshot_end(snapshot_end_for_cow);
    // Use the full host-populated snapshot range as the snapshot end. The budget portion
    // corresponds to guest-usable frames, while pt_overhead accounts for host-appended
    // metadata such as page tables that must still be covered by region tracking.
    let snapshot_end_address: usize = snapshot_end_for_cow;
    info!("snapshot region: [{:#010x}, {:#010x})", SNAPSHOT_BASE, snapshot_end_address);

    // Register gap-filling reserved regions within the snapshot to ensure every snapshot
    // frame is booked without overlapping with individually-registered sub-regions
    // (kernel text/data/rodata/bss, PEB, heap padding, kpool, initrd modules from kmain).
    //
    // The two gaps are:
    //   1. The initrd header page — multibinary descriptors before the first module payload.
    //   2. The snapshot tail — host budget padding after the last module.
    //
    // When no init_data is present, the entire range from kpool_end to snapshot_end is a gap.
    {
        let kpool_end: usize = kpool_base + ::config::kernel::KPOOL_SIZE;
        let peb: *const HyperlightPEB = peb_base as *const HyperlightPEB;
        let init_data_start: usize = unsafe { (*peb).init_data.ptr } as usize;
        let init_data_size: usize = unsafe { (*peb).init_data.size } as usize;

        if init_data_size > 0 {
            // Gap between kpool end and init_data start (if any alignment padding exists).
            if init_data_start > kpool_end {
                let gap_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                    "kpool-initrd gap",
                    VirtualAddress::from_raw_value(kpool_end),
                    init_data_start - kpool_end,
                    MemoryRegionType::Reserved,
                    AccessPermission::RDONLY,
                )?;
                memory_regions.push_back(gap_region);
            }

            // For multibinary (NVMB) format, the first page of the init_data blob contains
            // the header and entry descriptors. Module payloads start at page-aligned offsets
            // after this header, leaving the header page as a gap not covered by any module
            // region. For single-binary format, the module payload starts at byte 8 within
            // the first page (after the size header), and kmain page-aligns downward so the
            // first page IS part of the module — no gap to register.
            let init_data_slice: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    init_data_start as *const u8,
                    init_data_size.min(multibin::MAGIC.len()),
                )
            };
            let is_multibinary: bool = init_data_slice.len() >= multibin::MAGIC.len()
                && init_data_slice[..multibin::MAGIC.len()] == multibin::MAGIC;
            if is_multibinary {
                let initrd_header_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                    "initrd header",
                    VirtualAddress::from_raw_value(init_data_start),
                    mem::PAGE_SIZE,
                    MemoryRegionType::Reserved,
                    AccessPermission::RDONLY,
                )?;
                memory_regions.push_back(initrd_header_region);
            }

            // Snapshot tail: padding between the end of init_data and snapshot_end.
            // The host allocates a full snapshot budget that may extend beyond the last module.
            let init_data_end: usize = init_data_start + init_data_size;
            let tail_start: usize =
                ::sys::mm::align_up(init_data_end, PAGE_ALIGNMENT).unwrap_or(init_data_end);
            if snapshot_end_address > tail_start {
                let tail_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                    "snapshot tail",
                    VirtualAddress::from_raw_value(tail_start),
                    snapshot_end_address - tail_start,
                    MemoryRegionType::Reserved,
                    AccessPermission::RDONLY,
                )?;
                memory_regions.push_back(tail_region);
            }
        } else if snapshot_end_address > kpool_end {
            // No init_data — the entire range from kpool_end to snapshot_end is unused.
            let gap_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                "snapshot tail",
                VirtualAddress::from_raw_value(kpool_end),
                snapshot_end_address - kpool_end,
                MemoryRegionType::Reserved,
                AccessPermission::RDONLY,
            )?;
            memory_regions.push_back(gap_region);
        }
    }

    // Register the host page tables area. This is the scratch region between the IO buffers
    // and the scratch-reserved structures that holds the host-built page directory and page
    // tables. No individually-named region covers it.
    if host_pt_gva_end > io_buffers_end {
        let host_pt_size: usize = host_pt_gva_end - io_buffers_end;
        let host_pt_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
            "host page tables",
            VirtualAddress::from_raw_value(io_buffers_end),
            host_pt_size,
            MemoryRegionType::Reserved,
            AccessPermission::RDONLY,
        )?;
        memory_regions.push_back(host_pt_region);
    }

    // Register pre-faulted CoW pages. These are bump-allocated scratch frames consumed by
    // `eager_prefault_cow_pages()`. They lie between the scratch-reserved structures and
    // the boot stack area. The bump pointer (first_free_scratch_gpa) marks their end.
    {
        let prefault_start_gva: usize = scratch_reserved_base + SCRATCH_RESERVED_SIZE;
        let prefault_end_gva: usize = gpa_to_gva(first_free_scratch_gpa());
        if prefault_end_gva > prefault_start_gva {
            let prefault_size: usize = prefault_end_gva - prefault_start_gva;
            let prefault_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
                "prefault pages",
                VirtualAddress::from_raw_value(prefault_start_gva),
                prefault_size,
                MemoryRegionType::Reserved,
                AccessPermission::RDWR,
            )?;
            memory_regions.push_back(prefault_region);
        }
    }

    // Build a sparse bitmap representing the physical memory layout.
    // Each disjoint physical region (snapshot, RAMFS, scratch) gets its own chunk in the
    // SparseBitmap.  However, bitmap storage is byte-aligned so the snapshot chunk's last
    // byte may cover phantom frames beyond snapshot_end.  If the RAMFS starts within that
    // padded range the two chunks would overlap, so they are merged into a single "low"
    // chunk.  When the RAMFS is absent or far enough away it becomes a separate chunk.
    //
    // The backing store for the bitmap lives in the scratch region at `scratch_reserved_base`,
    // not in BSS, so it is never part of the CoW snapshot.
    let ramfs_end_address: usize = ramfs_base + ramfs_size;
    // Safety: frame_alloc_bitmap_ptr returns a pointer within the scratch-reserved region
    // that was identity-mapped and zeroed by Hyperlight before the guest started.
    // The memory outlives the returned `SparseBitmap` (it is never freed) and no other
    // code writes to this range after this point.
    let physical_memory_layout: SparseBitmap = unsafe {
        // Use GPAs for all regions. Snapshot and RAMFS are identity-mapped (GVA == GPA).
        // Scratch GVA ≠ GPA; convert back to GPA for the frame allocator bitmap.
        let scratch_base_gpa: usize = MAX_GPA + 1 - scratch_size;
        let scratch_end_gpa: usize = MAX_GPA + 1;
        build_physical_memory_layout(
            KERNEL_BASE_RAW,
            snapshot_end_address,
            ramfs_base,
            ramfs_end_address,
            scratch_base_gpa,
            scratch_end_gpa,
            (frame_alloc_bitmap_ptr(scratch_reserved_base), FRAME_ALLOC_BITMAP_SIZE),
        )?
    };

    // Build a bitmap for the kernel page pool.
    // The kpool bitmap storage is placed at a word-aligned offset after the frame allocator
    // storage in the same scratch reservation.
    let kpool_bitmap: Bitmap = {
        // Safety: kpool_bitmap_ptr returns a word-aligned pointer within the
        // scratch-reserved storage region and outlives the returned bitmap.
        let storage: RawArray<u8> = unsafe {
            let ptr: *mut u8 = kpool_bitmap_ptr(scratch_reserved_base);
            debug_assert!(::sys::mm::is_aligned(ptr as usize, WORD_ALIGNMENT));
            RawArray::from_raw_parts(ptr, KPOOL_BITMAP_SIZE)?
        };
        Bitmap::from_raw_array(storage)?
    };

    // Install GDT backing storage in the scratch region, outside the CoW snapshot.
    // The GDT is placed at GDT_OFFSET within the scratch-reserved area.
    //
    // Safety: gdt_ptr returns a pointer aligned to GDTE_ALIGNMENT (enforced at compile
    // time by the scratch_layout! macro). The scratch region is identity-mapped, zeroed
    // by Hyperlight, and never freed, so the pointer is valid for GDT_NUM_ENTRIES entries
    // and outlives all GDT usage. This is the only call to set_backing_storage() in the
    // Hyperlight init path.
    unsafe {
        let gdt_backing: *mut Gdte = gdt_ptr(scratch_reserved_base) as *mut Gdte;
        core::ptr::copy_nonoverlapping(
            gdt::DEFAULT_ENTRIES.as_ptr(),
            gdt_backing,
            gdt::GDT_NUM_ENTRIES,
        );
        gdt::Gdt::set_backing_storage(gdt_backing)?;
    }

    // Install IDT and IDTR backing storage in the scratch region, outside the CoW snapshot.
    //
    // Safety: idt_ptr returns a pointer aligned to IDTE_ALIGNMENT (enforced at compile
    // time by the scratch_layout! macro). idtr_ptr returns a word-aligned pointer.
    // The scratch region is identity-mapped, zeroed by Hyperlight, and never freed,
    // so the pointers are valid and outlive all IDT usage. This is the only call to
    // Idt::set_backing_storage() in the Hyperlight init path.
    unsafe {
        let idt_backing: *mut Idte = idt_ptr(scratch_reserved_base) as *mut Idte;
        let idtr_backing: *mut Idtr = idtr_ptr(scratch_reserved_base) as *mut Idtr;
        idt::Idt::set_backing_storage(idt_backing, idtr_backing)?;
    }

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
        #[cfg(feature = "pit")]
        _pit: register_pit(ioports)?,
        physical_memory_layout: Some(physical_memory_layout),
        kpool_bitmap: Some(kpool_bitmap),
    })
}

///
/// # Description
///
/// Validates that the three memory regions (snapshot, RAMFS, scratch) cover exactly
/// [`MEMORY_SIZE`].
///
/// # Parameters
///
/// - `snapshot_budget_size`: Size of the snapshot region in bytes.
/// - `ramfs_size`: Size of the RAMFS region in bytes (0 when absent).
/// - `scratch_size`: Size of the scratch region in bytes.
///
/// # Returns
///
/// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
///
fn check_memory_size(
    snapshot_budget_size: usize,
    ramfs_size: usize,
    scratch_size: usize,
) -> Result<(), Error> {
    let total: usize = snapshot_budget_size + ramfs_size + scratch_size;
    if total != MEMORY_SIZE {
        let reason: &str = "region budget mismatch";
        error!(
            "check_memory_size(): {} snapshot_budget({:#x}) + ramfs({:#x}) + scratch({:#x}) = \
             {:#x}, expected MEMORY_SIZE={:#x}",
            reason, snapshot_budget_size, ramfs_size, scratch_size, total, MEMORY_SIZE
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    Ok(())
}

///
/// # Description
///
/// Builds a [`SparseBitmap`] representing the sparse physical memory layout.
///
/// Each disjoint physical region (snapshot, RAMFS, scratch) gets its own chunk in the bitmap.
/// However, bitmap storage is byte-aligned so the snapshot chunk's last byte may cover phantom
/// frames beyond the snapshot end.  If the RAMFS starts within that padded range the two chunks
/// would overlap, so they are merged into a single "low" chunk.  When the RAMFS is absent or far
/// enough away it becomes a separate chunk.
///
/// # Parameters
///
/// - `snapshot_start_address`: Inclusive start address of the snapshot region.
/// - `snapshot_end_address`: Exclusive end address of the snapshot region.
/// - `ramfs_start_address`: Inclusive start address of the RAMFS region (0 when absent).
/// - `ramfs_end_address`: Exclusive end address of the RAMFS region (0 when absent).
/// - `scratch_start_address`: Inclusive start address of the scratch region.
/// - `scratch_end_address`: Exclusive end address of the scratch bitmap region.
/// - `storage`: Pointer and length of the backing store for the bitmap.
///
/// # Returns
///
/// Upon success, a [`SparseBitmap`] covering all physical memory regions is returned.
/// Upon failure, an error is returned instead.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer in `storage`.
/// The caller must ensure that it points to at least `storage.1` bytes of writable,
/// zero-initialised memory with a lifetime that outlives the returned bitmap.
///
unsafe fn build_physical_memory_layout(
    snapshot_start_address: usize,
    snapshot_end_address: usize,
    ramfs_start_address: usize,
    ramfs_end_address: usize,
    scratch_start_address: usize,
    scratch_end_address: usize,
    storage: (*mut u8, usize),
) -> Result<SparseBitmap, Error> {
    // 32-bit address space (MAX_GVA + 1 overflows).  Use wrapping subtraction for
    // scratch to handle this correctly.
    if snapshot_end_address < snapshot_start_address || ramfs_end_address < ramfs_start_address {
        let reason: &str = "region end address precedes start address";
        error!("build_physical_memory_layout(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bits_per_byte: usize = u8::BITS as usize;
    let snapshot_size: usize = snapshot_end_address - snapshot_start_address;
    let ramfs_size: usize = ramfs_end_address - ramfs_start_address;
    let scratch_size: usize = scratch_end_address.wrapping_sub(scratch_start_address);

    let snapshot_frames: usize = snapshot_size / mem::FRAME_SIZE;
    let snapshot_padded_end: usize = snapshot_frames.div_ceil(bits_per_byte) * bits_per_byte;
    let ramfs_start_frame: usize = if ramfs_size > 0 {
        ramfs_start_address / mem::FRAME_SIZE
    } else {
        0
    };
    let ramfs_end_frame: usize = if ramfs_size > 0 {
        ramfs_end_address / mem::FRAME_SIZE
    } else {
        0
    };

    // Merge the RAMFS into the snapshot chunk when its start falls within the
    // byte-padded range of the snapshot bitmap to avoid an overlap error.
    let merge_ramfs: bool = ramfs_size > 0 && ramfs_start_frame < snapshot_padded_end;

    // "Low" chunk covers the snapshot and, when merged, the RAMFS as well.
    let low_end_frame: usize = if merge_ramfs {
        snapshot_frames.max(ramfs_end_frame)
    } else {
        snapshot_frames
    };
    let low_bytes: usize = low_end_frame.div_ceil(bits_per_byte);
    let low_phantom: usize = low_bytes * bits_per_byte - low_end_frame;

    // Separate RAMFS chunk (only when not merged).
    let ramfs_frames: usize = ramfs_size / mem::FRAME_SIZE;
    let ramfs_bytes: usize = if !merge_ramfs && ramfs_frames > 0 {
        ramfs_frames.div_ceil(bits_per_byte)
    } else {
        0
    };
    let ramfs_phantom: usize = if ramfs_bytes > 0 {
        ramfs_bytes * bits_per_byte - ramfs_frames
    } else {
        0
    };

    let scratch_frames: usize = scratch_size / mem::FRAME_SIZE;
    let scratch_bytes: usize = scratch_frames.div_ceil(bits_per_byte);
    let scratch_phantom: usize = scratch_bytes * bits_per_byte - scratch_frames;

    let total_bytes: usize = low_bytes + ramfs_bytes + scratch_bytes;

    let (storage_ptr, storage_len): (*mut u8, usize) = storage;
    if total_bytes > storage_len {
        let reason: &str = "frame allocator storage too small for sparse layout";
        error!(
            "build_physical_memory_layout(): {} (need={:#x}, have={:#x})",
            reason, total_bytes, storage_len
        );
        return Err(Error::new(ErrorCode::OutOfMemory, reason));
    }

    let base_ptr: *mut u8 = storage_ptr;
    #[allow(unused_assignments)]
    let mut offset: usize = 0;

    // Helper: build a bitmap chunk from the shared storage at the current offset.
    // Phantom bits (byte-alignment padding beyond the last real frame) are marked as used.
    macro_rules! make_chunk {
        ($num_bytes:expr, $num_frames:expr, $num_phantom:expr) => {{
            let ptr = base_ptr.add(offset);
            let storage: RawArray<u8> = RawArray::from_raw_parts(ptr, $num_bytes)?;
            let mut bitmap: Bitmap = Bitmap::from_raw_array(storage)?;
            let nf: usize = $num_frames;
            let np: usize = $num_phantom;
            for i in 0..np {
                bitmap.set(nf + i)?;
            }
            #[allow(unused_assignments)]
            {
                offset += $num_bytes;
            }
            bitmap
        }};
    }

    // Low chunk: snapshot (and optionally RAMFS when merged).
    let snapshot_start_frame: usize = snapshot_start_address / mem::FRAME_SIZE;
    let low_bitmap: Bitmap = make_chunk!(low_bytes, low_end_frame, low_phantom);
    let mut chunks: vec::Vec<(usize, Bitmap)> = vec![(snapshot_start_frame, low_bitmap)];

    // Separate RAMFS chunk (only when not merged with the snapshot).
    if ramfs_bytes > 0 {
        let ramfs_bitmap: Bitmap = make_chunk!(ramfs_bytes, ramfs_frames, ramfs_phantom);
        chunks.push((ramfs_start_frame, ramfs_bitmap));
    }

    // Scratch chunk: tracks frames in the scratch region so the frame allocator can
    // hand them out. The bitmap uses GPAs.
    if scratch_frames > 0 {
        let scratch_start_frame: usize = scratch_start_address / mem::FRAME_SIZE;
        let scratch_bitmap: Bitmap = make_chunk!(scratch_bytes, scratch_frames, scratch_phantom);
        chunks.push((scratch_start_frame, scratch_bitmap));
    }

    SparseBitmap::new(chunks)
}

///
/// # Description
///
/// Parses the initrd image and relocates it to the default initrd base address if needed.
///
/// # Parameters
///
/// - `init_data_start`: Address where the init data blob begins.
/// - `total_allocation_size`: Total allocation size for the init data blob.
///
/// # Return Value
///
/// On success, this function returns a tuple containing:
/// - The base address of the initrd payload.
/// - The actual size of the initrd payload.
/// - A tuple with the length of the command line and the command line string.
///
/// Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it performs unchecked pointer arithmetic and dereferences raw
/// pointers. Callers must ensure that the provided memory ranges are valid and mapped.
///
unsafe fn parse_initrd_image(
    init_data_start: usize,
    total_allocation_size: usize,
) -> Result<(usize, usize, (u8, String)), Error> {
    // Check if allocation is too small to hold the initrd header.
    if total_allocation_size < INITRD_SIZE_BYTES {
        let reason: &str = "insufficient initrd allocation size";
        error!("parse_initrd_image(): {reason} (total_allocation_size={total_allocation_size})");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Read actual size and relocate only that amount
    let initrd_header: &[u8] =
        core::slice::from_raw_parts(init_data_start as *const u8, INITRD_SIZE_BYTES);
    let actual_initrd_size: usize = u64::from_le_bytes([
        initrd_header[0],
        initrd_header[1],
        initrd_header[2],
        initrd_header[3],
        initrd_header[4],
        initrd_header[5],
        initrd_header[6],
        initrd_header[7],
    ]) as usize;

    // Compute offsets and check for overflows.
    let payload_offset: usize = INITRD_SIZE_BYTES;
    let current_initrd_start: usize = match init_data_start.checked_add(payload_offset) {
        Some(value) => value,
        None => {
            let reason: &str = "initrd payload address overflow";
            error!("parse_initrd_image(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if actual size is valid.
    let required_allocation: usize = match payload_offset.checked_add(actual_initrd_size) {
        Some(value) => value,
        None => {
            let reason: &str = "initrd required allocation size overflow";
            error!("parse_initrd_image(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if allocation is too small to hold the actual initrd payload.
    if total_allocation_size < required_allocation {
        let reason: &str = "initrd payload truncated";
        error!(
            "parse_initrd_image(): {reason} (total_allocation_size={total_allocation_size}, \
             required_allocation={required_allocation})"
        );
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    debug!(
        "parse_initrd_image(): initrd found at 0x{current_initrd_start:08x}, actual size: \
         {actual_initrd_size} bytes (total allocation: {total_allocation_size} bytes)"
    );

    // Read initrd command line.
    let initrd_cmdline: (u8, String) =
        read_initrd_cmdline(current_initrd_start, actual_initrd_size, total_allocation_size)?;

    // The ELF payload sits INITRD_SIZE_BYTES past the page-aligned init_data_start,
    // so its address is not page-aligned. Move the payload back to init_data_start
    // (the size header has already been consumed) so the returned module base
    // remains page-aligned as expected by downstream module loading.
    if actual_initrd_size > 0 {
        core::ptr::copy(
            current_initrd_start as *const u8,
            init_data_start as *mut u8,
            actual_initrd_size,
        );
    }
    let initrd_base: usize = init_data_start;

    Ok((initrd_base, actual_initrd_size, initrd_cmdline))
}

///
/// # Description
///
/// Reads the initrd command line from the initrd payload if present.
///
/// # Parameters
///
/// - `initrd_start`: Address where the initrd payload begins.
/// - `initrd_size`: Size of the initrd payload.
/// - `total_allocation_size`: Total allocation size that includes the initrd and any extra data.
///
/// # Returns
///
/// On success, this function returns a tuple containing the length of the command line and the
/// command line string. Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it performs unchecked pointer arithmetic and dereferences raw
/// pointers. Callers must ensure that the provided ranges are valid and mapped.
///
unsafe fn read_initrd_cmdline(
    initrd_start: usize,
    initrd_size: usize,
    total_allocation_size: usize,
) -> Result<(u8, String), Error> {
    let args_section_size: usize =
        total_allocation_size.saturating_sub(::config::hyperlight::INITRD_SIZE_BYTES + initrd_size);

    // Check if initrd arguments section is missing.
    if args_section_size == 0 {
        let reason: &str = "initrd arguments section missing";
        error!("read_initrd_cmdline(): {reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Compute offset to arguments length byte and check for overflows.
    let args_len_offset: usize = match initrd_start.checked_add(initrd_size) {
        Some(offset) => offset,
        None => {
            let reason: &str = "initrd arguments address overflow";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Compute offset to arguments payload and check for overflows.
    let args_bytes_offset: usize = match args_len_offset.checked_add(1) {
        Some(offset) => offset,
        None => {
            let reason: &str = "initrd arguments payload address overflow";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if arguments length byte is missing.
    if args_section_size < 1 {
        let reason: &str = "initrd arguments length byte missing";
        error!("read_initrd_cmdline(): {reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let args_len: u8 = *(args_len_offset as *const u8);
    let args_payload_size: usize = usize::from(args_len);

    if args_section_size < 1 + args_payload_size {
        let reason: &str = "initrd arguments truncated";
        error!(
            "read_initrd_cmdline(): {reason} (args_section_size={args_section_size}, \
             args_len={args_len})"
        );
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let args_bytes: &[u8] =
        core::slice::from_raw_parts(args_bytes_offset as *const u8, args_payload_size);

    // Convert arguments to UTF-8 string and check for errors.
    let args_str: &str = match core::str::from_utf8(args_bytes) {
        Ok(value) => value,
        Err(_) => {
            let reason: &str = "invalid UTF-8 in initrd arguments";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    Ok((args_len, args_str.to_string()))
}

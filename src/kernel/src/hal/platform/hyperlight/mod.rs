// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod peb;
// `#[macro_use]` is required so that the `scratch_layout!` macro defined
// in this submodule is visible in the parent module without a path prefix.
#[macro_use]
mod scratch_layout;

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
    ".extern kstack",
    ".extern nanvix_dispatch_function",
    ".globl _nanvix_dispatch",
    "_nanvix_dispatch:",
    // When Hyperlight's `dispatch_call_from_host()` resumes the VM here, all general-purpose
    // registers are zeroed except RSP (restored to the value saved during evolve) and RFLAGS (RES1
    // set; ZF may be set to signal a pending TLB flush). Segment registers are not touched — they
    // retain whatever state was left after `evolve()`.
    //
    // Restore the boot stack. The KernelArguments (magic, info) are still
    // at kstack-8 from _do_start2's pushes during the evolve phase.
    //
    // NOTE: after snapshot/restore support is added, kstack will be CoW-ed
    // and this simple restore may need updating.
    "    movl $kstack, %esp",
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

/// Guest handle initialised once during `hyperlight_pre_kmain` (evolve phase)
/// and reused by `nanvix_dispatch_function` on each `sandbox.call()`.
static mut GUEST_HANDLE: Option<GuestHandle> = None;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of page tables needed for identity-mapping physical memory regions.
///
/// On Hyperlight the physical memory is split across two disjoint VA ranges: the low region
/// (snapshot + RAMFS) starting near GPA 0 and the scratch region at the top of the 32-bit address
/// space. Because both ranges occupy separate page directory entries, each can independently
/// require up to `MEMORY_SIZE / PGTAB_SIZE` page tables. We provision twice the base count to cover
/// the worst-case split.
///
pub const NUM_PAGE_TABLES: usize = 2 * (MEMORY_SIZE / mem::PGTAB_SIZE);

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

// Scratch-reserved layout.
//
// These structures are allocated in the scratch region (outside the CoW snapshot) so that
// runtime writes do not trigger copy-on-write faults.  The `scratch_layout!` macro generates
// *_OFFSET, *_SIZE constants, per-entry _ptr() accessors, and the page-aligned
// SCRATCH_RESERVED_SIZE constant from the entries below.
scratch_layout! {
    page_align = PAGE_ALIGNMENT;

    /// One bit per frame for the entire `MEMORY_SIZE` address range, plus three extra bytes
    /// so that per-chunk frame counts that are not multiples of 8 can be rounded up without
    /// overflow.
    FRAME_ALLOC_BITMAP : size = MEMORY_SIZE / (mem::FRAME_SIZE * u8::BITS as usize) + 3,
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
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Snapshot region base address (inclusive).
static SNAPSHOT_BASE: AtomicUsize = AtomicUsize::new(KERNEL_BASE_RAW);
/// Snapshot region end address (exclusive).
/// Uses `KERNEL_BASE_RAW + MEMORY_SIZE` as a generous upper bound so that early-boot code
/// (before `init()`) can construct `PhysicalAddress` values.  Tightened in `init()` once the
/// actual region boundaries are discovered.
static SNAPSHOT_END: AtomicUsize = AtomicUsize::new(KERNEL_BASE_RAW + MEMORY_SIZE);

/// RAMFS region base address (inclusive). Zero when no RAMFS is present.
static RAMFS_BASE: AtomicUsize = AtomicUsize::new(0);
/// RAMFS region end address (exclusive). Zero when no RAMFS is present.
static RAMFS_END: AtomicUsize = AtomicUsize::new(0);

/// Scratch region base address (inclusive). Zero until `init()` discovers the actual value.
static SCRATCH_BASE: AtomicUsize = AtomicUsize::new(0);
/// Scratch region end address (exclusive). Zero until `init()` discovers the actual value.
static SCRATCH_END: AtomicUsize = AtomicUsize::new(0);

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
            if let Err(e) = crate::mm::memcpy(payload_dst, data_gpa as *const u8, data_len) {
                error!("vmbus_write(): memcpy failed: {:?}", e);
            }
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
                                if let Err(e) = crate::mm::memcpy(
                                    (dest_gpa + offset) as *mut u8,
                                    chunk.as_ptr(),
                                    chunk_len,
                                ) {
                                    error!("vmbus_read(): memcpy failed: {:?}", e);
                                    break;
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
    // SAFETY: GUEST_HANDLE is initialised once in hyperlight_pre_kmain during the evolve phase
    // and never mutated again. This is a single-core guest, so there are no data races.
    if let Some(handle) = unsafe { GUEST_HANDLE.as_ref() } {
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
    } else {
        // If shutdown happens before Hyperlight guest initialization completes, halting here can
        // make evolve appear successful and leave the host with an invalid entrypoint. Abort
        // instead so the evolve phase fails explicitly.
        ::hyperlight_guest::exit::abort_with_code(&[code as u8]);
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
/// Hyperlight evolve-phase entry point, called from `_do_start2` before `kmain`.
///
/// Performs one-time initialization that subsequent `sandbox.call()` invocations depend on:
///
/// 1. Initializes the kernel heap (needed for FunctionCall deserialisation).
/// 2. Computes the PEB base address and stores a [`GuestHandle`] in [`GUEST_HANDLE`] for reuse
///    by [`nanvix_dispatch_function`].
/// 3. Halts the VM by writing the `_nanvix_dispatch` address to EAX and issuing `VmAction::Halt`
///    causing `evolve()` to return on the host with a `MultiUseSandbox`.
///
/// Hyperlight captures `regs.rax` as the dispatch entry point when the halt completes — this is how
/// the host knows where to jump on the next `call()`.  See the comment block above
/// `_nanvix_dispatch` for the full protocol.
///
/// This function never returns — the VM is halted by the `out` instruction and Hyperlight regains
/// control. The `options(noreturn)` ensures the compiler does not emit a stack epilogue that would
/// never execute.
///
#[unsafe(no_mangle)]
extern "C" fn hyperlight_pre_kmain() -> ! {
    extern "C" {
        fn _nanvix_dispatch();
        static __KERNEL_END: u8;
    }

    // Initializes the kernel heap once so FunctionCall deserialisation can allocate on every
    // subsequent sandbox.call().  On Hyperlight the heap is only initialised here (during evolve);
    // kmain skips heap init via `#[cfg(not(feature = "hyperlight"))]`.
    if let Err(_e) = unsafe { crate::mm::kheap::init() } {
        unsafe {
            core::arch::asm!("cli", "2: hlt", "jmp 2b", options(noreturn));
        }
    }

    // Compute the PEB base address and store a GuestHandle for reuse.
    let kernel_end: usize = unsafe { &__KERNEL_END as *const u8 as usize };
    let peb_base: usize = ::sys::mm::align_up(kernel_end, PAGE_ALIGNMENT)
        .expect("hyperlight_pre_kmain(): PEB align_up overflow");
    let peb_ptr: *mut HyperlightPEB = peb_base as *mut HyperlightPEB;

    // Patch PEB scratch pointers from GVA to GPA.
    //
    // The host writes input_stack.ptr and output_stack.ptr using scratch_base_gva()
    // (derived from MAX_GVA).  On the Hyperlight i686 platform the guest runs with
    // identity mapping (GVA == GPA), but the KVM memory slot for scratch is placed at
    // scratch_base_gpa() (derived from MAX_GPA).  When MAX_GPA < MAX_GVA the two
    // diverge and the guest would read from unmapped physical addresses.  Subtract the
    // delta so the PEB pointers target the actual GPA range.
    let gva_gpa_delta: u64 = (MAX_GVA - MAX_GPA) as u64;
    if gva_gpa_delta != 0 {
        unsafe {
            (*peb_ptr).input_stack.ptr -= gva_gpa_delta;
            (*peb_ptr).output_stack.ptr -= gva_gpa_delta;
        }
    }

    unsafe {
        GUEST_HANDLE = Some(GuestHandle::init(peb_ptr));
    }

    let dispatch_addr: u32 = _nanvix_dispatch as *const () as u32;
    // SAFETY: Port I/O write targets Hyperlight's VmAction::Halt port with the dispatch address in
    // EAX. This halts the VM and causes evolve() to return.  ESP must be 16-byte aligned at the
    // halt point because Hyperlight validates alignment when reading registers after initialise
    // completes.
    //
    // The halt+hlt sequence is entirely in assembly so no Rust stack epilogue is skipped.
    // options(noreturn) tells the compiler this block diverges.
    unsafe {
        core::arch::asm!(
            "and esp, 0xFFFFFFF0",
            "mov eax, {0:e}",
            "mov dx, {HALT_PORT}",
            "out dx, al",
            "cli",
            "2: hlt",
            "jmp 2b",
            in(reg) dispatch_addr,
            HALT_PORT = const VmAction::Halt as u16,
            options(noreturn),
        );
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
    // SAFETY: GUEST_HANDLE is initialised once in hyperlight_pre_kmain
    // during the evolve phase and never mutated again. This is a single-core
    // guest, so there are no data races.
    let handle: &GuestHandle = unsafe {
        GUEST_HANDLE
            .as_ref()
            .expect("nanvix_dispatch_function(): GUEST_HANDLE not initialised")
    };

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
    let snapshot: (usize, usize) =
        (SNAPSHOT_BASE.load(Ordering::Relaxed), SNAPSHOT_END.load(Ordering::Relaxed));
    let ramfs: (usize, usize) =
        (RAMFS_BASE.load(Ordering::Relaxed), RAMFS_END.load(Ordering::Relaxed));
    let scratch: (usize, usize) =
        (SCRATCH_BASE.load(Ordering::Relaxed), SCRATCH_END.load(Ordering::Relaxed));
    // An address is valid when the half-open interval [raw, raw+1) lies inside a region.
    region_contains(snapshot.0, snapshot.1, raw, raw + 1)
        || region_contains(ramfs.0, ramfs.1, raw, raw + 1)
        || region_contains(scratch.0, scratch.1, raw, raw + 1)
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

    let snapshot: (usize, usize) =
        (SNAPSHOT_BASE.load(Ordering::Relaxed), SNAPSHOT_END.load(Ordering::Relaxed));
    let ramfs: (usize, usize) =
        (RAMFS_BASE.load(Ordering::Relaxed), RAMFS_END.load(Ordering::Relaxed));
    let scratch: (usize, usize) =
        (SCRATCH_BASE.load(Ordering::Relaxed), SCRATCH_END.load(Ordering::Relaxed));

    region_contains(snapshot.0, snapshot.1, start, end)
        || region_contains(ramfs.0, ramfs.1, start, end)
        || region_contains(scratch.0, scratch.1, start, end)
}

///
/// # Description
///
/// Checks whether the half-open interval `[start, end)` is entirely contained within the region
/// `[region_base, region_end)`. Absent regions (base == end == 0) never contain any interval.
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
    region_base != region_end && start >= region_base && end <= region_end
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
    let scratch_end: usize = SCRATCH_END.load(Ordering::Relaxed);
    if scratch_end != 0 {
        // Post-init: the highest valid address is SCRATCH_END - 1.
        scratch_end - 1
    } else {
        // Pre-init: only the snapshot region is known.
        SNAPSHOT_END.load(Ordering::Relaxed) - 1
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

    unsafe {
        ProcessEnvironmentBlock::init(peb_ptr)?;
    };

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
    ioports.register_read_write(::config::hyperlight::PV_TIMER_PORT)?;
    let mut pv_timer_port = ioports.allocate_read_write(::config::hyperlight::PV_TIMER_PORT)?;
    pv_timer_port.write32(::config::hyperlight::TIMER_PERIOD_US);

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
        static __KERNEL_END: u8;
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

        RAMFS_BASE.store(ramfs_base, Ordering::Relaxed);
        RAMFS_END.store(ramfs_end, Ordering::Relaxed);
        info!("ramfs region: [{:#010x}, {:#010x})", ramfs_base, ramfs_end);
    }

    // Derive scratch region addresses from the host-provided scratch_size.
    // scratch_size covers the full range [scratch_base, scratch_end) and includes the last
    // page that Hyperlight reserves for bookkeeping.  That page is included in the bitmap
    // but marked as reserved (used) so the frame allocator never hands it out.
    // Validate that scratch_size is non-zero and large enough to contain the required scratch
    // regions (input buffer + output buffer + allocator storage pages + one I/O page +
    // one reserved last page).
    let min_scratch_size: usize = INPUT_DATA_BUFFER_SIZE
        + OUTPUT_DATA_BUFFER_SIZE
        + SCRATCH_RESERVED_SIZE
        + 2 * mem::PAGE_SIZE;
    if scratch_size == 0 || scratch_size < min_scratch_size {
        let reason: &str = "scratch_size is too small for required scratch regions";
        error!(
            "init(): {} (scratch_size={:#x}, min={:#x})",
            reason, scratch_size, min_scratch_size
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    // scratch_base = MAX_GPA - scratch_size + 1.
    // MAX_GPA (not MAX_GVA) because the guest uses identity mapping (GVA == GPA)
    // and the KVM memory slot is placed relative to MAX_GPA.
    let scratch_base_address: usize = MAX_GPA - scratch_size + 1;
    // scratch_end is the exclusive end of the full scratch range, including the last page
    // reserved for Hyperlight bookkeeping metadata.
    let scratch_end_address: usize =
        scratch_base_address
            .checked_add(scratch_size)
            .ok_or_else(|| {
                let reason: &str = "scratch region end address overflow";
                error!("init(): {}", reason);
                Error::new(ErrorCode::InvalidArgument, reason)
            })?;

    // Record scratch region bounds for is_valid_physical_address.
    SCRATCH_BASE.store(scratch_base_address, Ordering::Relaxed);
    SCRATCH_END.store(scratch_end_address, Ordering::Relaxed);
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

    // Reserve pages at the start of the free scratch area for the frame allocator bitmap,
    // kpool bitmap, and GDT backing stores.  This memory is in scratch (never snapshot/CoW)
    // and is registered as a reserved memory region so the frame allocator never hands it out.
    let scratch_reserved_base: usize =
        scratch_base_address + INPUT_DATA_BUFFER_SIZE + OUTPUT_DATA_BUFFER_SIZE;
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
             size={:#x})",
            scratch_reserved_base,
            scratch_reserved_base + SCRATCH_RESERVED_SIZE,
            FRAME_ALLOC_BITMAP_SIZE,
            KPOOL_BITMAP_SIZE,
            GDT_SIZE,
            SCRATCH_RESERVED_SIZE,
        );
    }
    {
        let scratch_io_page: usize = scratch_end_address - 2 * mem::PAGE_SIZE;
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
        let scratch_reserved_page: usize = scratch_end_address - mem::PAGE_SIZE;
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
    // pt_overhead is always 0 with nanvix-unstable (Hyperlight skips PT generation).
    // The variable is acknowledged here for forward-compatibility.
    let _ = pt_overhead;
    let snapshot_end_address: usize = KERNEL_BASE_RAW + snapshot_budget_size;
    SNAPSHOT_END.store(snapshot_end_address, Ordering::Relaxed);
    info!("snapshot region: [{:#010x}, {:#010x})", KERNEL_BASE_RAW, snapshot_end_address);

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
        build_physical_memory_layout(
            KERNEL_BASE_RAW,
            snapshot_end_address,
            ramfs_base,
            ramfs_end_address,
            scratch_base_address,
            scratch_end_address,
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
    // Validate that region ends are not before their starts.
    if snapshot_end_address < snapshot_start_address
        || ramfs_end_address < ramfs_start_address
        || scratch_end_address < scratch_start_address
    {
        let reason: &str = "region end address precedes start address";
        error!("build_physical_memory_layout(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bits_per_byte: usize = u8::BITS as usize;
    let snapshot_size: usize = snapshot_end_address - snapshot_start_address;
    let ramfs_size: usize = ramfs_end_address - ramfs_start_address;
    let scratch_size: usize = scratch_end_address - scratch_start_address;

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
    // Phantom bits are trailing padding bits in the byte-aligned bitmap that do not
    // correspond to real frames. Pre-mark them as used so alloc() never returns them.
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
    let mut offset: usize = 0;

    // Build a bitmap chunk from the shared storage at the current offset.
    // `num_bytes` is the byte-aligned storage size, `num_frames` is the real frame count,
    // and `num_phantom` is the number of trailing padding bits to pre-mark as used.
    let mut make_chunk =
        |num_bytes: usize, num_frames: usize, num_phantom: usize| -> Result<Bitmap, Error> {
            let storage: RawArray<u8> = RawArray::from_raw_parts(base_ptr.add(offset), num_bytes)?;
            let mut bitmap: Bitmap = Bitmap::from_raw_array(storage)?;
            for i in 0..num_phantom {
                bitmap.set(num_frames + i)?;
            }
            offset += num_bytes;
            Ok(bitmap)
        };

    // Low chunk: snapshot (and optionally RAMFS when merged).
    let snapshot_start_frame: usize = snapshot_start_address / mem::FRAME_SIZE;
    let low_bitmap: Bitmap = make_chunk(low_bytes, low_end_frame, low_phantom)?;
    let mut chunks: vec::Vec<(usize, Bitmap)> = vec![(snapshot_start_frame, low_bitmap)];

    // Separate RAMFS chunk (only when not merged with the snapshot).
    if ramfs_bytes > 0 {
        let ramfs_bitmap: Bitmap = make_chunk(ramfs_bytes, ramfs_frames, ramfs_phantom)?;
        chunks.push((ramfs_start_frame, ramfs_bitmap));
    }

    // Scratch chunk: frames [scratch_start / FRAME_SIZE, ...).
    if scratch_frames > 0 {
        let scratch_bitmap: Bitmap = make_chunk(scratch_bytes, scratch_frames, scratch_phantom)?;
        chunks.push((scratch_start_address / mem::FRAME_SIZE, scratch_bitmap));
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
    // so its address is not page-aligned.  Shift it back to init_data_start (the
    // size header has already been consumed and is no longer needed).  The source
    // and destination overlap, but core::ptr::copy handles that correctly.
    let initrd_base: usize = init_data_start;
    if current_initrd_start != initrd_base {
        let src_ptr: *const u8 = current_initrd_start as *const u8;
        let dst_ptr: *mut u8 = initrd_base as *mut u8;
        core::ptr::copy(src_ptr, dst_ptr, actual_initrd_size);
        debug!(
            "parse_initrd_image(): initrd shifted from {current_initrd_start:#010x} to \
             {initrd_base:#010x}"
        );
    }

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

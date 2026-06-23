// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![allow(static_mut_refs)] // https://github.com/nanvix/kernel/issues/454
#![allow(internal_features)]
#![feature(allocator_api)] // kheap uses this.
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]
#![feature(linked_list_cursors)] // vmem uses this.
#![feature(linked_list_remove)] // vmem uses this.
#![feature(linked_list_retain)] // vmem uses this.
#![feature(never_type)] // exit() uses this.
#![cfg_attr(not(verus_keep_ghost), feature(stmt_expr_attributes))] // stdio uses this.
#![feature(likely_unlikely)] // performance hints.
#![no_std]
#![no_main]
#![allow(clippy::result_large_err)] // FIXME: introduced by thread manager.

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use crate::{
    hal::{
        io::IoMemoryAllocator,
        mem::{
            AccessPermission,
            Address,
            MemoryRegion,
            MemoryRegionType,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
        platform::madt::MadtInfo,
        Hal,
    },
    kargs::KernelArguments,
    kimage::KernelImage,
    kmod::KernelModule,
    mm::{
        elf::Elf32Fhdr,
        VirtMemoryManager,
        Vmem,
    },
    pm::ProcessManager,
};
use ::alloc::collections::LinkedList;
use ::bitmap::Bitmap;
use ::core::sync::atomic::{
    AtomicBool,
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::Error,
    pm::ProcessIdentifier,
    ExitStatus,
};

use crate::mm::kheap;

#[cfg(feature = "smp")]
use crate::mm::kredzone;

//==================================================================================================
// Modules
//==================================================================================================

#[macro_use]
mod macros;

/// Collections.
mod collections;

mod debug;
mod event;
mod hal;
mod io;
mod ipc;
mod kargs;
mod kcall;
mod kimage;
mod klog;
mod kmod;
mod kpanic;
mod mm;
#[cfg(feature = "microvm")]
mod multibin;
pub(crate) mod pm;
#[cfg(feature = "stdio")]
mod stdio;
mod uart;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Use for synchronizing the startup of application cores.
#[cfg(feature = "smp")]
mod startup {
    use crate::pm::sync::fence::Fence;
    use ::sys::error::{
        Error,
        ErrorCode,
    };

    static mut STARTUP_FENCE: Option<Fence> = None;

    pub fn init(ncores: usize) {
        unsafe {
            STARTUP_FENCE = Some(Fence::new(ncores));
        }
    }

    pub fn wait() -> Result<(), Error> {
        unsafe {
            match STARTUP_FENCE.as_ref() {
                Some(fence) => fence.wait(),
                None => {
                    let reason: &str = "startup fence not initialized";
                    error!("{reason:?}");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                },
            }
        }

        Ok(())
    }

    pub fn signal() -> Result<(), Error> {
        unsafe {
            match STARTUP_FENCE.as_ref() {
                Some(fence) => fence.signal(),
                None => {
                    let reason: &str = "startup fence not initialized";
                    error!("{reason:?}");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                },
            }
        }
        Ok(())
    }
}

/// Counts the number of cores online.
static CORES_ONLINE: AtomicUsize = AtomicUsize::new(1);

/// Performance counter for the number of times the kernel was idle.
static PERF_SCHED_KERNEL_IDLE: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of soft context switches that occurred.
static PERF_SCHED_SOFT_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of involuntary context switches that occurred.
static PERF_SCHED_HARD_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of context switches that  were triggered by `exit`.
static PERF_SCHED_EXIT_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of context switches that were triggered by `exit_thread`.
static PERF_SCHED_EXIT_THREAD_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of context switches that were triggered by `sleep`.
static PERF_SCHED_SLEEP_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of context switches that were triggered by `giveup`.
static PERF_SCHED_GIVEUP_CONTEXT_SWITCHES: AtomicUsize = AtomicUsize::new(0);

/// Performance counter for the number of times `wakeup` was called.
static PERF_SCHED_WAKEUP: AtomicUsize = AtomicUsize::new(0);

/// Number of times that `vmbus_read` was called.
static PERF_VMBUS_READ: AtomicUsize = AtomicUsize::new(0);

/// Number of times that `vmbus_write` was called.
static PERF_VMBUS_WRITE: AtomicUsize = AtomicUsize::new(0);

/// Number of IKC messages sent.
static PERF_IKC_MESSAGES_SENT: AtomicUsize = AtomicUsize::new(0);

/// Number of IKC messages received.
static PERF_IKC_MESSAGES_RECEIVED: AtomicUsize = AtomicUsize::new(0);

/// Whether the guest is entitled to take a VM snapshot.
/// Set to `true` during boot when the `snapshot` kernel option is present.
/// Consumed (set to `false`) on the first successful snapshot request.
static SNAPSHOT_ALLOWED: AtomicBool = AtomicBool::new(false);

/// Attempts to consume the one-time snapshot permission.
///
/// Returns `true` if the snapshot was allowed and the permission has now been consumed.
/// Returns `false` if the snapshot was never enabled or has already been consumed.
pub(crate) fn try_consume_snapshot() -> bool {
    SNAPSHOT_ALLOWED
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "test")]
fn test() {
    if !crate::hal::mem::test() {
        panic!("memory tests failed");
    }
    if !crate::event::test() {
        panic!("event manager tests failed");
    }
}

/// Magic value used for in-kernel verification of the kernel arguments mechanism.
#[cfg(feature = "test")]
const TEST_KERNEL_ARGS_MAGIC: &str = "test_magic=0xDEADBEEF";

///
/// # Description
///
/// Verifies that the kernel received the expected magic kernel arguments from the command line.
/// The test TOML configs pass `test_magic=0xDEADBEEF` as a kernel argument, so by the time this
/// function runs the stored value must match [`TEST_KERNEL_ARGS_MAGIC`].
///
/// # Parameters
///
/// - `kernel_args`: The kernel arguments string parsed from boot info.
///
#[cfg(feature = "test")]
fn test_kernel_args(kernel_args: &str) {
    info!("testing kernel arguments...");

    assert!(
        !kernel_args.is_empty(),
        "kernel args: expected non-empty string, got empty (was --kernel-args passed?)",
    );

    assert!(
        kernel_args == TEST_KERNEL_ARGS_MAGIC,
        "kernel args: expected={:?}, got={:?}",
        TEST_KERNEL_ARGS_MAGIC,
        kernel_args,
    );

    // Verify that the getter returns the same value that was stored.
    let stored = kargs::get_kernel_args();
    assert!(
        stored == kernel_args,
        "get_kernel_args() mismatch: expected={:?}, got={:?}",
        kernel_args,
        stored,
    );

    info!("kernel arguments test passed");
}

///
/// # Description
///
/// Spawn bootstrap servers.
///
/// # Parameters
///
/// - `mm`: A reference to the virtual memory manager to use.
/// - `kmods`: A mutable reference to the list of kernel modules to spawn.
///
/// # Returns
///
/// The number of servers that were successfully spawned.
///
fn spawn_servers(mm: &mut VirtMemoryManager, kmods: &mut LinkedList<KernelModule>) -> usize {
    // SAFETY: the process manager is initialized, this is a single-core system, interrupts are
    // disabled, and the resulting `&mut ProcessManager` does not alias `mm`.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    let mut count: usize = 0;
    // Spawn all servers.
    for kmod in kmods.iter_mut() {
        info!("spawning server: {:?}", kmod.cmdline());

        // SAFETY: `kmod.start()` points to a valid, page-aligned ELF image loaded by the
        // bootloader that remains in memory for the lifetime of the kernel.
        let elf: &Elf32Fhdr = unsafe { Elf32Fhdr::from_address(kmod.start().into_raw_value()) };
        let pid: ProcessIdentifier = {
            // Split command line into arguments and environment variables in place.
            // A single `;` separates args from env; `\;` is a literal `;`.
            // SAFETY: module pages are mapped read-write; `iter_mut` yields exclusive access
            // so no other reference aliases the cmdline bytes.
            let cmdline_buf: &mut [u8] = unsafe { kmod.cmdline_bytes_mut() };
            let (args, env): (&str, &str) = ::cmdline::split_cmdline(cmdline_buf);
            // Capture compacted length before create_process borrows args/env.
            let compacted_len: usize = args.len() + env.len();

            let result: Result<ProcessIdentifier, Error> = pm.create_process(mm, elf, args, env);

            // Update the tracked length so that a subsequent cmdline() call returns the
            // compacted content without stale trailing bytes.
            kmod.set_cmdline_len(compacted_len);

            match result {
                Ok(pid) => {
                    count += 1;
                    pid
                },
                Err(err) => {
                    warn!("failed to create server process: {:?}", err);
                    continue;
                },
            }
        };

        info!("server spawned, pid={:?}", pid);
    }

    count
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain(kargs: &KernelArguments) {
    // Install klog buffer backing storage before the first logging call.
    // Under SMP there is no klog buffer.
    #[cfg(not(feature = "smp"))]
    {
        if let Err(e) = unsafe { crate::hal::platform::setup_klog_backing_storage() } {
            panic!("failed to set up klog backing storage: {:?}", e);
        }
    }

    info!("initializing the kernel...");

    // Initialize the kernel heap.
    {
        if let Err(e) = unsafe { crate::hal::platform::setup_heap_backing_storage() } {
            panic!("failed to set up heap backing storage: {:?}", e);
        }
        if let Err(e) = unsafe { kheap::init() } {
            panic!("failed to initialize kernel heap: {:?}", e);
        }
    }

    #[cfg(feature = "test")]
    test();

    // Parse kernel arguments.
    info!("parsing kernel arguments...");
    type KernelArgs = (
        Option<MadtInfo>,
        Option<usize>,
        LinkedList<MemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        IoMemoryAllocator,
        LinkedList<KernelModule>,
        &'static str,
    );
    let (
        madt,
        mem_lower,
        mut memory_regions,
        mut mmio_regions,
        mut ioaddresses,
        mut kernel_modules,
        kernel_args,
    ): KernelArgs = match kargs.parse() {
        Ok(bootinfo) => (
            bootinfo.madt,
            bootinfo.mem_lower,
            bootinfo.memory_regions,
            bootinfo.mmio_regions,
            bootinfo.ioaddresses,
            bootinfo.kernel_modules,
            bootinfo.kernel_args,
        ),
        Err(err) => {
            panic!("failed to parse kernel arguments: {:?}", err);
        },
    };

    if !kernel_args.is_empty() {
        info!("kernel args: {:?}", kernel_args);
    }

    // Store kernel arguments in a global so they can be queried later.
    // SAFETY: called once during single-threaded boot, before any user process is started.
    unsafe {
        kargs::set_kernel_args(kernel_args);
    }

    // Parse kernel arguments into structured options.
    let kernel_options: ::alloc::vec::Vec<::koptions::KernelOption<'_>> =
        ::koptions::parse(kernel_args);
    if !kernel_options.is_empty() {
        info!("kernel options: {:?}", kernel_options);
    }

    // Enable snapshot capability if the `snapshot` option was passed.
    for opt in &kernel_options {
        if *opt == ::koptions::KernelOption::Snapshot {
            info!("snapshot capability enabled via kernel option");
            SNAPSHOT_ALLOWED.store(true, Ordering::SeqCst);
            break;
        }
    }

    // Verify that kernel arguments were stored and can be retrieved correctly.
    #[cfg(feature = "test")]
    test_kernel_args(kernel_args);

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

    // Add kernel modules to list of memory regions.
    // Track which page-aligned region bases have already been registered to avoid
    // booking the same frames twice (multibinary modules share a single image region).
    let mut registered_bases: [usize; ::multibin::MAX_ENTRIES] =
        [usize::MAX; ::multibin::MAX_ENTRIES];
    let mut registered_count: usize = 0;
    for module in kernel_modules.iter() {
        // Use only the program name (first token) as the region name to avoid large
        // heap allocations when the full command line is very long.
        let name: &str = module
            .cmdline()
            .split_once(' ')
            .map_or(module.cmdline(), |(n, _)| n);
        let raw_start: usize = module.region_base().into_virtual_address().into_raw_value();
        let size: usize = module.region_size();
        // Page-align the region: round start down and end up to page boundaries.
        // The module payload may start at an offset within a page (e.g., after a size header).
        let page_start: usize = ::sys::mm::align_down(raw_start, ::sys::mm::Alignment::Align4096);
        let raw_end: usize = match raw_start.checked_add(size) {
            Some(v) => v,
            None => panic!("kernel module region end overflows address space"),
        };
        let page_end: usize = match ::sys::mm::align_up(raw_end, ::sys::mm::Alignment::Align4096) {
            Some(v) => v,
            None => panic!("kernel module region end overflows address space"),
        };

        // Skip if this exact page-aligned base was already registered.
        let mut already_registered: bool = false;
        for base in registered_bases.iter().take(registered_count) {
            if *base == page_start {
                already_registered = true;
                break;
            }
        }
        if already_registered {
            continue;
        }
        if registered_count < registered_bases.len() {
            registered_bases[registered_count] = page_start;
            registered_count += 1;
        }

        let start: VirtualAddress = VirtualAddress::from_raw_value(page_start);
        let aligned_size: usize = page_end - page_start;
        let typ: MemoryRegionType = MemoryRegionType::Reserved;
        if let Ok(region) =
            MemoryRegion::new(name, start, aligned_size, typ, AccessPermission::RDWR)
        {
            memory_regions.push_back(region);
        }
    }

    let physical_memory_layout: Bitmap =
        match Hal::init(&mut memory_regions, &mut mmio_regions, &mut ioaddresses, &madt, mem_lower)
        {
            Ok(result) => result,
            Err(err) => {
                panic!("failed to initialize hardware abstraction layer: {:?}", err);
            },
        };

    // Initialize the memory manager.
    let root: Vmem = match mm::init(memory_regions, mmio_regions, physical_memory_layout) {
        Ok(root) => root,
        Err(err) => {
            panic!("failed to initialize memory manager: {:?}", err);
        },
    };

    // Check boot stack guard watermark for corruption.
    #[cfg(feature = "exception-stack-guard")]
    if let Err(err) = mm::kstack::check_boot_stack_guard() {
        panic!("boot stack overflow detected: {:?}", err);
    }

    if let Err(err) = pm::init(root) {
        panic!("failed to initialize process manager: {:?}", err);
    }

    // Start application cores.
    #[cfg(feature = "smp")]
    if let Some(madt) = &madt {
        use crate::{
            hal::platform::madt::MadtEntry,
            mm::kstack::KernelStack,
        };
        use ::arch::cpu::madt::MadtEntryLocalApic;
        use ::core::mem;

        // Report number of application cores.
        let ncores: usize = madt.cores_count() - 1;
        startup::init(ncores - 1);

        // Traverse all cores.
        for e in madt.entries.iter() {
            // Check if entry is a local APIC.
            if let MadtEntry::LocalApic(entry) = e {
                let coreid: u8 = entry.apic_id;

                // Check if core is enabled or online capable.
                if (entry.flags
                    & (MadtEntryLocalApic::ENABLED | MadtEntryLocalApic::ONLINE_CAPABLE))
                    == 0
                {
                    continue;
                }

                // Check if core is the bootstrap core.
                if coreid == 0 {
                    continue;
                }

                info!("starting application core {}...", coreid);

                // Allocate a kernel stack for the application core.
                // SAFETY: the memory manager is initialized and access is synchronized.
                let kstack: KernelStack =
                    match KernelStack::new(unsafe { VirtMemoryManager::get_mut() }) {
                        Ok(kstack) => kstack,
                        Err(err) => {
                            panic!(
                                "failed to allocate kernel stack for application core (error={:?})",
                                err
                            );
                        },
                    };

                // Obtain a cached version of the number of cores online.
                let cores_online: usize = CORES_ONLINE.load(Ordering::Acquire);

                // Start core.
                // SAFETY: the hardware abstraction layer is initialized and access is
                // synchronized.
                if let Err(e) = unsafe { Hal::get_mut() }
                    .intman()
                    .expect("interrupts must be supported")
                    .start_core(
                        coreid,
                        hal::platform::TRAMPOLINE_ADDRESS,
                        kstack.top().into_raw_value() as *const u8,
                    )
                {
                    panic!("failed to start application core (e={:?}", e);
                }

                // Wait for application core to come online.
                info!("waiting for core {} to come online...", coreid);
                while CORES_ONLINE.load(Ordering::Acquire) == cores_online {
                    ::arch::cpu::pause();
                }

                // Prevent the kernel stack from being deallocated.
                // TODO: instead of forgetting we should store this in a per-core structure.
                mem::forget(kstack);
            }
        }
    }

    // Print number of cores online.
    let cores_online: usize = CORES_ONLINE.load(Ordering::Acquire);
    info!("number of cores online: {}", cores_online);

    // SAFETY: the memory manager is initialized and access is synchronized.
    let status: ExitStatus =
        if spawn_servers(unsafe { VirtMemoryManager::get_mut() }, &mut kernel_modules) > 0 {
            // Enable timer interrupts, if they are supported.
            // SAFETY: the hardware abstraction layer is initialized and access is synchronized.
            if let Some(intman) = unsafe { Hal::get_mut() }.intman() {
                if let Err(e) = intman.unmask(hal::arch::InterruptNumber::Timer) {
                    panic!("failed to mask timer interrupt: {:?}", e);
                }
            }

            kcall::handler()
        } else {
            ExitStatus::ok()
        };

    #[cfg(feature = "smp")]
    startup::wait().expect("failed to synchronize application cores");

    // Dump system statistics.
    info!("System Statistics:");
    info!("- No. Times Kernel Was Idle: {:?}", PERF_SCHED_KERNEL_IDLE.load(Ordering::Relaxed));
    info!(
        "- No. Soft Context Switches: {:?}",
        PERF_SCHED_SOFT_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!(
        "- No. Hard Context Switches: {:?}",
        PERF_SCHED_HARD_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!(
        "- No. Exit Context Switches: {:?}",
        PERF_SCHED_EXIT_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!(
        "- No. Exit Thread Context Switches: {:?}",
        PERF_SCHED_EXIT_THREAD_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!(
        "- No. Sleep Context Switches: {:?}",
        PERF_SCHED_SLEEP_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!(
        "- No. Giveup Context Switches: {:?}",
        PERF_SCHED_GIVEUP_CONTEXT_SWITCHES.load(Ordering::Relaxed)
    );
    info!("- No. Wakeup Calls: {:?}", PERF_SCHED_WAKEUP.load(Ordering::Relaxed));
    info!("- Ticks: {:?}", pm::ticks());
    info!("- No. Times VMBus Read Was Called: {:?}", PERF_VMBUS_READ.load(Ordering::Relaxed));
    info!("- No. Times VMBus Write Was Called: {:?}", PERF_VMBUS_WRITE.load(Ordering::Relaxed));
    info!("- No. IKC Messages Sent: {:?}", PERF_IKC_MESSAGES_SENT.load(Ordering::Relaxed));
    info!("- No. IKC Messages Received: {:?}", PERF_IKC_MESSAGES_RECEIVED.load(Ordering::Relaxed));

    trace!("the system will shutdown now!");
    kernel_magic_string(status);
}

#[unsafe(no_mangle)]
#[cfg(feature = "smp")]
pub extern "C" fn do_ap_start(coreid: u32) {
    // Load address of the kernel stack from the red zone.
    let kstack: *const u8 = match kredzone::load(0) {
        Ok(kstack) => kstack as *const u8,
        Err(err) => {
            panic!("failed to load kernel stack address from the kernel's red zone: {:?}", err);
        },
    };

    match hal::initialize_application_core(kstack) {
        Ok(_arch) => {
            CORES_ONLINE.fetch_add(1, Ordering::Acquire);

            trace!("core {} is now online (kstack={:?})", coreid, kstack);

            #[cfg(feature = "smp")]
            startup::signal().expect("failed to signal main core");

            loop {
                core::hint::spin_loop();
            }
        },
        Err(err) => {
            panic!("failed to initialize application core: {:?}", err);
        },
    }
}

#[unsafe(no_mangle)]
#[cfg(not(feature = "smp"))]
pub extern "C" fn do_ap_start(_coreid: u32) {
    unreachable!("application cores are not supported");
}

///
/// # Description
///
/// Outputs a magic string to the console and enters an infinite loop. The continuous integration
/// system expects this to be the last thing that the kernel, and thus leverages this behavior to
/// assert for a successful execution.
///
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Returns
///
/// This function never returns.
///
pub fn kernel_magic_string(status: ExitStatus) -> ! {
    debug!("hello, world!");
    hal::platform::shutdown(status.into());
}

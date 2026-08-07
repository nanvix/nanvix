// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod arch;
pub mod cpu;
pub mod io;
pub mod mem;
pub mod platform;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "smp")]
use crate::hal::arch::Arch;
use crate::hal::{
    arch::ExceptionController,
    cpu::InterruptManager,
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
    mem::{
        MemoryRegion,
        TruncatedMemoryRegion,
        VirtualAddress,
    },
    platform::{
        madt::MadtInfo,
        Platform,
    },
};
use ::alloc::collections::linked_list::LinkedList;
use ::bitmap::Bitmap;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Hardware abstraction layer storage.
static mut HAL: MaybeUninit<Hal> = MaybeUninit::uninit();

/// Whether the hardware abstraction layer has been initialized.
static HAL_INIT: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that describes components of the hardware abstraction layer.
///
pub struct Hal {
    /// Platform.
    _platform: Platform,
    /// I/O port allocator.
    ioports: IoPortAllocator,
    /// I/O memory allocator.
    ioaddresses: IoMemoryAllocator,
    /// Interrupt manager.
    intman: Option<cpu::InterruptManager>,
    /// Exception controller.
    excpman: ExceptionController,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Hal {
    ///
    /// # Description
    ///
    /// Initializes the hardware abstraction layer.
    ///
    /// # Parameters
    ///
    /// - `memory_regions`: Memory regions.
    /// - `mmio_regions`: MMIO regions.
    /// - `ioaddresses`: I/O memory allocator.
    /// - `madt`: MADT information.
    /// - `mem_lower`: Lower memory size.
    ///
    /// # Returns
    ///
    /// Upon success, the physical memory layout bitmap is returned.  Upon failure, an error is
    /// returned instead.
    ///
    /// # Panics
    ///
    /// This function panics if the hardware abstraction layer is already initialized.
    ///
    pub fn init(
        memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
        mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        ioaddresses: &mut IoMemoryAllocator,
        madt: &Option<MadtInfo>,
        mem_lower: Option<usize>,
    ) -> Result<Bitmap, Error> {
        // Check if the hardware abstraction layer is already initialized.
        if unlikely(HAL_INIT.load(ORDER)) {
            panic!("hardware abstraction layer was already initialized");
        }

        info!("initializing hardware abstraction layer...");

        let mut ioports: IoPortAllocator = IoPortAllocator::new();
        let mut platform: Platform = platform::init(
            &mut ioports,
            ioaddresses,
            memory_regions,
            mmio_regions,
            madt,
            mem_lower,
        )?;

        // Take ownership of the physical memory layout bitmap from the platform.
        // This bitmap is consumed exactly once; a `None` here means it was already taken
        // (i.e., double initialization), hence `ResourceBusy`.
        let physical_memory_layout: Bitmap = match platform.physical_memory_layout.take() {
            Some(bitmap) => bitmap,
            None => {
                let reason: &str = "physical memory layout is not available";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            },
        };

        // Verify that all MMIO regions are registered in ioaddresses.
        if mmio_regions.len() != ioaddresses.len() {
            let reason: &str = "mmio_regions count does not match ioaddresses count";
            error!(
                "mmio_regions.len()={}, ioaddresses.len()={} (error={})",
                mmio_regions.len(),
                ioaddresses.len(),
                reason
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Initialize the interrupt manager.
        let intman: Option<InterruptManager> = match platform.arch.controller.take() {
            Some(controller) => Some(InterruptManager::new(controller)?),
            None => {
                warn!("no interrupt controller found");
                None
            },
        };

        // Initialize the hardware page-table manager on 64-bit targets.
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        unsafe {
            arch::native::mem::mmu::hwpt::init();
        }

        // Initialize exception manager.
        // TODO: add comments about safety.
        let excpman: ExceptionController = unsafe { ExceptionController::init()? };

        // Take ownership of ioaddresses.
        let ioaddresses: IoMemoryAllocator = core::mem::take(ioaddresses);

        let hal: Hal = Hal {
            _platform: platform,
            ioports,
            ioaddresses,
            intman,
            excpman,
        };

        // SAFETY: This happens during kernel initialization and no other threads are running.
        unsafe { HAL.write(hal) };
        HAL_INIT.store(true, ORDER);

        Ok(physical_memory_layout)
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the hardware abstraction layer.
    ///
    /// # Safety
    ///
    /// This function panics if the hardware abstraction layer is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    ///
    /// - Access to the hardware abstraction layer is synchronized.
    ///
    #[allow(dead_code)] // TODO: remove this lint allowance when the function is used.
    pub unsafe fn get<'a>() -> &'a Hal {
        if unlikely(!HAL_INIT.load(ORDER)) {
            panic!("hardware abstraction layer is not initialized");
        }

        // SAFETY: The hardware abstraction layer has been initialized, so the value is valid.
        HAL.assume_init_ref()
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the hardware abstraction layer.
    ///
    /// # Safety
    ///
    /// This function panics if the hardware abstraction layer is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    ///
    /// - Access to the hardware abstraction layer is synchronized.
    ///
    pub unsafe fn get_mut<'a>() -> &'a mut Hal {
        if unlikely(!HAL_INIT.load(ORDER)) {
            panic!("hardware abstraction layer is not initialized");
        }

        // SAFETY: The hardware abstraction layer has been initialized, so the value is valid.
        HAL.assume_init_mut()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the I/O port allocator.
    ///
    pub fn ioports(&mut self) -> &mut IoPortAllocator {
        &mut self.ioports
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the I/O memory allocator.
    ///
    pub fn ioaddresses(&mut self) -> &mut IoMemoryAllocator {
        &mut self.ioaddresses
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the interrupt manager.
    ///
    pub fn intman(&mut self) -> Option<&mut cpu::InterruptManager> {
        self.intman.as_mut()
    }

    ///
    /// # Description
    ///
    /// Returns whether the hardware abstraction layer is interrupt capable.
    ///
    pub fn is_interrupt_capable(&self) -> bool {
        self.intman.is_some()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the exception controller.
    ///
    pub fn excpman(&mut self) -> &mut ExceptionController {
        &mut self.excpman
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    info!("initializing application core...");

    let arch: Arch = arch::initialize_application_core(kstack)?;

    Ok(arch)
}

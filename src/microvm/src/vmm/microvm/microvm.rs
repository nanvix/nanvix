// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # MicroVM
//!
//! This module contains the front-end implementation of the MicroVM. Backend-end implementations
//! are provided by the [`kvm`](crate::kvm) modules.
//!

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::vmm::microvm::kvm::{
    emulator::Emulator,
    partition::VirtualPartition,
    vcpu::{
        VirtualProcessor,
        VirtualProcessorExitContext,
        VirtualProcessorExitReason,
    },
    vmem::VirtualMemory,
};

use ::anyhow::Result;
use ::std::sync::{
    Arc,
    Mutex,
};
use ::arch::mem::PAGE_SIZE;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a MicroVM.
///
pub struct MicroVm {
    // Virtual partition that hosts the virtual machine.
    _partition: Arc<Mutex<VirtualPartition>>,
    // Virtual memory of the virtual machine.
    vmem: Arc<Mutex<VirtualMemory>>,
    // Virtual processor of the virtual machine.
    vcpu: VirtualProcessor,
    // Emulator of the virtual machine.
    emulator: Emulator,
    // If present, initial RAM disk location and size.
    initrd: Option<(u64, usize)>,
}

unsafe impl Send for MicroVm {}
unsafe impl Sync for MicroVm {}

//==================================================================================================
// Types
//==================================================================================================

pub type InputFn = dyn FnMut(&Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()>;

pub type OutputFn = dyn FnMut(&Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()>;

//==================================================================================================
// Implementations
//==================================================================================================

impl MicroVm {
    ///
    /// # Description
    ///
    /// Creates a MicroVM.
    ///
    /// # Parameters
    ///
    /// - `memory_size`: Size of the virtual memory of the virtual machine.
    /// - `input`: Input function used for emulating I/O port reads.
    /// - `output`: Output function used for emulating I/O port writes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the MicroVM that was created. Otherwise, it
    /// returns an error.
    ///
    pub fn new(memory_size: usize, input: Box<InputFn>, output: Box<OutputFn>) -> Result<Self> {
        trace!("new(): memory_size={memory_size}");
        crate::timer!("vm_creation");

        let partition: Arc<Mutex<VirtualPartition>> =
            Arc::new(Mutex::new(VirtualPartition::new()?));

        let vmem: Arc<Mutex<VirtualMemory>> =
            Arc::new(Mutex::new(VirtualMemory::new(partition.clone(), memory_size)?));

        let vcpu: VirtualProcessor = VirtualProcessor::new(partition.clone(), 0)?;

        let emulator: Emulator = Emulator::new(vmem.clone(), input, output)?;

        Ok(Self {
            _partition: partition,
            vmem,
            vcpu,
            emulator,
            initrd: None,
        })
    }

    ///
    /// # Description
    ///
    /// Loads a kernel into the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `kernel_filename`: Path to the kernel binary.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the entry point of the program that was
    /// loaded into the virtual machine. Otherwise, it returns an error.
    ///
    pub fn load_kernel(&mut self, kernel_filename: &str) -> Result<u64> {
        trace!("load_kernel(): {kernel_filename}");
        crate::timer!("vm_load_kernel");
        let entry: u64 = self
            .vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .load_kernel(kernel_filename)?;
        Ok(entry)
    }

    ///
    /// # Description
    ///
    /// Loads an initial RAM disk into the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `initrd_filename`: Path to the initial RAM disk.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn load_initrd(&mut self, initrd_filename: &str) -> Result<()> {
        trace!("load_initrd(): {initrd_filename}");
        crate::timer!("vm_load_initrd");
        let initrd: (u64, usize) = self
            .vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .load_initrd(initrd_filename)?;
        self.initrd = Some(initrd);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes a command line to the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `args`: Command line arguments to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_args(&mut self, args: &str) -> Result<()> {
        trace!("write_args(): args={args}");
        crate::timer!("vm_write_args");
        self.vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
            .write_args(args)?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resets the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `rip`: Entry point of the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn reset(&mut self, rip: u64) -> Result<()> {
        trace!("reset(): {rip:#010x}");
        crate::timer!("vm_reset");
        let rax: u64 = ::config::microvm::DEFAULT_BOOT_MAGIC as u64;

        // Check if initrd is too large.
        let nzeros: usize = ::config::microvm::DEFAULT_INITRD_BASE.trailing_zeros() as usize;
        let max_initrd_size: usize = (1 << 12) * ((1 << nzeros) - 1);
        if let Some((_, initrd_size)) = self.initrd {
            if initrd_size > max_initrd_size {
                return Err(anyhow::anyhow!(
                    "initrd is too large (initrd_size={initrd_size}, max_initrd_size={max_initrd_size:?})",
                ));
            }
        }

        // Retrieve initrd information.
        let (initrd_base, initrd_size): (u64, u64) = match self.initrd {
            Some((base, size)) => (base, size as u64),
            None => (0, 0),
        };

        // Ensure that the initrd base and size are aligned to page size boundaries.
        assert_eq!(initrd_base as usize % PAGE_SIZE, 0, "initrd base is not aligned to page size");
        assert_eq!(initrd_size as usize % PAGE_SIZE, 0, "initrd size is not aligned to page size");

        // Encode initrd location and size:
        // - Lower bits encode the size in 4KB pages
        // - Higher bits encode the base address
        let rbx: u64 =
            (initrd_base & !((1 << nzeros) - 1)) | ((initrd_size >> 12) & ((1 << nzeros) - 1));

        self.vcpu.reset(rip, rax, rbx)
    }

    ///
    /// # Description
    ///
    /// Runs the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn run(&mut self) -> Result<u16> {
        trace!("run()");
        crate::timer!("vm_run");

        // Run the virtual processor until it goes offline.
        while self.vcpu.is_online() {
            let exit_context: VirtualProcessorExitContext = self.vcpu.run()?;

            // Parse exit reason.
            match exit_context.reason() {
                // The guest requested to access an I/O port.
                VirtualProcessorExitReason::PmioAccess => {
                    crate::timer!("vm_run_pmio_access");
                    if let Some(exit_status) = self.emulator.handle_pmio_access(exit_context)? {
                        self.vcpu.poweroff(exit_status);
                    }
                },

                // The guest requested to halt the virtual processor.
                VirtualProcessorExitReason::Halt => {
                    self.vcpu.poweroff(0);
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReason::Unknown => {
                    return Err(anyhow::anyhow!("unknown exit reason"));
                },
            }
        }

        Ok(self.vcpu.exit_status())
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the virtual memory of the target virtual machine.
    ///
    /// # Returns
    ///
    /// A reference to the virtual memory of the target virtual machine.
    ///
    pub fn vmem(&self) -> Arc<Mutex<VirtualMemory>> {
        self.vmem.clone()
    }
}

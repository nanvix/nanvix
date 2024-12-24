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
use crate::kvm::{
    emulator::Emulator,
    partition::VirtualPartition,
    vcpu::{
        VirtualProcessor,
        VirtualProcessorExitContext,
        VirtualProcessorExitReason,
    },
    vmem::VirtualMemory,
};

use crate::config;
use ::anyhow::Result;
use ::std::{
    cell::RefCell,
    rc::Rc,
};

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
    _partition: Rc<RefCell<VirtualPartition>>,
    // Virtual memory of the virtual machine.
    vmem: Rc<RefCell<VirtualMemory>>,
    // Virtual processor of the virtual machine.
    vcpu: VirtualProcessor,
    // Emulator of the virtual machine.
    emulator: Emulator,
    // If present, initial RAM disk location and size.
    initrd: Option<(u64, usize)>,
}

//==================================================================================================
// Types
//==================================================================================================

pub type InputFn = dyn FnMut(&Rc<RefCell<VirtualMemory>>, u32, usize) -> Result<()>;

pub type OutputFn = dyn FnMut(&Rc<RefCell<VirtualMemory>>, u32, usize) -> Result<()>;

//==================================================================================================
// Implementations
//==================================================================================================

impl MicroVm {
    /// I/O port that is connected to the standard output of the virtual machine.
    pub const STDOUT_PORT: u16 = config::STDOUT_PORT;
    /// I/O port that is connected to the standard input of the virtual machine.
    pub const STDIN_PORT: u16 = config::STDIN_PORT;
    /// I/O port that enables the guest to invoke functionalities of the virtual machine monitor.
    pub const VMM_PORT: u16 = config::VMM_PORT;

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
        trace!("new(): memory_size={}", memory_size);
        crate::timer!("vm_creation");

        let partition: Rc<RefCell<VirtualPartition>> =
            Rc::new(RefCell::new((VirtualPartition::new())?));

        let vmem: Rc<RefCell<VirtualMemory>> =
            Rc::new(RefCell::new(VirtualMemory::new(partition.clone(), memory_size)?));

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
        trace!("load_kernel(): {}", kernel_filename);
        crate::timer!("vm_load_kernel");
        let entry: u64 = self.vmem.borrow_mut().load_kernel(kernel_filename)?;
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
        trace!("load_initrd(): {}", initrd_filename);
        crate::timer!("vm_load_initrd");
        let initrd: (u64, usize) = self.vmem.borrow_mut().load_initrd(initrd_filename)?;
        self.initrd = Some(initrd);
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
        trace!("reset(): {:#010x}", rip);
        crate::timer!("vm_reset");
        let rax: u64 = config::MICROVM_MAGIC as u64;

        // Encode initrd location and size:
        // - Lower 12 bits encode the size in 4KB pages
        // - Higher bits encode the base address
        let (initrd_base, initrd_size): (u64, u64) = match self.initrd {
            Some((base, size)) => (base, size as u64),
            None => (0, 0),
        };
        let rbx: u64 = (initrd_base & 0xfffff000) | ((initrd_size >> 12) & 0xfff);

        self.vcpu.reset(rip, rax, rbx)
    }

    ///
    /// # Description
    ///
    /// Runs the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn run(&mut self) -> Result<()> {
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
                    if !(self.emulator.handle_pmio_access(exit_context)?) {
                        self.vcpu.poweroff();
                    }
                },

                // The guest requested to halt the virtual processor.
                VirtualProcessorExitReason::Halt => {
                    self.vcpu.poweroff();
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReason::Unknown => {
                    return Err(anyhow::anyhow!("unknown exit reason"));
                },
            }
        }

        Ok(())
    }
}

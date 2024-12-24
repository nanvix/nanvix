// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kvm::{
        vcpu::VirtualProcessorExitContext,
        vmem::VirtualMemory,
    },
    microvm::{
        InputFn,
        MicroVm,
        OutputFn,
    },
};
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
/// A structure that represents an instruction emulator for the virtual machine.
///
pub struct Emulator {
    vmem: Rc<RefCell<VirtualMemory>>,
    /// Input function used for emulating I/O port reads.
    input: Box<InputFn>,
    /// Output function used for emulating I/O port writes.
    output: Box<OutputFn>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Emulator {
    ///
    /// # Description
    ///
    /// Creates a new emulator.
    ///
    /// # Parameters
    ///
    /// - `input`: Input function used for emulating I/O port reads.
    /// - `output`: Output function used for emulating I/O port writes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the new emulator. Otherwise, it returns an
    /// error.
    ///
    pub fn new(
        vmem: Rc<RefCell<VirtualMemory>>,
        input: Box<InputFn>,
        output: Box<OutputFn>,
    ) -> Result<Self> {
        trace!("new()");
        Ok(Self {
            vmem,
            input,
            output,
        })
    }

    ///
    /// # Description
    ///
    /// Emulates an I/O port access.
    ///
    /// # Parameters
    ///
    /// - `vcpu`: Virtual processor on which the I/O port access occurred.
    /// - `exit_context`: Context in which the I/O port access occurred.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method a boolean value that encodes wether the virtual
    /// processor should be resumed (`true`) or not .(`false`). If an error is encountered, an error
    /// is returned instead.
    ///
    pub fn handle_pmio_access(
        &mut self,
        exit_context: VirtualProcessorExitContext,
    ) -> Result<bool> {
        // Parse context.
        match exit_context {
            // Read from an I/O port.
            VirtualProcessorExitContext::PmioIn(port, _data) => {
                // Read from an I/O port that is not supported.
                let reason: String = format!("read from unsupported port i/o (port={:#06x})", port);
                error!("handle_pmio_access(): {}", reason);
                anyhow::bail!(reason);
            },
            // Write to an I/O port.
            VirtualProcessorExitContext::PmioOut(port, data, size) => match port {
                // Write to standard output.
                MicroVm::STDOUT_PORT => {
                    (self.output)(&self.vmem, data, size)?;
                },
                // Read from standard input.
                MicroVm::STDIN_PORT => {
                    (self.input)(&self.vmem, data, size)?;
                },
                // Write to the virtual machine monitor port.
                MicroVm::VMM_PORT => {
                    // TODO: check if data matches an expected command.
                    return Ok(false);
                },
                // Write to an I/O port that is not supported.
                _ => {
                    let reason: String =
                        format!("write to unsupported port i/o (port={:#06x})", port);
                    error!("handle_pmio_access(): {}", reason);
                    anyhow::bail!(reason);
                },
            },
            // Unexpected I/O port access.
            _ => {
                // This should never happen, as all I/O port accesses are emulated above.
                unreachable!("unexpected i/O port access");
            },
        }

        Ok(true)
    }
}

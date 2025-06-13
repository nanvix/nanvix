// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::{
    kvm::{
        vcpu::VirtualProcessorExitContext,
        vmem::VirtualMemory,
    },
    microvm::{
        InputFn,
        OutputFn,
    },
};
use ::anyhow::Result;
use ::std::sync::{
    Arc,
    mpsc::Sender,
    Mutex,
};
use ::sys::ipc::Message;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents an instruction emulator for the virtual machine.
///
pub struct Emulator {
    vmem: Arc<Mutex<VirtualMemory>>,
    /// Input function used for emulating I/O port reads.
    input: Box<InputFn>,
    /// Output function used for emulating I/O port writes.
    output: Box<OutputFn>,
    /// Channel to tell the orchestrator all vcpus have paused and are ready for snapshots.
    // NOTE(gribel): when there are multiple vcpus, there will be a single stdin / stdout.
    //   Having this channel in the Emulator instead of in each vcpu
    //   reduces the number of messages between the orchestrator and the vcpus from O(n) to O(1),
    //   with `n` being the number of vcpus.
    _paused_tx: Sender<Message>,
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
        vmem: Arc<Mutex<VirtualMemory>>,
        input: Box<InputFn>,
        output: Box<OutputFn>,
        paused_tx: Sender<Message>,
    ) -> Result<Self> {
        trace!("new()");
        Ok(Self {
            vmem,
            input,
            output,
            _paused_tx: paused_tx,
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
    /// Upon successful completion, this method returns `Ok(None)` if the virtual machine should
    /// resumed or `Ok(Some(status))` if the virtual machine should be stopped. The status is
    /// returned in case the virtual machine should be stopped.  . If an error is encountered, an
    /// error is returned instead.
    ///
    pub fn handle_pmio_access(
        &mut self,
        exit_context: VirtualProcessorExitContext,
    ) -> Result<Option<u16>> {
        // Parse context.
        match exit_context {
            // Read from an I/O port.
            VirtualProcessorExitContext::PmioIn(port, _data) => {
                // Read from an I/O port that is not supported.
                let reason: String = format!("read from unsupported port i/o (port={port:#06x})");
                error!("handle_pmio_access(): {reason}");
                anyhow::bail!(reason);
            },
            // Write to an I/O port.
            VirtualProcessorExitContext::PmioOut(port, data, size) => match port {
                // Write to standard output.
                ::config::microvm::DEFAULT_STDOUT_PORT => {
                    (self.output)(&self.vmem, data, size)?;
                },
                // Read from standard input.
                ::config::microvm::DEFAULT_STDIN_PORT => {
                    (self.input)(&self.vmem, data, size)?;
                },
                // Write to the virtual machine monitor port.
                ::config::microvm::DEFAULT_VMM_PORT => {
                    // Extract parse command.
                    match (data >> 16) as u16 {
                        ::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD => {
                            // Extract status code.
                            let status: u16 = (data & 0xffff) as u16;
                            return Ok(Some(status));
                        },
                        cmd => anyhow::bail!("unknown virtual machine command (cmd=:{cmd:#06x})"),
                    }
                },
                // Write to an I/O port that is not supported.
                _ => {
                    let reason: String =
                        format!("write to unsupported port i/o (port={port:#06x})");
                    error!("handle_pmio_access(): {reason}");
                    anyhow::bail!(reason);
                },
            },
            // Unexpected I/O port access.
            _ => {
                // This should never happen, as all I/O port accesses are emulated above.
                unreachable!("unexpected i/O port access");
            },
        }

        Ok(None)
    }
}

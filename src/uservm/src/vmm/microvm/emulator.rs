// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    StderrFn,
    StdinFn,
    StdoutDirectFn,
    StdoutFn,
    guest::Guest,
    kvm::pmio::{
        PmioAccess,
        PmioWidth,
    },
    microvm::kvm::vmem::VirtualMemory,
};
use ::anyhow::Result;
use ::std::{
    io::Write,
    sync::Arc,
};
use ::syslog::{
    error,
    trace,
};
use ::tokio::sync::Mutex;
use syslog::warn;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents an instruction emulator for the virtual machine.
///
pub struct Emulator {
    /// State of the guest.
    guest: Arc<Mutex<Guest>>,
    /// Virtual memory of the guest.
    vmem: Arc<Mutex<VirtualMemory>>,
    /// Function used for emulating reads from standard input.
    stdin_fn: Box<StdinFn>,
    /// Function used for emulating writes to standard output.
    stdout_fn: Box<StdoutFn>,
    /// Function used for forwarding pre-read messages to the I/O thread (ring buffer path).
    stdout_direct_fn: Box<StdoutDirectFn>,
    /// Function used for emulating writs to standard error.
    stderr_fn: Box<StderrFn>,
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
    /// - `guest`: State of the guest.
    /// - `vmem`: Virtual memory of the guest.
    /// - `stdin_fn`: Function used for emulating reads from standard input.
    /// - `stdout_fn`: Function used for emulating writes to standard output.
    /// - `stderr_fn`: Function used for emulating writs to standard error.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the new emulator. Otherwise, it returns an
    /// error.
    ///
    pub fn new(
        guest: Arc<Mutex<Guest>>,
        vmem: Arc<Mutex<VirtualMemory>>,
        stdin_fn: Box<StdinFn>,
        stdout_fn: Box<StdoutFn>,
        stdout_direct_fn: Box<StdoutDirectFn>,
        stderr_fn: Box<StderrFn>,
    ) -> Result<Self> {
        trace!("new()");
        Ok(Self {
            guest,
            vmem,
            stdin_fn,
            stdout_fn,
            stdout_direct_fn,
            stderr_fn,
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
    pub fn handle_pmio_access(&mut self, exit_context: &PmioAccess) -> Result<Option<u16>> {
        // Parse context.
        match exit_context {
            // Read from an I/O port.
            PmioAccess::PmioIn(port, _data) => {
                // Read from an I/O port that is not supported.
                let reason: String = format!("read from unsupported port i/o (port={:#06x})", port);
                error!("handle_pmio_access(): {reason}");
                anyhow::bail!(reason);
            },
            // Write to an I/O port.
            PmioAccess::PmioOut(port, data, width) => match *port {
                // Write to standard output.
                ::config::microvm::DEFAULT_STDOUT_PORT => {
                    if *width == PmioWidth::Byte {
                        // Convert data to a character.
                        let ch: char = match char::from_u32(*data) {
                            // Valid character.
                            Some(ch) => ch,
                            // Invalid character.
                            None => {
                                let reason: String = format!("invalid character (data={data:?})");
                                error!("output(): {reason}");
                                anyhow::bail!(reason);
                            },
                        };
                        let buf: &[u8] = &[ch as u8];
                        self.stderr_fn.write_all(buf)?;
                    } else if *width == PmioWidth::Dword {
                        if *data == ::config::microvm::DOORBELL_VALUE {
                            // Doorbell: drain the TX ring buffer.
                            self.handle_tx_doorbell()?;
                        } else {
                            // Legacy PMIO: read message from guest address.
                            (self.stdout_fn)(&self.vmem, *data)?;
                        }
                    } else {
                        warn!("handle_pmio_access(): invalid write size (size={width:?})");
                    }
                },
                // Read from standard input.
                ::config::microvm::DEFAULT_STDIN_PORT => {
                    (self.stdin_fn)(&self.guest, &self.vmem, *data, width.into())?;
                },
                // Write to the virtual machine monitor port.
                ::config::microvm::DEFAULT_VMM_PORT => {
                    // Extract parse command.
                    match (*data >> 16) as u16 {
                        ::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD => {
                            // Extract status code.
                            let status: u16 = (*data & 0xffff) as u16;
                            return Ok(Some(status));
                        },
                        ::config::microvm::DEFAULT_VMM_PAUSE_CMD => {
                            return Ok(Some(::config::microvm::DEFAULT_VMM_PAUSE_CMD));
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
        }

        Ok(None)
    }

    ///
    /// # Description
    ///
    /// Handles a doorbell notification from the guest. Drains all pending messages from the
    /// TX ring buffer and forwards each one to the standard output function.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    fn handle_tx_doorbell(&mut self) -> Result<()> {
        trace!("handle_tx_doorbell()");

        let messages: Vec<::sys::ipc::Message> = {
            let locked_guest: ::tokio::sync::MutexGuard<'_, Guest> = self.guest.blocking_lock();
            let mut locked_vmem: ::tokio::sync::MutexGuard<'_, VirtualMemory> =
                self.vmem.blocking_lock();
            locked_guest.drain_tx_ring(&mut locked_vmem)?
        };

        for message in messages {
            (self.stdout_direct_fn)(message)?;
        }

        Ok(())
    }
}

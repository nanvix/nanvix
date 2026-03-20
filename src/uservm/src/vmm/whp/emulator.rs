// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The WHP emulator uses u32/u16 casts when interfacing with the Windows API.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    StderrFn,
    StdinFn,
    StdoutFn,
    guest::Guest,
    whp::{
        vcpu::exit::{
            PmioAccess,
            PmioWidth,
        },
        vmem::VirtualMemory,
    },
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    io::Write,
    sync::Arc,
};
use ::sys::ipc::VmBusMessage;
use ::tokio::sync::Mutex;

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
    /// Function used for emulating writes to standard error.
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
    /// - `stderr_fn`: Function used for emulating writes to standard error.
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
        stderr_fn: Box<StderrFn>,
    ) -> Result<Self> {
        trace!("new()");
        Ok(Self {
            guest,
            vmem,
            stdin_fn,
            stdout_fn,
            stderr_fn,
        })
    }

    ///
    /// # Description
    ///
    /// Reads a [`VmBusMessage`] from guest virtual memory at the given address.
    ///
    /// # Parameters
    ///
    /// - `envelope_addr`: Guest virtual address of the vmbus message.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the parsed vmbus message. If an error is
    /// encountered, an error is returned instead.
    ///
    fn read_envelope(&self, envelope_addr: u32) -> Result<VmBusMessage> {
        let mut envelope_bytes: [u8; VmBusMessage::SIZE] = [0; VmBusMessage::SIZE];
        self.vmem
            .blocking_lock()
            .read_bytes(envelope_addr as u64, &mut envelope_bytes)?;
        let envelope: VmBusMessage = VmBusMessage::try_from_bytes(envelope_bytes).map_err(|e| {
            let reason: String = format!("failed to parse vmbus message: {e:?}");
            error!("read_envelope(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        Ok(envelope)
    }

    ///
    /// # Description
    ///
    /// Emulates an I/O port access.
    ///
    /// # Parameters
    ///
    /// - `exit_context`: Context in which the I/O port access occurred.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns `Ok(None)` if the virtual machine should
    /// be resumed or `Ok(Some(status))` if the virtual machine should be stopped.
    ///
    pub fn handle_pmio_access(&mut self, exit_context: &PmioAccess) -> Result<Option<u16>> {
        match exit_context {
            // Read from an I/O port.
            PmioAccess::PmioIn(port, _data) => {
                // Silently return zero for well-known hardware ports read during boot.
                match *port {
                    0x20
                    | 0x21
                    | 0x22
                    | 0x23
                    | 0xA0
                    | 0xA1
                    | 0x40..=0x43
                    | 0x61
                    | 0x70
                    | 0x71
                    | 0x3F8..=0x3FF
                    | 0xCF8
                    | 0xCFC..=0xCFF => {
                        trace!(
                            "handle_pmio_access(): ignoring read from port (port={:#06x})",
                            port
                        );
                    },
                    _ => {
                        warn!(
                            "handle_pmio_access(): ignoring read from unknown port (port={:#06x})",
                            port
                        );
                    },
                }
            },
            // Write to an I/O port.
            PmioAccess::PmioOut(port, data, width) => match *port {
                // Write to standard output.
                ::config::microvm::DEFAULT_STDOUT_PORT => {
                    if *width == PmioWidth::Byte {
                        let ch: char = match char::from_u32(*data) {
                            Some(ch) => ch,
                            None => {
                                let reason: String = format!("invalid character (data={data:?})");
                                error!("output(): {reason}");
                                anyhow::bail!(reason);
                            },
                        };
                        let buf: &[u8] = &[ch as u8];
                        self.stderr_fn.write_all(buf)?;
                    } else if *width == PmioWidth::Dword {
                        let envelope: VmBusMessage = self.read_envelope(*data)?;
                        (self.stdout_fn)(&self.vmem, &envelope)?;
                    } else {
                        warn!("handle_pmio_access(): invalid write size (size={width:?})");
                    }
                },
                // Read from standard input.
                ::config::microvm::DEFAULT_STDIN_PORT => {
                    let envelope: VmBusMessage = self.read_envelope(*data)?;
                    (self.stdin_fn)(
                        &self.guest,
                        &self.vmem,
                        envelope.message_addr(),
                        width.into(),
                    )?;
                },
                // Write to the virtual machine monitor port.
                ::config::microvm::DEFAULT_VMM_PORT => match (*data >> 16) as u16 {
                    ::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD => {
                        let status: u16 = (*data & 0xffff) as u16;
                        trace!("VMM port shutdown: raw_data={data:#010x}, status={status}");
                        return Ok(Some(status));
                    },
                    ::config::microvm::DEFAULT_VMM_PAUSE_CMD => {
                        return Ok(Some(::config::microvm::DEFAULT_VMM_PAUSE_CMD));
                    },
                    cmd => anyhow::bail!("unknown virtual machine command (cmd={cmd:#06x})"),
                },
                // Write to an I/O port that is not emulated.
                // Silently ignore writes to well-known hardware initialization ports
                // (PIC, PIT, CMOS, etc.) that the kernel programs during boot.
                0x20 | 0x21 | 0xA0 | 0xA1 | 0x22 | 0x23 => {
                    // PIC (8259A) and IMCR initialization — silently ignore.
                    trace!("handle_pmio_access(): ignoring PIC/IMCR write (port={port:#06x})");
                },
                0x40..=0x43 | 0x61 => {
                    // PIT (8254) and speaker port — silently ignore.
                    trace!("handle_pmio_access(): ignoring PIT write (port={port:#06x})");
                },
                0x70 | 0x71 => {
                    // CMOS/RTC — silently ignore.
                    trace!("handle_pmio_access(): ignoring CMOS write (port={port:#06x})");
                },
                0x3F8..=0x3FF => {
                    // COM1 serial port — silently ignore.
                    trace!("handle_pmio_access(): ignoring COM1 write (port={port:#06x})");
                },
                0xCF8 | 0xCFC..=0xCFF => {
                    // PCI configuration space — silently ignore.
                    trace!("handle_pmio_access(): ignoring PCI write (port={port:#06x})");
                },
                // Write to an I/O port that is truly unsupported.
                _ => {
                    warn!(
                        "handle_pmio_access(): ignoring write to unknown port (port={port:#06x})"
                    );
                },
            },
        }

        Ok(None)
    }
}

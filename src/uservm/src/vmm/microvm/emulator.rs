// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The emulator uses u32/u16 casts when interfacing with port I/O.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::vmm::kvm::pmio::{
    PmioAccess,
    PmioWidth,
};
#[cfg(target_os = "linux")]
use crate::vmm::microvm::kvm::vmem::VirtualMemory;
#[cfg(target_os = "windows")]
use crate::vmm::microvm::whp::vcpu::exit::{
    PmioAccess,
    PmioWidth,
};
#[cfg(target_os = "windows")]
use crate::vmm::microvm::whp::vmem::VirtualMemory;
use crate::vmm::{
    StderrFn,
    StdinFn,
    StdoutFn,
    guest::Guest,
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
                // On Linux/KVM, unsupported port reads are fatal errors.
                // On Windows/WHP, well-known hardware ports read during boot are silently ignored.
                #[cfg(target_os = "linux")]
                {
                    let reason: String =
                        format!("read from unsupported port i/o (port={:#06x})", port);
                    error!("handle_pmio_access(): {reason}");
                    anyhow::bail!(reason);
                }
                #[cfg(target_os = "windows")]
                {
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
                                "handle_pmio_access(): ignoring read from unknown port \
                                 (port={:#06x})",
                                port
                            );
                        },
                    }
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
                        return Ok(Some(status));
                    },
                    ::config::microvm::DEFAULT_VMM_PAUSE_CMD => {
                        return Ok(Some(::config::microvm::DEFAULT_VMM_PAUSE_CMD));
                    },
                    ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD => {
                        return Ok(Some(::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD));
                    },
                    ::config::microvm::DEFAULT_VMM_BOOT_COMPLETE_CMD => {
                        return Ok(Some(::config::microvm::DEFAULT_VMM_BOOT_COMPLETE_CMD));
                    },
                    cmd => anyhow::bail!("unknown virtual machine command (cmd={cmd:#06x})"),
                },
                // Write to an I/O port that is not emulated.
                _ => {
                    // On Linux/KVM, unsupported port writes are fatal errors.
                    // On Windows/WHP, well-known hardware initialization ports are silently
                    // ignored.
                    #[cfg(target_os = "linux")]
                    {
                        let reason: String =
                            format!("write to unsupported port i/o (port={port:#06x})");
                        error!("handle_pmio_access(): {reason}");
                        anyhow::bail!(reason);
                    }
                    #[cfg(target_os = "windows")]
                    {
                        match *port {
                            0x20 | 0x21 | 0xA0 | 0xA1 | 0x22 | 0x23 => {
                                trace!(
                                    "handle_pmio_access(): ignoring PIC/IMCR write \
                                     (port={port:#06x})"
                                );
                            },
                            0x40..=0x43 | 0x61 => {
                                // PIT (8254) is not emulated here: on WHP the
                                // periodic timer interrupt comes from the LAPIC
                                // emulator, and channel-2 calibration is handled
                                // inline in the WHP PMIO fast path. This slow
                                // path only logs and ignores stray PIT writes.
                                trace!(
                                    "handle_pmio_access(): ignoring PIT write (port={port:#06x})"
                                );
                            },
                            0x70 | 0x71 => {
                                trace!(
                                    "handle_pmio_access(): ignoring CMOS write (port={port:#06x})"
                                );
                            },
                            0x3F8..=0x3FF => {
                                trace!(
                                    "handle_pmio_access(): ignoring COM1 write (port={port:#06x})"
                                );
                            },
                            0xCF8 | 0xCFC..=0xCFF => {
                                trace!(
                                    "handle_pmio_access(): ignoring PCI write (port={port:#06x})"
                                );
                            },
                            _ => {
                                warn!(
                                    "handle_pmio_access(): ignoring write to unknown port \
                                     (port={port:#06x})"
                                );
                            },
                        }
                    }
                },
            },
        }

        Ok(None)
    }
}

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
use crate::vmm::microvm::whp::console::AsyncConsoleWriter;
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
use ::sys::ipc::{
    VmBusMessage,
    VmBusMessageKind,
};
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
        // On Windows/WHP, decouple per-byte guest console writes (port 0xE9) from the WHP
        // partition lock held across `handle_pmio_access()`: hand bytes to a dedicated drain
        // thread via a bounded in-process buffer so that a back-pressured host sink can never
        // block the vCPU thread while it holds the partition lock.
        #[cfg(target_os = "windows")]
        let stderr_fn: Box<StderrFn> = Box::new(AsyncConsoleWriter::new(stderr_fn));
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
    /// Flushes buffered guest console output to the underlying host sink.
    ///
    /// On Windows this drains the asynchronous console writer's in-process buffer so that the
    /// final bytes the guest emitted (for example the shutdown magic string) are durable on the
    /// host sink before the VMM thread unwinds. The wait is bounded, so a starved console reader
    /// cannot wedge VM teardown.
    ///
    #[cfg(target_os = "windows")]
    pub fn flush_console(&mut self) {
        if let Err(error) = self.stderr_fn.flush() {
            warn!("flush_console(): failed to flush guest console (error={error:?})");
        }
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
                // Block kernel-log flush. The guest writes the address of a `VmBusMessage`
                // envelope describing its whole log buffer, so the host drains the buffer to the
                // console sink in a single `write_all`. This collapses what used to be one VM exit
                // per logged byte (per-byte `out8` on `DEFAULT_STDOUT_PORT`) into a single exit per
                // buffer flush -- the source-level cure for the debug/trace large-image slowness.
                ::config::microvm::DEFAULT_KLOG_PORT => {
                    if *width != PmioWidth::Dword {
                        warn!("handle_pmio_access(): invalid klog write size (size={width:?})");
                    } else {
                        let envelope: VmBusMessage = self.read_envelope(*data)?;
                        // The port is dedicated to kernel-log blocks; reject anything else rather
                        // than silently treating a stray envelope as console output.
                        if matches!(envelope.kind(), Ok(VmBusMessageKind::KlogBlock)) {
                            let bytes: Vec<u8> = read_console_block(&envelope, |addr, buf| {
                                self.vmem.blocking_lock().read_bytes(addr, buf)
                            })?;
                            if !bytes.is_empty() {
                                self.stderr_fn.write_all(&bytes)?;
                            }
                        } else {
                            warn!(
                                "handle_pmio_access(): unexpected envelope kind on klog port \
                                 (kind={:?}), dropping",
                                envelope.kind()
                            );
                        }
                    }
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Decodes a kernel-log block envelope into the bytes that should be forwarded to the console sink.
///
/// The declared length is clamped to the guest's kernel-log buffer size
/// ([`::config::kernel::KLOG_BUFFER_SIZE`]) so a guest-provided size can never drive an unbounded
/// host allocation; any excess is dropped (the console path is best-effort). The payload itself is
/// read through `read_payload`, which the caller wires to guest memory.
///
/// # Parameters
///
/// - `envelope`: Envelope describing the block (its `size` and `message_addr`).
/// - `read_payload`: Reads `buf.len()` bytes from guest memory at the given address into `buf`.
///
/// # Return Value
///
/// On success, returns the bytes to write to the console sink (possibly truncated, possibly empty).
/// Returns an error if `read_payload` fails.
///
fn read_console_block(
    envelope: &VmBusMessage,
    read_payload: impl FnOnce(u64, &mut [u8]) -> Result<()>,
) -> Result<Vec<u8>> {
    let declared: usize = envelope.size() as usize;
    let cap: usize = ::config::kernel::KLOG_BUFFER_SIZE;
    let len: usize = declared.min(cap);
    if declared > cap {
        warn!(
            "read_console_block(): klog block exceeds cap, truncating (declared={declared}, \
             cap={cap})"
        );
    }

    let mut bytes: Vec<u8> = vec![0u8; len];
    if len > 0 {
        read_payload(envelope.message_addr() as u64, &mut bytes)?;
    }
    Ok(bytes)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::read_console_block;
    use ::config::kernel::KLOG_BUFFER_SIZE;
    use ::sys::ipc::{
        VmBusMessage,
        VmBusMessageKind,
    };

    /// A normal block returns exactly the declared bytes read from the simulated guest memory.
    #[test]
    fn read_console_block_returns_declared_payload() {
        let guest_memory: &[u8] = b"[INFO][kernel] hello, klog";
        let envelope: VmBusMessage =
            VmBusMessage::new(guest_memory.len() as u32, VmBusMessageKind::KlogBlock, 0);

        let bytes: Vec<u8> = read_console_block(&envelope, |addr, buf| {
            let start: usize = addr as usize;
            buf.copy_from_slice(&guest_memory[start..start + buf.len()]);
            Ok(())
        })
        .expect("decoding a valid block must succeed");

        assert_eq!(bytes, guest_memory, "the whole declared payload must be returned");
    }

    /// A zero-length block is a no-op: it never reads guest memory and yields no bytes.
    #[test]
    fn read_console_block_empty_does_not_read() {
        let envelope: VmBusMessage = VmBusMessage::new(0, VmBusMessageKind::KlogBlock, 0xdead_beef);

        let bytes: Vec<u8> = read_console_block(&envelope, |_addr, _buf| {
            // A zero-length transfer must not touch guest memory.
            Err(::anyhow::anyhow!("read_payload must not be called for an empty block"))
        })
        .expect("an empty block must succeed without reading");

        assert!(bytes.is_empty(), "an empty block must produce no bytes");
    }

    /// An oversized declared length is clamped to the cap, bounding the host allocation.
    #[test]
    fn read_console_block_clamps_oversized_length() {
        let oversized: u32 = (KLOG_BUFFER_SIZE as u32) + 4096;
        let envelope: VmBusMessage = VmBusMessage::new(oversized, VmBusMessageKind::KlogBlock, 0);

        let mut requested: usize = 0;
        let bytes: Vec<u8> = read_console_block(&envelope, |_addr, buf| {
            requested = buf.len();
            for byte in buf.iter_mut() {
                *byte = b'x';
            }
            Ok(())
        })
        .expect("a clamped block must still succeed");

        assert_eq!(requested, KLOG_BUFFER_SIZE, "the read must be clamped to the cap");
        assert_eq!(bytes.len(), KLOG_BUFFER_SIZE, "the output must be clamped to the cap");
    }

    /// A propagated read error surfaces to the caller rather than being swallowed.
    #[test]
    fn read_console_block_propagates_read_error() {
        let envelope: VmBusMessage = VmBusMessage::new(8, VmBusMessageKind::KlogBlock, 0x1000);

        let result = read_console_block(&envelope, |_addr, _buf| {
            Err(::anyhow::anyhow!("simulated out-of-bounds guest read"))
        });

        assert!(result.is_err(), "a failing payload read must produce an error");
    }
}

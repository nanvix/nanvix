// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! The MicroVM device model exposed to the Nanvix guest.
//!
//! Nanvix talks to its VMM through three I/O ports (console/stdout, stdin, and
//! a control port) plus a shared-memory control page. This module implements
//! the host side of that contract as a [`CpuIo`] device that the OpenVMM
//! virtualization stack drives on each port-I/O exit, and forwards guest IKC
//! traffic to the [`IkcBridge`](crate::ikc::IkcBridge) (which in turn reuses the
//! Nanvix host-side daemons).

use crate::ikc::IkcBridge;
use ::guestmem::GuestMemory;
use ::std::sync::{
    Arc,
    Mutex,
};
use ::sys::ipc::VmBusMessage;
use ::virt::{
    io::CpuIo,
    StopVpSource,
    VpIndex,
};
use ::vmotherboard::Chipset;

/// How the virtual machine stopped.
#[derive(Copy, Clone, Debug)]
pub enum ExitReason {
    /// The guest requested shutdown via the control port; carries the exit code.
    Shutdown(u16),
    /// The guest requested a snapshot (unsupported); treated as a clean stop.
    Snapshot,
}

/// Line-buffered host sink for guest kernel-console output.
struct Console {
    sink: crate::ConsoleSink,
    buf: Vec<u8>,
}

/// Host-side MicroVM device model.
pub struct NanvixDevice<'a> {
    /// Guest RAM, used to read message envelopes and payloads.
    gm: GuestMemory,
    /// Bridge that routes guest IKC traffic to stdio and the host daemons.
    io: Mutex<IkcBridge>,
    /// Source used to stop the vCPU when the guest requests shutdown.
    stop: &'a StopVpSource,
    /// Reason the VM stopped, populated when a control-port command arrives.
    exit: Mutex<Option<ExitReason>>,
    /// Host sink for the guest's kernel console, with a partial-line buffer.
    console: Mutex<Console>,
    /// Legacy chipset (8259 PIC + 8254 PIT) that owns the interrupt-controller
    /// and timer port-I/O, the PIC acknowledge path, and EOI handling.
    chipset: Arc<Chipset>,
}

impl<'a> NanvixDevice<'a> {
    /// Creates a new device model.
    pub fn new(
        gm: GuestMemory,
        io: IkcBridge,
        stop: &'a StopVpSource,
        chipset: Arc<Chipset>,
        console: crate::ConsoleSink,
    ) -> Self {
        Self {
            gm,
            io: Mutex::new(io),
            stop,
            exit: Mutex::new(None),
            console: Mutex::new(Console {
                sink: console,
                buf: Vec::new(),
            }),
            chipset,
        }
    }

    /// Returns the reason the VM stopped, if a control-port command was seen.
    pub fn take_exit(&self) -> Option<ExitReason> {
        self.exit.lock().expect("exit lock").take()
    }

    /// Records a stop reason and asks the vCPU runner to stop.
    fn request_stop(&self, reason: ExitReason) {
        *self.exit.lock().expect("exit lock") = Some(reason);
        self.stop.stop();
    }

    /// Emits a single guest console byte to the configured sink, line-buffered.
    fn console_byte(&self, byte: u8) {
        use std::io::Write as _;
        let mut console = self.console.lock().expect("console lock");
        console.buf.push(byte);
        if byte == b'\n' {
            let line = std::mem::take(&mut console.buf);
            let _ = console.sink.write_all(&line);
            let _ = console.sink.flush();
        }
    }

    /// Handles a write to the control port: decodes the command and acts on it.
    fn control_command(&self, value: u32) {
        let command = (value >> 16) as u16;
        let arg = (value & 0xffff) as u16;
        match command {
            config::microvm::DEFAULT_VMM_SHUTDOWN_CMD => {
                self.request_stop(ExitReason::Shutdown(arg))
            },
            config::microvm::DEFAULT_VMM_SNAPSHOT_CMD => self.request_stop(ExitReason::Snapshot),
            config::microvm::DEFAULT_VMM_PAUSE_CMD => {
                // Pause is a no-op: there is no host actor to pause for.
                log::debug!("guest requested pause; ignoring");
            },
            config::microvm::DEFAULT_VMM_BOOT_COMPLETE_CMD => {
                log::info!("guest boot complete");
            },
            other => log::warn!("unknown control-port command {other:#06x}"),
        }
    }

    /// Reads a [`VmBusMessage`] envelope from guest memory.
    fn read_envelope(&self, gpa: u32) -> anyhow::Result<VmBusMessage> {
        let mut bytes = [0u8; VmBusMessage::SIZE];
        self.gm.read_at(u64::from(gpa), &mut bytes)?;
        VmBusMessage::try_from_bytes(bytes)
            .map_err(|e| anyhow::anyhow!("failed to parse vmbus message: {e:?}"))
    }

    /// Dispatches an outbound (guest → host) IKC envelope.
    fn on_stdout(&self, gpa: u32) {
        match self.read_envelope(gpa) {
            Ok(envelope) => {
                let mut bridge = self.io.lock().expect("io lock");
                if let Err(e) = bridge.guest_stdout(&self.gm, &envelope) {
                    log::error!("failed to handle guest stdout: {e:?}");
                }
            },
            Err(e) => log::error!("failed to read stdout envelope: {e:?}"),
        }
    }

    /// Dispatches an inbound (host → guest) IKC fetch.
    fn on_stdin(&self, gpa: u32) {
        match self.read_envelope(gpa) {
            Ok(envelope) => {
                let mut bridge = self.io.lock().expect("io lock");
                if let Err(e) = bridge.guest_stdin(&self.gm, &envelope) {
                    log::error!("failed to handle guest stdin: {e:?}");
                }
            },
            Err(e) => log::error!("failed to read stdin envelope: {e:?}"),
        }
    }
}

impl CpuIo for NanvixDevice<'_> {
    fn is_mmio(&self, _address: u64) -> bool {
        // The guest's only MMIO is the in-kernel local APIC, which the
        // hypervisor handles directly; nothing is emulated in userspace.
        false
    }

    fn acknowledge_pic_interrupt(&self) -> Option<u8> {
        self.chipset.acknowledge_pic_interrupt()
    }

    fn handle_eoi(&self, irq: u32) {
        self.chipset.handle_eoi(irq)
    }

    async fn read_mmio(&self, vp: VpIndex, address: u64, data: &mut [u8]) {
        log::warn!("unexpected mmio read vp={} addr={address:#x}", vp.index());
        data.fill(!0);
    }

    async fn write_mmio(&self, vp: VpIndex, address: u64, data: &[u8]) {
        log::warn!("unexpected mmio write vp={} addr={address:#x} len={}", vp.index(), data.len());
    }

    async fn read_io(&self, vp: VpIndex, port: u16, data: &mut [u8]) {
        // The Nanvix ports are write-only; everything else (PIC/PIT) is owned
        // by the legacy chipset.
        self.chipset.io_read(vp.index(), port, data).await;
    }

    async fn write_io(&self, vp: VpIndex, port: u16, data: &[u8]) {
        let dword = || -> Option<u32> {
            (data.len() == 4).then(|| u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
        };
        match port {
            config::microvm::DEFAULT_STDOUT_PORT => match data.len() {
                1 => self.console_byte(data[0]),
                4 => self.on_stdout(dword().expect("4-byte dword")),
                n => log::warn!("unexpected stdout write width {n}"),
            },
            config::microvm::DEFAULT_STDIN_PORT => match dword() {
                Some(gpa) => self.on_stdin(gpa),
                None => log::warn!("unexpected stdin write width {}", data.len()),
            },
            config::microvm::DEFAULT_VMM_PORT => match dword() {
                Some(value) => self.control_command(value),
                None => log::warn!("unexpected control-port write width {}", data.len()),
            },
            // Everything else (the 8259 PIC and 8254 PIT legacy ports) is owned
            // by the chipset device model.
            _ => self.chipset.io_write(vp.index(), port, data).await,
        }
    }

    fn fatal_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> virt::VpHaltReason {
        log::error!("fatal vcpu error: {error}");
        virt::VpHaltReason::TripleFault {
            vtl: hvdef::Vtl::Vtl0,
        }
    }
}

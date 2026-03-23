// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    StderrFn,
    StdinFn,
    StdoutFn,
    guest::Guest,
    kvm::pmio::{
        PmioAccess,
        PmioWidth,
    },
    microvm::kvm::vmem::VirtualMemory,
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::nvx_ring::{
    CQ_OFFSET,
    CTRL_CQ_MASK,
    CTRL_CQ_TAIL,
    CTRL_SQ_FLAGS,
    CTRL_SQ_HEAD,
    CTRL_SQ_MASK,
    CTRL_SQ_TAIL,
    CqEntry,
    SQ_OFFSET,
    SqEntry,
    SqeOpcode,
};
use ::std::{
    io::Write,
    sync::Arc,
};
use ::sys::ipc::{
    DataChunkHeader,
    VmBusMessage,
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

    /// Drains all pending SQEs from the shared ring buffer in guest memory.
    ///
    /// For each SQE, converts it to the legacy [`IkcFrame`] protocol and forwards
    /// it via the stdout callback so that existing I/O thread and linuxd code paths
    /// handle the actual syscall execution. Nop SQEs are completed immediately with
    /// a CQE written back to guest memory.
    ///
    /// # Returns
    ///
    /// The number of SQEs drained, or an error.
    fn drain_ring_buffer(&mut self) -> Result<u32> {
        let ring_base: u64 = ::config::microvm::RING_BUFFER_GPA as u64;
        let sqe_size: u64 = core::mem::size_of::<SqEntry>() as u64;

        // Read the SQ head, tail, and mask from the control block.
        let (head, tail, mask) = {
            let vmem = self.vmem.blocking_lock();
            let mut buf4: [u8; 4] = [0u8; 4];
            // sq_head at offset 0.
            vmem.read_bytes(ring_base + CTRL_SQ_HEAD as u64, &mut buf4)?;
            let head: u32 = u32::from_ne_bytes(buf4);
            // sq_tail at offset 4.
            vmem.read_bytes(ring_base + CTRL_SQ_TAIL as u64, &mut buf4)?;
            let tail: u32 = u32::from_ne_bytes(buf4);
            // sq_mask at offset 8.
            vmem.read_bytes(ring_base + CTRL_SQ_MASK as u64, &mut buf4)?;
            let mask: u32 = u32::from_ne_bytes(buf4);
            (head, tail, mask)
        };

        let pending: u32 = tail.wrapping_sub(head);
        if pending == 0 {
            return Ok(0);
        }

        let sq_base: u64 = ring_base + SQ_OFFSET as u64;
        let mut drained: u32 = 0;
        let mut current_head: u32 = head;

        while current_head != tail {
            // Read the SQE from guest memory.
            let idx: u32 = current_head & mask;
            let sqe: SqEntry = {
                let vmem = self.vmem.blocking_lock();
                let mut sqe_bytes: [u8; 64] = [0u8; 64];
                vmem.read_bytes(sq_base + (idx as u64) * sqe_size, &mut sqe_bytes)?;
                // SAFETY: SqEntry is repr(C), 64 bytes, plain data.
                unsafe { core::ptr::read(sqe_bytes.as_ptr().cast::<SqEntry>()) }
            };

            match SqeOpcode::from_u16(sqe.opcode) {
                Some(SqeOpcode::IkcMessage) => {
                    // Read the IKC message from guest memory at sqe.addr and forward
                    // via the legacy stdout callback.
                    let envelope_gpa: u32 = u32::try_from(sqe.addr)
                        .map_err(|e| anyhow::anyhow!("IKC message GPA does not fit in u32: {e}"))?;
                    let envelope: VmBusMessage = VmBusMessage::new(sqe.len, true, envelope_gpa);
                    (self.stdout_fn)(&self.vmem, &envelope)?;
                },
                Some(SqeOpcode::BulkData) => {
                    // Extract PID/TID metadata from inline_data and write a
                    // DataChunkHeader to a scratch area in guest memory so the
                    // legacy stdout callback can read it.
                    let spid: i32 =
                        i32::from_le_bytes(sqe.inline_data[0..4].try_into().unwrap_or([0; 4]));
                    let stid: i32 =
                        i32::from_le_bytes(sqe.inline_data[4..8].try_into().unwrap_or([0; 4]));
                    let dpid: i32 =
                        i32::from_le_bytes(sqe.inline_data[8..12].try_into().unwrap_or([0; 4]));
                    let dtid: i32 =
                        i32::from_le_bytes(sqe.inline_data[12..16].try_into().unwrap_or([0; 4]));
                    let header: DataChunkHeader = DataChunkHeader::new(
                        spid.into(),
                        stid.into(),
                        dpid.into(),
                        dtid.into(),
                        u32::try_from(sqe.addr).map_err(|e| {
                            anyhow::anyhow!("bulk-data GPA does not fit in u32: {e}")
                        })?,
                        sqe.len,
                    );
                    // Write header to scratch area at end of ring buffer region.
                    let scratch_gpa: u64 = ring_base + (::nvx_ring::REGION_SIZE as u64) - 64;
                    let header_bytes: [u8; core::mem::size_of::<DataChunkHeader>()] =
                        // SAFETY: DataChunkHeader is repr(C), plain data.
                        unsafe { core::mem::transmute(header) };
                    {
                        let mut vmem = self.vmem.blocking_lock();
                        vmem.write_bytes(scratch_gpa, &header_bytes)?;
                    }
                    let scratch_gpa_u32: u32 = u32::try_from(scratch_gpa)
                        .map_err(|e| anyhow::anyhow!("scratch GPA does not fit in u32: {e}"))?;
                    let bulk_envelope: VmBusMessage =
                        VmBusMessage::new(sqe.len, false, scratch_gpa_u32);
                    (self.stdout_fn)(&self.vmem, &bulk_envelope)?;
                },
                Some(SqeOpcode::Nop) => {
                    // No-op: post a CQE with result 0 directly.
                    self.post_cqe(ring_base, CqEntry::new(sqe.user_data, 0))?;
                },
                _ => {
                    warn!("drain_ring_buffer(): unknown SQE opcode {:#06x}, skipping", sqe.opcode);
                },
            }

            current_head = current_head.wrapping_add(1);
            drained += 1;
        }

        // Update the SQ head in guest memory so the producer knows slots are free.
        {
            let mut vmem = self.vmem.blocking_lock();
            vmem.write_bytes(ring_base + CTRL_SQ_HEAD as u64, &current_head.to_ne_bytes())?;
        }

        Ok(drained)
    }

    /// Writes the SQ flags field in the guest ring buffer control block.
    #[allow(dead_code)]
    fn set_ring_sq_flags(&self, flags: u32) -> Result<()> {
        let ring_base: u64 = ::config::microvm::RING_BUFFER_GPA as u64;
        let mut vmem = self.vmem.blocking_lock();
        vmem.write_bytes(ring_base + CTRL_SQ_FLAGS as u64, &flags.to_ne_bytes())?;
        Ok(())
    }

    /// Posts a single CQE to the completion queue in guest memory.
    fn post_cqe(&self, ring_base: u64, cqe: CqEntry) -> Result<()> {
        let mut vmem = self.vmem.blocking_lock();
        let cq_base: u64 = ring_base + CQ_OFFSET as u64;

        // Read CQ tail and mask.
        let mut buf4: [u8; 4] = [0u8; 4];
        // cq_tail at control offset 68 (sq: 4×4=16 + pad 48 = 64, cq_head=64, cq_tail=68).
        vmem.read_bytes(ring_base + CTRL_CQ_TAIL as u64, &mut buf4)?;
        let cq_tail: u32 = u32::from_ne_bytes(buf4);
        vmem.read_bytes(ring_base + CTRL_CQ_MASK as u64, &mut buf4)?;
        let cq_mask: u32 = u32::from_ne_bytes(buf4);

        let cq_idx: u32 = cq_tail & cq_mask;
        let cqe_bytes: [u8; 64] =
            // SAFETY: CqEntry is repr(C), 64 bytes, plain data.
            unsafe { core::mem::transmute::<CqEntry, [u8; 64]>(cqe) };
        vmem.write_bytes(cq_base + (cq_idx as u64) * 64, &cqe_bytes)?;

        // Advance CQ tail.
        let new_tail: u32 = cq_tail.wrapping_add(1);
        vmem.write_bytes(ring_base + CTRL_CQ_TAIL as u64, &new_tail.to_ne_bytes())?;

        Ok(())
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
                        // Read the vmbus message from guest memory and forward it to the
                        // stdout callback, which handles both IKC messages and data chunk transfers.
                        let envelope: VmBusMessage = self.read_envelope(*data)?;
                        (self.stdout_fn)(&self.vmem, &envelope)?;
                    } else {
                        warn!("handle_pmio_access(): invalid write size (size={width:?})");
                    }
                },
                // Read from standard input.
                ::config::microvm::DEFAULT_STDIN_PORT => {
                    // Read the vmbus message from guest memory.
                    let envelope: VmBusMessage = self.read_envelope(*data)?;
                    (self.stdin_fn)(
                        &self.guest,
                        &self.vmem,
                        envelope.message_addr(),
                        width.into(),
                    )?;
                },
                // Ring buffer doorbell: drain SQ entries.
                //
                // Adaptive polling (Tier 2) cannot spin here because the vCPU is paused
                // during KVM_EXIT_IO. The guest cannot submit new SQEs while we spin.
                // True adaptive polling requires either:
                //   - A separate polling thread (Tier 3).
                //   - ioeventfd so the doorbell does not cause a VM exit.
                // For now, we drain once and return. The SQ_NEED_WAKEUP flag is left
                // set so the guest always rings the doorbell.
                ::config::microvm::RING_DOORBELL_PORT => {
                    let drained: u32 = self.drain_ring_buffer()?;
                    trace!("handle_pmio_access(): ring doorbell, drained {} SQEs", drained);
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
                        ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD => {
                            return Ok(Some(::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD));
                        },
                        cmd => anyhow::bail!("unknown virtual machine command (cmd={cmd:#06x})"),
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
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Ring buffer drain thread — reads SQEs from guest memory via ioeventfd notification.
//!
//! When the guest writes to the doorbell port, KVM signals an [`EventFd`] without causing a VM
//! exit. This thread blocks on that eventfd and drains the submission queue, converting each SQE
//! into an [`IkcFrame`] that is forwarded to the I/O thread over a tokio channel.

use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::nvx_ring::{
    CqEntry,
    SqEntry,
    SqeOpcode,
    CTRL_CQ_MASK,
    CTRL_CQ_TAIL,
    CTRL_SQ_FLAGS,
    CTRL_SQ_HEAD,
    CTRL_SQ_MASK,
    CTRL_SQ_TAIL,
    CQ_OFFSET,
    SQ_OFFSET,
};
use ::std::sync::Arc;
use ::sys::{
    ipc::{
        DataChunk,
        DataChunkHeader,
        FixedBufferTransfer,
        IkcFrame,
        Message,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::tokio::sync::{
    Mutex,
    mpsc::Sender,
};
use ::vmm_sys_util::eventfd::EventFd;

use crate::vmm::kvm::vmem::VirtualMemory;

/// Runs the ring buffer drain loop on the current thread (blocking).
///
/// # Parameters
///
/// - `evtfd`: EventFd signaled by KVM ioeventfd on doorbell writes.
/// - `vmem`: Shared guest physical memory.
/// - `stdout_tx`: Channel to forward outbound IkcFrames to the I/O thread.
///
/// This function blocks indefinitely. It is intended to be called from a dedicated
/// `std::thread::spawn`.
pub fn run(evtfd: EventFd, vmem: Arc<Mutex<VirtualMemory>>, stdout_tx: Sender<IkcFrame>) {
    trace!("ring_drain::run(): drain thread started");

    loop {
        // Block until the guest rings the doorbell.
        match evtfd.read() {
            Ok(_count) => {},
            Err(e) => {
                // EAGAIN means no pending signals (non-blocking mode race).
                if e.raw_os_error() == Some(libc::EAGAIN) {
                    continue;
                }
                error!("ring_drain::run(): eventfd read failed: {e}");
                return;
            },
        }

        // Drain all pending SQEs.
        match drain_sq(&vmem, &stdout_tx) {
            Ok(n) if n > 0 => {
                trace!("ring_drain::run(): drained {n} SQEs via ioeventfd");
            },
            Ok(_) => {},
            Err(e) => {
                error!("ring_drain::run(): drain failed: {e}");
                return;
            },
        }
    }
}

/// Drains all pending SQEs from the submission queue and forwards them as IkcFrames.
///
/// Returns the number of SQEs drained.
fn drain_sq(vmem: &Arc<Mutex<VirtualMemory>>, stdout_tx: &Sender<IkcFrame>) -> Result<u32> {
    let ring_base: u64 = ::config::microvm::RING_BUFFER_GPA as u64;
    let sqe_size: u64 = core::mem::size_of::<SqEntry>() as u64;

    // Read the SQ head, tail, and mask from the control block.
    let (head, tail, mask) = {
        let vmem = vmem.blocking_lock();
        let mut buf4: [u8; 4] = [0u8; 4];
        vmem.read_bytes(ring_base + CTRL_SQ_HEAD as u64, &mut buf4)?;
        let head: u32 = u32::from_ne_bytes(buf4);
        vmem.read_bytes(ring_base + CTRL_SQ_TAIL as u64, &mut buf4)?;
        let tail: u32 = u32::from_ne_bytes(buf4);
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
            let vmem = vmem.blocking_lock();
            let mut sqe_bytes: [u8; 64] = [0u8; 64];
            vmem.read_bytes(sq_base + (idx as u64) * sqe_size, &mut sqe_bytes)?;
            // SAFETY: SqEntry is repr(C), 64 bytes, plain data.
            unsafe { core::ptr::read(sqe_bytes.as_ptr() as *const SqEntry) }
        };

        match SqeOpcode::from_u16(sqe.opcode) {
            Some(SqeOpcode::IkcMessage) => {
                // Read the IPC Message from guest memory and forward as an IkcFrame.
                let msg_buf: [u8; core::mem::size_of::<Message>()] = {
                    let vmem = vmem.blocking_lock();
                    let mut buf: [u8; core::mem::size_of::<Message>()] =
                        [0u8; core::mem::size_of::<Message>()];
                    vmem.read_bytes(sqe.addr, &mut buf)?;
                    buf
                };
                match Message::try_from_bytes(msg_buf) {
                    Ok(msg) => {
                        if stdout_tx.blocking_send(IkcFrame::Message(msg)).is_err() {
                            warn!("ring_drain: stdout channel closed");
                            return Ok(drained);
                        }
                    },
                    Err(_) => {
                        warn!("ring_drain: failed to parse IKC message, skipping");
                    },
                }
            },
            Some(SqeOpcode::Nop) => {
                // Post a CQE directly for Nop operations.
                post_cqe(vmem, ring_base, CqEntry::new(sqe.user_data, 0))?;
            },
            Some(SqeOpcode::BulkData) => {
                // BulkData SQEs store the payload in guest memory and the endpoint metadata in
                // inline_data, so rebuild the DataChunk explicitly instead of trying to deserialize
                // a header from the guest payload bytes.
                let parse_inline_i32 = |offset: usize| -> i32 {
                    let mut bytes: [u8; 4] = [0u8; 4];
                    bytes.copy_from_slice(&sqe.inline_data[offset..offset + 4]);
                    i32::from_le_bytes(bytes)
                };
                let source_pid: ProcessIdentifier = ProcessIdentifier::from(parse_inline_i32(0));
                let source_tid: ThreadIdentifier = ThreadIdentifier::from(parse_inline_i32(4));
                let destination_pid: ProcessIdentifier =
                    ProcessIdentifier::from(parse_inline_i32(8));
                let destination_tid: ThreadIdentifier =
                    ThreadIdentifier::from(parse_inline_i32(12));
                if sqe.is_fixed_buf() {
                    let buffer_id: u32 = match u32::try_from(sqe.addr) {
                        Ok(id) => id,
                        Err(_) => {
                            warn!(
                                "ring_drain: fixed-buffer SQE id does not fit in u32 ({:#x}), skipping",
                                sqe.addr
                            );
                            current_head = current_head.wrapping_add(1);
                            drained += 1;
                            continue;
                        },
                    };
                    let transfer: FixedBufferTransfer = FixedBufferTransfer::new(
                        source_pid,
                        source_tid,
                        destination_pid,
                        destination_tid,
                        buffer_id,
                        sqe.len,
                    );
                    if stdout_tx.blocking_send(IkcFrame::Fixed(transfer)).is_err() {
                        warn!("ring_drain: stdout channel closed");
                        return Ok(drained);
                    }
                    current_head = current_head.wrapping_add(1);
                    drained += 1;
                    continue;
                }

                let data_addr: u32 = match u32::try_from(sqe.addr) {
                    Ok(addr) => addr,
                    Err(_) => {
                        warn!(
                            "ring_drain: bulk SQE address does not fit in u32 ({:#x}), skipping",
                            sqe.addr
                        );
                        current_head = current_head.wrapping_add(1);
                        drained += 1;
                        continue;
                    },
                };
                let payload: Vec<u8> = {
                    let vmem = vmem.blocking_lock();
                    let mut buf: Vec<u8> = vec![0u8; sqe.len as usize];
                    vmem.read_bytes(sqe.addr, &mut buf)?;
                    buf
                };
                let header: DataChunkHeader = DataChunkHeader::new(
                    source_pid,
                    source_tid,
                    destination_pid,
                    destination_tid,
                    data_addr,
                    sqe.len,
                );
                let chunk: DataChunk = DataChunk::new(header, payload);
                if stdout_tx.blocking_send(IkcFrame::Bulk(chunk)).is_err() {
                    warn!("ring_drain: stdout channel closed");
                    return Ok(drained);
                }
            },
            _ => {
                warn!("ring_drain: unknown SQE opcode {:#06x}, skipping", sqe.opcode);
            },
        }

        current_head = current_head.wrapping_add(1);
        drained += 1;
    }

    // Update the SQ head in guest memory.
    {
        let mut vmem = vmem.blocking_lock();
        vmem.write_bytes(ring_base + CTRL_SQ_HEAD as u64, &current_head.to_ne_bytes())?;
    }

    Ok(drained)
}

/// Posts a single CQE to the completion queue in guest memory.
fn post_cqe(vmem: &Arc<Mutex<VirtualMemory>>, ring_base: u64, cqe: CqEntry) -> Result<()> {
    let mut vmem = vmem.blocking_lock();
    let cq_base: u64 = ring_base + CQ_OFFSET as u64;

    let mut buf4: [u8; 4] = [0u8; 4];
    vmem.read_bytes(ring_base + CTRL_CQ_TAIL as u64, &mut buf4)?;
    let cq_tail: u32 = u32::from_ne_bytes(buf4);
    vmem.read_bytes(ring_base + CTRL_CQ_MASK as u64, &mut buf4)?;
    let cq_mask: u32 = u32::from_ne_bytes(buf4);

    let cq_idx: u32 = cq_tail & cq_mask;
    let cqe_bytes: [u8; 64] =
        // SAFETY: CqEntry is repr(C), 64 bytes, plain data.
        unsafe { core::mem::transmute::<CqEntry, [u8; 64]>(cqe) };
    vmem.write_bytes(cq_base + (cq_idx as u64) * 64, &cqe_bytes)?;

    let new_tail: u32 = cq_tail.wrapping_add(1);
    vmem.write_bytes(ring_base + CTRL_CQ_TAIL as u64, &new_tail.to_ne_bytes())?;

    Ok(())
}

/// Writes the SQ flags field in the guest ring buffer control block.
#[allow(dead_code)]
fn set_sq_flags(vmem: &Arc<Mutex<VirtualMemory>>, flags: u32) -> Result<()> {
    let ring_base: u64 = ::config::microvm::RING_BUFFER_GPA as u64;
    let mut vmem = vmem.blocking_lock();
    vmem.write_bytes(ring_base + CTRL_SQ_FLAGS as u64, &flags.to_ne_bytes())?;
    Ok(())
}

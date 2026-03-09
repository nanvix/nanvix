// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Completion queue writer — posts CQEs and response data to the guest ring buffer.
//!
//! Used by the I/O thread to deliver linuxd responses directly to the guest's completion queue
//! instead of routing through the legacy memory-thread → stdin PIO path.

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::nvx_ring::{
    CqFlags,
    CqEntry,
    CqeFlags,
    CTRL_CQ_FLAGS,
    CTRL_CQ_HEAD,
    CTRL_CQ_MASK,
    CTRL_CQ_TAIL,
    CQ_OFFSET,
    DATA_OFFSET,
    DATA_SLOT_SIZE,
};
use ::std::sync::{
    Arc,
    atomic::{
        AtomicU32,
        Ordering,
    },
};
use ::sys::ipc::{
    DataChunkHeader,
    IkcFrame,
    Message,
    MessageReceiver,
    MessageSender,
    MessageType,
};
use ::tokio::sync::Mutex;

use crate::vmm::{
    IkcNotifier,
    kvm::vmem::VirtualMemory,
};

/// Writes CQEs and response payloads into the guest ring buffer.
///
/// Holds shared references to guest memory and the IKC notifier so that the I/O thread can
/// deliver responses without a VM exit.
pub struct CqWriter {
    /// Shared guest physical memory.
    vmem: Arc<Mutex<VirtualMemory>>,
    /// IKC notifier for injecting an interrupt after posting a CQE.
    notifier: IkcNotifier,
    /// Data slot allocator (round-robin).
    next_slot: AtomicU32,
}

impl CqWriter {
    /// Creates a new CQ writer.
    pub fn new(vmem: Arc<Mutex<VirtualMemory>>, notifier: IkcNotifier) -> Self {
        Self {
            vmem,
            notifier,
            next_slot: AtomicU32::new(0),
        }
    }

    /// Writes an inbound response to the ring buffer.
    ///
    /// 1. Copies the response payload into a data slot.
    ///    For bulk responses, the payload bytes are first written directly to the guest address
    ///    carried in the [`DataChunkHeader`], and the CQ entry carries a synthetic
    ///    [`MessageType::PullResponse`] notification.
    /// 2. Posts a [`CqEntry`] referencing the data slot.
    /// 3. Injects an IKC interrupt only when the guest explicitly armed CQ notifications and the
    ///    queue transitioned from empty to non-empty.
    pub async fn write_response(&self, frame: &IkcFrame) -> Result<()> {
        let ring_base: u64 = ::config::microvm::RING_BUFFER_GPA as u64;

        // Write the payload into the data slot and prepare the CQE.
        let cqe: CqEntry = match frame {
            IkcFrame::Message(msg) => {
                let slot_count: u32 = ::nvx_ring::DATA_SLOT_COUNT as u32;
                let slot_id: u32 = self.next_slot.fetch_add(1, Ordering::Relaxed) % slot_count;
                let slot_gpa: u64 =
                    ring_base + DATA_OFFSET as u64 + (slot_id as u64) * DATA_SLOT_SIZE as u64;
                let bytes: [u8; core::mem::size_of::<Message>()] = msg.clone().to_bytes();
                let write_len: usize = bytes.len().min(DATA_SLOT_SIZE);
                {
                    let mut vmem: ::tokio::sync::MutexGuard<'_, VirtualMemory> =
                        self.vmem.lock().await;
                    vmem.write_bytes(slot_gpa, &bytes[..write_len])?;
                }
                let mut cqe: CqEntry = CqEntry::new(0, write_len as i64);
                cqe.buffer_id = slot_id;
                cqe.flags = 0; // IkcFrame::Message type indicator.
                cqe
            },
            IkcFrame::Bulk(bulk) => {
                let slot_count: u32 = ::nvx_ring::DATA_SLOT_COUNT as u32;
                let slot_id: u32 = self.next_slot.fetch_add(1, Ordering::Relaxed) % slot_count;
                let slot_gpa: u64 =
                    ring_base + DATA_OFFSET as u64 + (slot_id as u64) * DATA_SLOT_SIZE as u64;
                let actual_len: u32 = u32::try_from(bulk.data().len())?;
                let completion_header: DataChunkHeader = DataChunkHeader::new(
                    bulk.header().source_pid(),
                    bulk.header().source_tid(),
                    bulk.header().destination_pid(),
                    bulk.header().destination_tid(),
                    bulk.header().data_addr(),
                    actual_len,
                );
                let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
                payload[..DataChunkHeader::SIZE].copy_from_slice(&completion_header.to_bytes());
                let completion_msg: Message = Message::new(
                    MessageSender::KERNEL,
                    MessageReceiver::KERNEL,
                    MessageType::PullResponse,
                    None,
                    payload,
                );
                let bytes: [u8; core::mem::size_of::<Message>()] = completion_msg.to_bytes();
                let write_len: usize = bytes.len().min(DATA_SLOT_SIZE);
                {
                    let mut vmem: ::tokio::sync::MutexGuard<'_, VirtualMemory> =
                        self.vmem.lock().await;
                    if !bulk.data().is_empty() {
                        vmem.write_bytes(bulk.header().data_addr() as u64, bulk.data())?;
                    }
                    vmem.write_bytes(slot_gpa, &bytes[..write_len])?;
                }
                let mut cqe: CqEntry = CqEntry::new(0, write_len as i64);
                cqe.buffer_id = slot_id;
                cqe.flags = 0;
                cqe
            },
            IkcFrame::Fixed(fixed) => {
                let source_tid_raw: i32 = fixed.source_tid().into();
                let mut cqe: CqEntry = CqEntry::new(source_tid_raw as u64, i64::from(fixed.data_len()));
                cqe.flags = CqeFlags::BUFFER.0;
                cqe.buffer_id = fixed.buffer_id();
                cqe
            },
        };

        let notify_guest: bool;

        // Post the CQE.
        {
            let mut vmem: ::tokio::sync::MutexGuard<'_, VirtualMemory> =
                self.vmem.lock().await;
            let cq_base: u64 = ring_base + CQ_OFFSET as u64;

            let mut buf4: [u8; 4] = [0u8; 4];
            vmem.read_bytes(ring_base + CTRL_CQ_HEAD as u64, &mut buf4)?;
            let cq_head: u32 = u32::from_ne_bytes(buf4);
            vmem.read_bytes(ring_base + CTRL_CQ_TAIL as u64, &mut buf4)?;
            let cq_tail: u32 = u32::from_ne_bytes(buf4);
            vmem.read_bytes(ring_base + CTRL_CQ_MASK as u64, &mut buf4)?;
            let cq_mask: u32 = u32::from_ne_bytes(buf4);
            vmem.read_bytes(ring_base + CTRL_CQ_FLAGS as u64, &mut buf4)?;
            let cq_flags: CqFlags = CqFlags(u32::from_ne_bytes(buf4));

            let cq_idx: u32 = cq_tail & cq_mask;
            let cqe_bytes: [u8; 64] =
                // SAFETY: CqEntry is repr(C), 64 bytes, plain data.
                unsafe { core::mem::transmute::<CqEntry, [u8; 64]>(cqe) };
            vmem.write_bytes(cq_base + (cq_idx as u64) * 64, &cqe_bytes)?;

            // Advance CQ tail with a release fence so the guest sees the payload.
            let new_tail: u32 = cq_tail.wrapping_add(1);
            vmem.write_bytes(ring_base + CTRL_CQ_TAIL as u64, &new_tail.to_ne_bytes())?;

            let was_empty: bool = cq_head == cq_tail;
            notify_guest = was_empty && cq_flags == CqFlags::NOTIFY_ME;
        }

        if notify_guest {
            if let Err(e) = self.notifier.notify_unconditional() {
                error!("cq_writer: failed to inject IRQ: {e}");
            }
        } else {
            trace!("cq_writer: CQ notification suppressed");
        }

        trace!("cq_writer: posted CQE");

        Ok(())
    }
}

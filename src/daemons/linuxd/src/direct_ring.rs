// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{shared_ring::SharedRing, user_vm_event::UserVmEvent};
use log::{debug, error, trace, warn};
use nvx_ring::{
    CqEntry, CqFlags, CqeFlags, SqEntry, SqFlags, SqeOpcode, CQ_OFFSET, CTRL_CQ_FLAGS,
    CTRL_CQ_HEAD, CTRL_CQ_MASK, CTRL_CQ_TAIL, CTRL_SQ_FLAGS, CTRL_SQ_HEAD, CTRL_SQ_MASK,
    CTRL_SQ_TAIL, DATA_OFFSET, DATA_SLOT_COUNT, DATA_SLOT_SIZE, SQ_OFFSET,
};
use std::{
    hint,
    io::ErrorKind,
    sync::mpsc::{
        Receiver as SyncReceiver,
        SyncSender,
        TrySendError,
        sync_channel,
    },
    sync::{
        atomic::{fence, AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
};
use sys::{
    ipc::{FixedBufferTransfer, IkcFrame, Message},
    pm::{ProcessIdentifier, ThreadIdentifier},
};
use tokio::sync::mpsc::Sender;
use user_vm_api::UserVmIdentifier;

fn futex_wait(word: *mut u32, expected: u32) -> Result<(), ErrorKind> {
    loop {
        // SAFETY: `word` points to a u32 in a shared anonymous/file-backed mapping that stays live
        // while the waiting thread runs.
        let ret: libc::c_long = unsafe {
            libc::syscall(
                libc::SYS_futex,
                word,
                libc::FUTEX_WAIT,
                expected,
                core::ptr::null::<libc::timespec>(),
            )
        };
        if ret == 0 {
            return Ok(());
        }

        let err: std::io::Error = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::EAGAIN) | Some(libc::EINTR) => return Ok(()),
            _ => return Err(err.kind()),
        }
    }
}

fn futex_wake(word: *mut u32) -> Result<(), ErrorKind> {
    // SAFETY: `word` points to a u32 in a shared anonymous/file-backed mapping that stays live
    // while the futex is in use.
    let ret: libc::c_long =
        unsafe { libc::syscall(libc::SYS_futex, word, libc::FUTEX_WAKE, i32::MAX) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error().kind());
    }
    Ok(())
}

fn signal(word: *mut u32) -> Result<(), ErrorKind> {
    let atomic: &AtomicU32 =
        // SAFETY: the notification word is 4-byte aligned and points into the shared ring mapping.
        unsafe { &*(word.cast_const().cast::<AtomicU32>()) };
    atomic.fetch_add(1, Ordering::AcqRel);
    futex_wake(word)
}

fn wait_for_signal(word: *mut u32, observed: &mut u32) -> Result<(), ErrorKind> {
    let atomic: &AtomicU32 =
        // SAFETY: the notification word is 4-byte aligned and points into the shared ring mapping.
        unsafe { &*(word.cast_const().cast::<AtomicU32>()) };
    loop {
        let current: u32 = atomic.load(Ordering::Acquire);
        if current != *observed {
            *observed = current;
            return Ok(());
        }
        futex_wait(word, current)?;
    }
}

fn parse_inline_i32(sqe: &SqEntry, offset: usize) -> i32 {
    let mut bytes: [u8; 4] = [0u8; 4];
    bytes.copy_from_slice(&sqe.inline_data[offset..offset + 4]);
    i32::from_le_bytes(bytes)
}

fn set_sq_flags(shared_ring: &SharedRing, flags: SqFlags) -> Result<(), ErrorKind> {
    shared_ring
        .write_copy(CTRL_SQ_FLAGS, flags.0)
        .map_err(|_| ErrorKind::InvalidData)
}

#[derive(Clone)]
pub enum SqWakeHandle {
    SharedFutex(Arc<SharedRing>),
    Channel(SyncSender<()>),
}

impl SqWakeHandle {
    pub fn wake(&self) -> Result<(), ErrorKind> {
        match self {
            Self::SharedFutex(shared_ring) => signal(shared_ring.sq_signal_word()),
            Self::Channel(sender) => match sender.try_send(()) {
                Ok(()) | Err(TrySendError::Full(())) => Ok(()),
                Err(TrySendError::Disconnected(())) => Err(ErrorKind::BrokenPipe),
            },
        }
    }
}

#[derive(Clone)]
pub enum CqNotifyHandle {
    SharedFutex(Arc<SharedRing>),
    Channel(tokio::sync::mpsc::Sender<()>),
}

impl CqNotifyHandle {
    fn notify_guest(&self) -> Result<(), ErrorKind> {
        match self {
            Self::SharedFutex(shared_ring) => signal(shared_ring.cq_signal_word()),
            Self::Channel(sender) => match sender.try_send(()) {
                Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Full(())) => Ok(()),
                Err(tokio::sync::mpsc::error::TrySendError::Closed(())) => {
                    Err(ErrorKind::BrokenPipe)
                },
            },
        }
    }
}

enum SqWaitStrategy {
    SharedFutex {
        signal_word_addr: usize,
        observed: u32,
    },
    Channel(SyncReceiver<()>),
}

impl SqWaitStrategy {
    fn new_shared_futex(shared_ring: &SharedRing) -> Self {
        let signal_word: *mut u32 = shared_ring.sq_signal_word();
        let atomic: &AtomicU32 =
            // SAFETY: the notification word points to a stable shared mapping for the thread lifetime.
            unsafe { &*(signal_word.cast_const().cast::<AtomicU32>()) };
        let observed: u32 = atomic.load(Ordering::Acquire);
        Self::SharedFutex {
            signal_word_addr: signal_word as usize,
            observed,
        }
    }

    fn wait(&mut self) -> Result<(), ErrorKind> {
        match self {
            Self::SharedFutex {
                signal_word_addr,
                observed,
            } => wait_for_signal(*signal_word_addr as *mut u32, observed),
            Self::Channel(receiver) => receiver.recv().map_err(|_| ErrorKind::BrokenPipe),
        }
    }
}

#[derive(Clone)]
pub struct DirectCqWriter {
    shared_ring: Arc<SharedRing>,
    next_slot: Arc<AtomicU32>,
    cq_notify: CqNotifyHandle,
}

impl DirectCqWriter {
    pub fn new(shared_ring: Arc<SharedRing>, cq_notify: CqNotifyHandle) -> Self {
        Self {
            shared_ring,
            next_slot: Arc::new(AtomicU32::new(0)),
            cq_notify,
        }
    }

    pub fn write_message(&self, user_data: u64, message: &Message) -> Result<(), ErrorKind> {
        let slot_id: u32 =
            self.next_slot.fetch_add(1, Ordering::Relaxed) % (DATA_SLOT_COUNT as u32);
        let slot_offset: usize = DATA_OFFSET + (slot_id as usize) * DATA_SLOT_SIZE;
        let bytes: [u8; core::mem::size_of::<Message>()] = message.clone().to_bytes();
        let write_len: usize = bytes.len().min(DATA_SLOT_SIZE);
        self.shared_ring
            .write_bytes(slot_offset, &bytes[..write_len])
            .map_err(|e| {
                error!("DirectCqWriter::write_message(): failed writing response payload: {e:?}");
                ErrorKind::InvalidData
            })?;

        let mut cqe: CqEntry = CqEntry::new(user_data, write_len as i64);
        cqe.buffer_id = slot_id;
        self.post_cqe(cqe)
    }

    pub fn write_fixed(&self, transfer: &FixedBufferTransfer) -> Result<(), ErrorKind> {
        let source_tid_raw: i32 = transfer.source_tid().into();
        let mut cqe: CqEntry = CqEntry::new(source_tid_raw as u64, i64::from(transfer.data_len()));
        cqe.flags = CqeFlags::BUFFER.0;
        if transfer.is_completion_batch() {
            cqe.flags |= CqeFlags::BATCH.0;
        }
        cqe.buffer_id = transfer.buffer_id();
        self.post_cqe(cqe)
    }

    pub fn write_nop(&self, user_data: u64) -> Result<(), ErrorKind> {
        self.post_cqe(CqEntry::new(user_data, 0))
    }

    fn post_cqe(&self, cqe: CqEntry) -> Result<(), ErrorKind> {
        let cq_head: u32 = self
            .shared_ring
            .read_copy(CTRL_CQ_HEAD)
            .map_err(|_| ErrorKind::InvalidData)?;
        let cq_tail: u32 = self
            .shared_ring
            .read_copy(CTRL_CQ_TAIL)
            .map_err(|_| ErrorKind::InvalidData)?;
        let cq_mask: u32 = self
            .shared_ring
            .read_copy(CTRL_CQ_MASK)
            .map_err(|_| ErrorKind::InvalidData)?;
        let cq_flags_raw: u32 = self
            .shared_ring
            .read_copy(CTRL_CQ_FLAGS)
            .map_err(|_| ErrorKind::InvalidData)?;

        let cq_idx: u32 = cq_tail & cq_mask;
        let cqe_offset: usize = CQ_OFFSET + (cq_idx as usize) * core::mem::size_of::<CqEntry>();
        self.shared_ring
            .write_copy(cqe_offset, cqe)
            .map_err(|_| ErrorKind::InvalidData)?;
        self.shared_ring
            .write_copy(CTRL_CQ_TAIL, cq_tail.wrapping_add(1))
            .map_err(|_| ErrorKind::InvalidData)?;

        let was_empty: bool = cq_head == cq_tail;
        if was_empty && CqFlags(cq_flags_raw) == CqFlags::NOTIFY_ME {
            self.cq_notify.notify_guest()?;
        }

        Ok(())
    }
}

fn read_message(shared_ring: &SharedRing, gpa: u64) -> Result<Message, ErrorKind> {
    let ptr: *mut u8 = shared_ring
        .ring_ptr_from_gpa(gpa, core::mem::size_of::<Message>())
        .map_err(|e| {
            error!("read_message(): invalid ring GPA for message payload (error={e:?})");
            ErrorKind::InvalidData
        })?;
    let mut bytes: [u8; core::mem::size_of::<Message>()] = [0u8; core::mem::size_of::<Message>()];
    // SAFETY: `ptr` was bounds-checked against the shared ring mapping and does not overlap `bytes`.
    unsafe { core::ptr::copy_nonoverlapping(ptr.cast_const(), bytes.as_mut_ptr(), bytes.len()) };
    Message::try_from_bytes(bytes).map_err(|e| {
        error!("read_message(): failed to decode message payload from ring (error={e:?})");
        ErrorKind::InvalidData
    })
}

fn drain_sq(
    uvm_id: UserVmIdentifier,
    shared_ring: &SharedRing,
    events_tx: &Sender<UserVmEvent>,
    cq_writer: &DirectCqWriter,
) -> Result<u32, ErrorKind> {
    let head: u32 = shared_ring
        .read_copy(CTRL_SQ_HEAD)
        .map_err(|_| ErrorKind::InvalidData)?;
    let tail: u32 = shared_ring
        .read_copy(CTRL_SQ_TAIL)
        .map_err(|_| ErrorKind::InvalidData)?;
    let mask: u32 = shared_ring
        .read_copy(CTRL_SQ_MASK)
        .map_err(|_| ErrorKind::InvalidData)?;

    let mut current_head: u32 = head;
    let mut drained: u32 = 0;
    while current_head != tail {
        let idx: u32 = current_head & mask;
        let sqe_offset: usize = SQ_OFFSET + (idx as usize) * core::mem::size_of::<SqEntry>();
        let sqe: SqEntry = shared_ring
            .read_copy(sqe_offset)
            .map_err(|_| ErrorKind::InvalidData)?;

        match SqeOpcode::from_u16(sqe.opcode) {
            Some(SqeOpcode::IkcMessage) => {
                let message: Message = read_message(shared_ring, sqe.addr)?;
                if events_tx
                    .blocking_send(UserVmEvent::Transfer {
                        uvm_id,
                        transfer: IkcFrame::Message(message),
                        user_data: Some(sqe.user_data),
                    })
                    .is_err()
                {
                    debug!("drain_sq(): dispatcher dropped receiver for VM {uvm_id}");
                    return Ok(drained);
                }
            }
            Some(SqeOpcode::BulkData) if sqe.is_fixed_buf() => {
                let transfer: FixedBufferTransfer = FixedBufferTransfer::new(
                    ProcessIdentifier::from(parse_inline_i32(&sqe, 0)),
                    ThreadIdentifier::from(parse_inline_i32(&sqe, 4)),
                    ProcessIdentifier::from(parse_inline_i32(&sqe, 8)),
                    ThreadIdentifier::from(parse_inline_i32(&sqe, 12)),
                    u32::try_from(sqe.addr).map_err(|_| ErrorKind::InvalidData)?,
                    sqe.len,
                );
                if events_tx
                    .blocking_send(UserVmEvent::Transfer {
                        uvm_id,
                        transfer: IkcFrame::Fixed(transfer),
                        user_data: None,
                    })
                    .is_err()
                {
                    debug!("drain_sq(): dispatcher dropped receiver for VM {uvm_id}");
                    return Ok(drained);
                }
            }
            Some(SqeOpcode::BulkData) => {
                error!(
                    "drain_sq(): encountered non-fixed bulk SQE on direct ring path \
                     (uvm_id={uvm_id}, addr={:#x}, len={})",
                    sqe.addr, sqe.len
                );
                return Err(ErrorKind::InvalidData);
            }
            Some(SqeOpcode::Nop) => {
                cq_writer.write_nop(sqe.user_data)?;
            }
            Some(other) => {
                warn!("drain_sq(): unsupported SQE opcode {other:?} on direct ring path");
            }
            None => {
                warn!("drain_sq(): unknown SQE opcode {:#06x}", sqe.opcode);
            }
        }

        current_head = current_head.wrapping_add(1);
        drained += 1;
    }

    if drained > 0 {
        shared_ring
            .write_copy(CTRL_SQ_HEAD, current_head)
            .map_err(|_| ErrorKind::InvalidData)?;
        trace!("drain_sq(): drained {drained} SQEs for VM {uvm_id}");
    }

    Ok(drained)
}

fn drain_sq_with_adaptive_polling(
    uvm_id: UserVmIdentifier,
    shared_ring: &SharedRing,
    stop: &AtomicBool,
    events_tx: &Sender<UserVmEvent>,
    cq_writer: &DirectCqWriter,
) -> Result<(), ErrorKind> {
    let poll_spin_iters: u32 = ::config::microvm::RING_POLL_SPIN_ITERS;
    let mut poll_budget: u32 = 0;
    let mut sq_wakeup_suppressed: bool = false;

    loop {
        let drained: u32 = drain_sq(uvm_id, shared_ring, events_tx, cq_writer)?;
        if drained > 0 {
            if poll_spin_iters == 0 {
                return Ok(());
            }

            if !sq_wakeup_suppressed {
                trace!(
                    "drain_sq_with_adaptive_polling(): entering SQ poll window \
                     (uvm_id={uvm_id}, spins={poll_spin_iters})"
                );
                set_sq_flags(shared_ring, SqFlags::NONE)?;
                sq_wakeup_suppressed = true;
            }
            poll_budget = poll_spin_iters;
            continue;
        }

        if !sq_wakeup_suppressed {
            return Ok(());
        }

        if stop.load(Ordering::Acquire) {
            break;
        }

        if poll_budget == 0 {
            trace!(
                "drain_sq_with_adaptive_polling(): re-arming SQ wakeup \
                 (uvm_id={uvm_id})"
            );
            set_sq_flags(shared_ring, SqFlags::NEED_WAKEUP)?;
            fence(Ordering::SeqCst);

            // Re-check the SQ after re-arming the doorbell to avoid sleeping
            // through work that arrived while wakeups were suppressed.
            let raced: u32 = drain_sq(uvm_id, shared_ring, events_tx, cq_writer)?;
            if raced > 0 {
                trace!(
                    "drain_sq_with_adaptive_polling(): observed SQ work after re-arm \
                     (uvm_id={uvm_id}, drained={raced})"
                );
                set_sq_flags(shared_ring, SqFlags::NONE)?;
                poll_budget = poll_spin_iters;
                continue;
            }

            return Ok(());
        }

        poll_budget -= 1;
        hint::spin_loop();
    }

    if sq_wakeup_suppressed {
        set_sq_flags(shared_ring, SqFlags::NEED_WAKEUP)?;
    }

    Ok(())
}

fn run_sq_worker(
    uvm_id: UserVmIdentifier,
    shared_ring: Arc<SharedRing>,
    stop: Arc<AtomicBool>,
    events_tx: Sender<UserVmEvent>,
    cq_writer: Arc<DirectCqWriter>,
    mut wait_strategy: SqWaitStrategy,
) {
    if let Err(kind) =
        drain_sq_with_adaptive_polling(
            uvm_id,
            &shared_ring,
            stop.as_ref(),
            &events_tx,
            &cq_writer,
        )
    {
        let _ = events_tx.blocking_send(UserVmEvent::ConnectionError { uvm_id, kind });
        return;
    }

    while !stop.load(Ordering::Acquire) {
        if let Err(kind) = wait_strategy.wait() {
            let _ = events_tx.blocking_send(UserVmEvent::ConnectionError { uvm_id, kind });
            break;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        if let Err(kind) = drain_sq_with_adaptive_polling(
            uvm_id,
            &shared_ring,
            stop.as_ref(),
            &events_tx,
            &cq_writer,
        ) {
            let _ = events_tx.blocking_send(UserVmEvent::ConnectionError { uvm_id, kind });
            break;
        }
    }

    trace!("run_sq_worker(): exiting for VM {uvm_id}");
}

pub fn spawn_sq_worker(
    uvm_id: UserVmIdentifier,
    shared_ring: Arc<SharedRing>,
    stop: Arc<AtomicBool>,
    events_tx: Sender<UserVmEvent>,
    cq_writer: Arc<DirectCqWriter>,
    use_socket_doorbell: bool,
) -> Result<SqWakeHandle, std::io::Error> {
    let thread_name: String = format!("ring-sq-{uvm_id}");
    let (wait_strategy, wake_handle): (SqWaitStrategy, SqWakeHandle) = if use_socket_doorbell {
        let (sender, receiver) = sync_channel::<()>(1);
        (SqWaitStrategy::Channel(receiver), SqWakeHandle::Channel(sender))
    } else {
        (
            SqWaitStrategy::new_shared_futex(shared_ring.as_ref()),
            SqWakeHandle::SharedFutex(shared_ring.clone()),
        )
    };
    thread::Builder::new().name(thread_name).spawn(move || {
        run_sq_worker(uvm_id, shared_ring, stop, events_tx, cq_writer, wait_strategy);
    })?;
    Ok(wake_handle)
}

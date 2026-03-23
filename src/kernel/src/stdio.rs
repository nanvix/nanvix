// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(all(feature = "microvm", feature = "ring-buffer")))]
use crate::hal::platform;
use crate::{
    PERF_IKC_MESSAGES_RECEIVED,
    PERF_IKC_MESSAGES_SENT,
};
use core::{
    mem,
    sync::atomic::Ordering,
};
#[cfg(all(feature = "microvm", feature = "ring-buffer"))]
use nvx_ring::{
    CqeFlags,
    SqeFlags,
};
#[cfg(not(all(feature = "microvm", feature = "ring-buffer")))]
use sys::ipc::DataChunkHeader;
#[cfg(not(all(feature = "microvm", feature = "ring-buffer")))]
use sys::ipc::VmBusMessage;
use sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes an inter-kernel communication message to the kernel's standard output.
///
/// # Parameters
///
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
///
/// # Description
///
/// Writes an inter-kernel communication message to the kernel's standard output.
///
/// When running on microvm, this function submits the message as a ring buffer SQE
/// instead of using the legacy PIO vmbus path, reducing VM exits.
///
/// # Parameters
///
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn write(message: Message) -> Result<(), Error> {
    // Checks if message type is not supported.
    if { message.message_type } != MessageType::Ikc {
        let reason: &str = "unsupported message type";
        error!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();

    cfg_if::cfg_if! {
        if #[cfg(all(feature = "microvm", feature = "ring-buffer"))] {
            // Submit via ring buffer: encode the message as an IkcMessage SQE.
            let msg_addr: u64 = crate::ring::write_data(&bytes)?;
            let sqe: ::nvx_ring::SqEntry = ::nvx_ring::SqEntry::new_ikc_message(
                msg_addr,
                mem::size_of::<Message>() as u32,
                0, // user_data — not used for legacy passthrough.
            );
            crate::ring::submit(sqe);
        } else {
            // Legacy path: build vmbus message wrapping the message address.
            let vmbus_msg: VmBusMessage =
                VmBusMessage::new(mem::size_of::<Message>() as u32, true, &bytes as *const u8 as u32);

            // Write vmbus message to the kernel's standard output.
            // SAFETY: The standard output is present, initialized and thread-safe to write.
            unsafe {
                // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
                platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);
            }
        }
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

#[cfg(all(feature = "microvm", feature = "ring-buffer"))]
fn encode_bulk_metadata(
    sqe: &mut ::nvx_ring::SqEntry,
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
) {
    let pid_bytes: [u8; 4] = (i32::from(source_pid)).to_le_bytes();
    let tid_bytes: [u8; 4] = (i32::from(source_tid)).to_le_bytes();
    let dpid_bytes: [u8; 4] = (i32::from(destination_pid)).to_le_bytes();
    let dtid_bytes: [u8; 4] = (i32::from(destination_tid)).to_le_bytes();
    sqe.inline_data[0..4].copy_from_slice(&pid_bytes);
    sqe.inline_data[4..8].copy_from_slice(&tid_bytes);
    sqe.inline_data[8..12].copy_from_slice(&dpid_bytes);
    sqe.inline_data[12..16].copy_from_slice(&dtid_bytes);
}

///
/// # Description
///
/// Reads a message from the kernel's standard input. If the message is a
/// [`MessageType::PullResponse`] notification, the bulk pull completion is handled
/// internally (waking the sleeping pull thread) and `Ok(None)` is returned so that the caller
/// retries transparently.
///
/// # Returns
///
/// Upon success, this function either returns the message read or `None` if there are no more
/// messages (or if the message was an internal bulk-transfer completion).  Upon failure, an error
/// is returned instead.
///
pub fn read() -> Result<Option<Message>, Error> {
    const NBYTES: usize = core::mem::size_of::<Message>();
    let mut message: [u8; NBYTES] = [0; NBYTES];

    cfg_if::cfg_if! {
        if #[cfg(all(feature = "microvm", feature = "ring-buffer"))] {
            // Ring buffer CQ path: poll for a CQE from the host.
            let cqe: ::nvx_ring::CqEntry = match crate::ring::try_poll_or_enable_notification() {
                Some(cqe) => cqe,
                None => return Ok(None),
            };

            if (cqe.flags & CqeFlags::BUFFER.0) != 0 {
                let caller_tid: ThreadIdentifier = match i32::try_from(cqe.user_data) {
                    Ok(raw_tid) => ThreadIdentifier::from(raw_tid),
                    Err(_) => {
                        let reason: &str = "fixed-buffer CQE carried invalid thread identifier";
                        error!("{reason} (user_data={})", cqe.user_data);
                        return Err(Error::new(ErrorCode::InvalidMessage, reason));
                    },
                };

                if cqe.result < 0 {
                    let reason: &str = "fixed-buffer CQE carried negative length";
                    error!("{reason} (caller_tid={caller_tid:?}, result={})", cqe.result);
                    return Err(Error::new(ErrorCode::InvalidMessage, reason));
                }

                if !crate::ipc::fixed_pull::complete(
                    caller_tid,
                    cqe.buffer_id,
                    cqe.result as usize,
                    (cqe.flags & CqeFlags::MORE.0) != 0,
                    (cqe.flags & CqeFlags::BATCH.0) != 0,
                ) {
                    warn!(
                        "read(): fixed-buffer completion had no matching pending pull \
                         (caller_tid={caller_tid:?}, buffer_id={})",
                        cqe.buffer_id
                    );
                }

                return Ok(None);
            }

            // Read the response Message from the data slot referenced by the CQE.
            let read_len: usize = (cqe.result as usize).min(NBYTES);
            // SAFETY: The data slot is in the ring buffer region which is identity-mapped.
            unsafe {
                let slot_ptr: *const u8 = crate::ring::data_slot_ptr(cqe.buffer_id);
                core::ptr::copy_nonoverlapping(slot_ptr, message.as_mut_ptr(), read_len);
            }
        }
        else if #[cfg(feature = "hyperlight")] {
            // Read credits register.
            let credits: u64 = unsafe {
                crate::hal::platform::hyperlight::peb::ProcessEnvironmentBlock::get_credits()?
            };

            // No message available.
            if credits == 0 {
                return Ok(None);
            }

            // Build vmbus message wrapping the message buffer address.
            let vmbus_msg: VmBusMessage =
                VmBusMessage::new(NBYTES as u32, true, &mut message as *mut u8 as u32);

            // Read message from the kernel's standard input via the vmbus message.
            // SAFETY: The standard input is present, initialized and thread-safe to read.
            unsafe {
                // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
                platform::vmbus_read(&vmbus_msg as *const VmBusMessage as *mut u8);
            };
        }
        else if #[cfg(feature = "microvm")] {
            // Legacy microvm path: credit register + vmbus read.
            let credits: u32 = unsafe {
                core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as *const u32)
            };

            if credits == 0 {
                return Ok(None);
            }

            let vmbus_msg: VmBusMessage =
                VmBusMessage::new(NBYTES as u32, true, &mut message as *mut u8 as u32);

            // SAFETY: The standard input is present, initialized and thread-safe to read.
            unsafe {
                // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
                platform::vmbus_read(&vmbus_msg as *const VmBusMessage as *mut u8);
            };
        }
    }

    PERF_IKC_MESSAGES_RECEIVED.fetch_add(1, Ordering::Relaxed);

    // Convert message to Message struct.
    match Message::try_from_bytes(message) {
        Ok(message) => {
            // Handle data chunk transfer completion notifications internally.  The memory thread
            // already wrote the bulk payload into the caller's buffer; we just need to wake the
            // sleeping pull thread.
            if { message.message_type } == MessageType::PullResponse {
                trace!("read(): received PullResponse, waking sleeping pull thread");
                if !crate::ipc::bulk_pull::complete(&message) {
                    warn!(
                        "read(): PullResponse had no matching pending pull (thread may have been \
                         cleaned up)"
                    );
                }
                return Ok(None);
            }

            Ok(Some(message))
        },
        // No message available.
        Err(e) if e.code == ErrorCode::NoMessageAvailable => Ok(None),
        Err(e) => {
            warn!("{e:?}");
            Err(e)
        },
    }
}

///
/// # Description
///
/// Writes a data chunk transfer envelope to the kernel's standard output. This sends the
/// [`DataChunkHeader`] metadata to the VMM via the vmbus, which then reads the bulk payload
/// directly from guest memory. This function is only available for the microvm machine type.
///
/// # Parameters
///
/// - `source_pid`: Process identifier of the source (sender).
/// - `source_tid`: Thread identifier of the source (sender).
/// - `destination_pid`: Process identifier of the destination (receiver).
/// - `destination_tid`: Thread identifier of the destination (receiver).
/// - `buffer_addr`: Guest physical address of the bulk data buffer.
/// - `data_len`: Number of bytes in the bulk payload.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
#[cfg_attr(all(feature = "microvm", feature = "ring-buffer"), allow(dead_code))]
pub fn write_bulk(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer_addr: u32,
    data_len: u32,
) -> Result<(), Error> {
    cfg_if::cfg_if! {
        if #[cfg(all(feature = "microvm", feature = "ring-buffer"))] {
            // Submit via ring buffer: encode the bulk transfer as a BulkData SQE.
            // Pack source/dest identifiers into the inline_data field.
            let mut sqe: ::nvx_ring::SqEntry = ::nvx_ring::SqEntry::zeroed();
            sqe.opcode = ::nvx_ring::SqeOpcode::BulkData as u16;
            sqe.addr = buffer_addr as u64;
            sqe.len = data_len;
            encode_bulk_metadata(&mut sqe, source_pid, source_tid, destination_pid, destination_tid);
            crate::ring::submit(sqe);
        } else {
            // Legacy path: build data chunk transfer header in the caller's stack.
            let header: DataChunkHeader = DataChunkHeader::new(
                source_pid,
                source_tid,
                destination_pid,
                destination_tid,
                buffer_addr,
                data_len,
            );

            // Build a vmbus message referencing the header.
            let vmbus_msg: VmBusMessage =
                VmBusMessage::new(data_len, false, &header as *const DataChunkHeader as u32);

            // Write vmbus message to the kernel's standard output.
            // SAFETY: The standard output is present, initialized and thread-safe to write.
            unsafe {
                // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
                platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);
            }
        }
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

#[cfg(all(feature = "microvm", feature = "ring-buffer"))]
pub fn write_fixed_bulk(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer_id: u32,
    data_len: u32,
) -> Result<(), Error> {
    if (data_len as usize) > ::nvx_ring::FIXED_BUF_SIZE {
        let reason: &str = "fixed-buffer transfer exceeds fixed buffer size";
        error!(
            "{reason} (buffer_id={buffer_id}, data_len={data_len}, max={})",
            ::nvx_ring::FIXED_BUF_SIZE
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let mut sqe: ::nvx_ring::SqEntry = ::nvx_ring::SqEntry::zeroed();
    sqe.opcode = ::nvx_ring::SqeOpcode::BulkData as u16;
    sqe.flags = SqeFlags::FIXED_BUF.0;
    sqe.addr = u64::from(buffer_id);
    sqe.len = data_len;
    encode_bulk_metadata(&mut sqe, source_pid, source_tid, destination_pid, destination_tid);
    crate::ring::submit(sqe);

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

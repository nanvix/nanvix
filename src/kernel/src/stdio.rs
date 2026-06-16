// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::platform,
    PERF_IKC_MESSAGES_RECEIVED,
    PERF_IKC_MESSAGES_SENT,
};
use ::core::{
    mem,
    sync::atomic::Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        DataChunkHeader,
        Message,
        MessageType,
        VmBusMessage,
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
pub fn write(message: Message) -> Result<(), Error> {
    // Checks if message type is not supported.
    if { message.message_type } != MessageType::Ikc {
        let reason: &str = "unsupported message type";
        error!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();

    // Build vmbus message wrapping the message address.
    let vmbus_msg: VmBusMessage =
        VmBusMessage::new(mem::size_of::<Message>() as u32, true, &bytes as *const u8 as u32);

    // Write vmbus message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
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
        if #[cfg(feature = "microvm")] {
            // Read credits register.
            let credits: u32 = unsafe {
                core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as *const u32)
            };
        }
    }

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
pub fn write_bulk(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer_addr: u32,
    data_len: u32,
) -> Result<(), Error> {
    // Build data chunk transfer header in the caller's stack.
    let header: DataChunkHeader = DataChunkHeader::new(
        source_pid,
        source_tid,
        destination_pid,
        destination_tid,
        buffer_addr,
        data_len,
    );

    // Build a vmbus message referencing the header. The vmbus message signals a data chunk transfer by
    // setting `is_ikc` to `false` and `size` to the bulk payload length.
    let vmbus_msg: VmBusMessage =
        VmBusMessage::new(data_len, false, &header as *const DataChunkHeader as u32);

    // Write vmbus message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

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
        GuestSgBulkHeader,
        GuestSgBulkKind,
        GuestSgSegment,
        Message,
        MessageType,
        SegmentCount,
        VmBusMessage,
        VmBusMessageKind,
    },
    mm::{
        Address,
        VirtualAddress,
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
    let message_addr: u32 =
        VirtualAddress::try_from_ptr(bytes.as_ptr(), "message address exceeds u32")?
            .into_raw_value() as u32;

    // Build vmbus message wrapping the message address.
    let vmbus_msg: VmBusMessage =
        VmBusMessage::new(mem::size_of::<Message>() as u32, VmBusMessageKind::Ikc, message_addr);

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
    let message_addr: u32 =
        VirtualAddress::try_from_ptr(message.as_mut_ptr(), "message buffer address exceeds u32")?
            .into_raw_value() as u32;
    let vmbus_msg: VmBusMessage =
        VmBusMessage::new(NBYTES as u32, VmBusMessageKind::Ikc, message_addr);

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
/// [`GuestSgBulkHeader`] metadata to the VMM via the vmbus. This function is only available for
/// the microvm machine type.
///
/// # Parameters
///
/// - `source_pid`: Process identifier of the source (sender).
/// - `source_tid`: Thread identifier of the source (sender).
/// - `destination_pid`: Process identifier of the destination (receiver).
/// - `destination_tid`: Thread identifier of the destination (receiver).
/// - `kind`: Type of scatter/gather transfer.
/// - `segments`: Scatter/gather segment list. The first segment is embedded in the header and the
///   remaining segments are chained through guest memory addresses.
/// - `data_len`: Number of bytes in the bulk payload.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn write_bulk(
    source: (ProcessIdentifier, ThreadIdentifier),
    destination: (ProcessIdentifier, ThreadIdentifier),
    kind: GuestSgBulkKind,
    segments: &[GuestSgSegment],
    data_len: u32,
    tag: u32,
) -> Result<(), Error> {
    let (source_pid, source_tid): (ProcessIdentifier, ThreadIdentifier) = source;
    if segments.is_empty() {
        let reason: &str = "scatter/gather transfer has no segments";
        error!("{reason} (source_pid={source_pid:?}, source_tid={source_tid:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    let segment_count: SegmentCount = SegmentCount::try_from(segments.len()).inspect_err(|_| {
        error!(
            "too many scatter/gather segments (source_pid={source_pid:?}, \
             source_tid={source_tid:?}, segments={})",
            segments.len()
        );
    })?;

    // Build scatter/gather transfer header in the caller's stack. The descriptor chain (the `next`
    // links between segments) is wired up by the ipc subsystem that built `segments`.
    let header: GuestSgBulkHeader = GuestSgBulkHeader::new(
        source,
        destination,
        kind,
        segment_count,
        data_len,
        tag,
        segments[0],
    );
    let reason: &str = "scatter/gather header address exceeds u32";
    let header_addr: u32 = VirtualAddress::try_from_ptr(&header as *const GuestSgBulkHeader, reason)
        .inspect_err(|_| {
            let header_addr: usize = &header as *const GuestSgBulkHeader as usize;
            error!(
                "{reason} (source_pid={source_pid:?}, source_tid={source_tid:?}, \
                 header_addr={header_addr:#x})"
            );
        })?
        .into_raw_value() as u32;

    // Build a vmbus message referencing the scatter/gather header.
    let vmbus_msg: VmBusMessage =
        VmBusMessage::new(data_len, VmBusMessageKind::GuestSgBulk, header_addr);

    // Write vmbus message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    build_error,
    fat32_to_error_code,
};
use ::arch::mem::PAGE_SIZE;
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    unistd::message::{
        PartialReadRequest,
        PartialReadResponse,
        PartialWriteRequest,
        PartialWriteResponse,
        ReadRequest,
        ReadResponse,
        WriteRequest,
        WriteResponse,
    },
    SystemCallMessage,
};
//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of bytes transferred in a single read/write bulk operation.
/// Must be at least as large as the page-aligned chunk size used by the syscall layer.
const MAX_BULK_TRANSFER_SIZE: usize = PAGE_SIZE;

/// Static buffer used for bulk read/write data transfers.
/// Safety: vfsd processes one request at a time (single-threaded message loop),
/// so there is no concurrent access to this buffer.
static mut BULK_BUFFER: [u8; MAX_BULK_TRANSFER_SIZE] = [0u8; MAX_BULK_TRANSFER_SIZE];

//==================================================================================================
// Read/Write Handlers (with push/pull bulk data transfer)
//==================================================================================================

pub(crate) fn handle_read(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Message {
    let req: ReadRequest = ReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;

    // Cap the read to the maximum bulk transfer size.
    let buf_size: usize = if count > MAX_BULK_TRANSFER_SIZE {
        MAX_BULK_TRANSFER_SIZE
    } else {
        count
    };

    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    match ::vfs::fd::vfs_read(fd, buf) {
        Ok(n) => {
            let n: usize = n as usize;

            // Push the data to the caller.
            if let Err(e) = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &buf[..n]) {
                ::syslog::error!("handle_read(): push failed (error={:?})", e);
                return build_error(source_tid, ErrorCode::IoErr);
            }

            ReadResponse::build(
                source_tid,
                n as i32,
                [0u8; ReadResponse::BUFFER_SIZE],
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        Err(e) => {
            // The client is blocked on __kcall_pull — push an empty buffer to unblock it
            // before sending the error response, otherwise the client deadlocks.
            if let Err(push_err) = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[]) {
                ::syslog::error!("handle_read(): unblock push failed (error={:?})", push_err);
            }
            build_error(source_tid, fat32_to_error_code(&e))
        },
    }
}

pub(crate) fn handle_write(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Message {
    let req: WriteRequest = WriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;

    // Cap to the maximum bulk transfer size.
    let buf_size: usize = if count > MAX_BULK_TRANSFER_SIZE {
        MAX_BULK_TRANSFER_SIZE
    } else {
        count
    };

    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    // Pull the data from the caller.
    match ::sys::kcall::ipc::__kcall_pull(source_pid, source_tid, buf) {
        Ok(pulled) => {
            let write_len: usize = if pulled < count { pulled } else { count };
            match ::vfs::fd::vfs_write(fd, &buf[..write_len]) {
                Ok(n) => WriteResponse::build(
                    source_tid,
                    n as i32,
                    ProcessIdentifier::VFSD,
                    MessageType::Ipc,
                ),
                Err(e) => build_error(source_tid, fat32_to_error_code(&e)),
            }
        },
        Err(e) => {
            ::syslog::error!("handle_write(): pull failed (error={:?})", e);
            build_error(source_tid, ErrorCode::IoErr)
        },
    }
}

//==================================================================================================
// Partial Read/Write Handlers (inline data in message payload)
//==================================================================================================

pub(crate) fn handle_pread(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: PartialReadRequest = PartialReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;
    let offset = req.offset;

    let max_inline: usize = PartialReadResponse::BUFFER_SIZE;
    let read_count: usize = if count > max_inline {
        max_inline
    } else {
        count
    };
    let mut buf = [0u8; PartialReadResponse::BUFFER_SIZE];

    match ::vfs::fd::vfs_pread(fd, &mut buf[..read_count], offset) {
        Ok(n) => PartialReadResponse::build(
            source,
            n as i32,
            buf,
            ProcessIdentifier::VFSD,
            MessageType::Ipc,
        ),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_pwrite(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: PartialWriteRequest = PartialWriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;
    let offset = req.offset;

    let max_inline: usize = PartialWriteRequest::BUFFER_SIZE;
    let write_count: usize = if count > max_inline {
        max_inline
    } else {
        count
    };

    match ::vfs::fd::vfs_pwrite(fd, &req.buffer[..write_count], offset) {
        Ok(n) => {
            PartialWriteResponse::build(source, n as i32, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

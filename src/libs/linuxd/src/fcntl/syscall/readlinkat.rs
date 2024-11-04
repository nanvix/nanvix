// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        ReadLinkAtRequest,
        ReadLinkAtResponse,
    },
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
        MessagePartitioner,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn readlinkat(dirfd: i32, path: &str, buf: &mut [u8]) -> i32 {
    // Send request.
    let status: i32 = readlinkat_request(dirfd, path, buf);
    if status != 0 {
        return status;
    }

    // Wait for response.
    readlinkat_response(buf)
}

fn readlinkat_request(dirfd: i32, path: &str, buf: &[u8]) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let request: ReadLinkAtRequest =
        match ReadLinkAtRequest::new(dirfd, path.to_string(), buf.len()) {
            Ok(request) => request,
            Err(e) => return e.code.into_errno(),
        };

    let requests: Vec<Message> = match request.into_parts(pid) {
        Ok(requests) => requests,
        Err(e) => return e.code.into_errno(),
    };

    for request in requests {
        match ::nvx::ipc::send(&request) {
            Ok(_) => (),
            Err(e) => return e.code.into_errno(),
        }
    }

    0
}

fn readlinkat_response(buf: &mut [u8]) -> i32 {
    let capacity: usize =
        ReadLinkAtResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let mut assembler: LinuxDaemonLongMessage = match LinuxDaemonLongMessage::new(capacity) {
        Ok(assembler) => assembler,
        Err(e) => return e.code.into_errno(),
    };

    loop {
        let response: Message = match ::nvx::ipc::recv() {
            Ok(response) => response,
            Err(e) => break e.code.into_errno(),
        };

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(e) => break e.into_errno(),
                Err(_) => break ErrorCode::InvalidMessage.into_errno(),
            }
        } else {
            // System call succeeded, parse response.
            match LinuxDaemonMessage::try_from_bytes(response.payload) {
                Ok(message) => match message.header {
                    LinuxDaemonMessageHeader::ReadLinkAtResponsePart => {
                        let part: LinuxDaemonMessagePart =
                            LinuxDaemonMessagePart::from_bytes(message.payload);

                        if let Err(e) = assembler.add_part(part) {
                            break e.code.into_errno();
                        }

                        if !assembler.is_complete() {
                            continue;
                        }

                        let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                        match ReadLinkAtResponse::from_parts(&parts) {
                            Ok(response) => {
                                assert!(response.buffer.len() <= buf.len());
                                buf[..response.buffer.len()].copy_from_slice(&response.buffer);
                                break response.buffer.len() as i32;
                            },
                            Err(_) => break ErrorCode::InvalidMessage.into_errno(),
                        }
                    },
                    _ => break ErrorCode::InvalidMessage.into_errno(),
                },
                Err(_) => break ErrorCode::InvalidMessage.into_errno(),
            }
        }
    }
}

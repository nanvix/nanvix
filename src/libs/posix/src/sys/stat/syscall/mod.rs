// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fstat;
mod fstatat;
mod futimens;
mod stat;
mod utimensat;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
        MessagePartitioner,
    },
    sys::{
        self,
        stat::message::FileStatAtResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::nvx::{
    ipc::Message,
    sys::error::ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use fstat::fstat;
pub use fstatat::fstatat;
pub use futimens::futimens;
pub use stat::stat;
pub use utimensat::utimensat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// This function waits for the response of the `fstat()` system call.
///
/// # Parameters
///
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
fn fstatat_response(buf: &mut sys::stat::stat) -> i32 {
    let capacity: usize = sys::stat::stat::SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

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
                    LinuxDaemonMessageHeader::FileStatAtResponsePart => {
                        let part: LinuxDaemonMessagePart =
                            LinuxDaemonMessagePart::from_bytes(message.payload);

                        if let Err(e) = assembler.add_part(part) {
                            break e.code.into_errno();
                        }

                        if !assembler.is_complete() {
                            continue;
                        }

                        let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                        match FileStatAtResponse::from_parts(&parts) {
                            Ok(response) => {
                                *buf = response.stat;
                                break 0;
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

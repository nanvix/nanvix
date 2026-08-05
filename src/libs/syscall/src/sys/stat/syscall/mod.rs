// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod chmod;
mod fchmod;
mod fchmodat;
mod fstat;
mod fstatat;
mod futimens;
mod lstat;
mod mkdir;
mod mkdirat;
mod stat;
mod umask;
mod utimensat;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        MessagePartitioner,
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    sys::stat::message::FileStatAtResponse,
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
};
use ::sysapi::sys_stat;

//==================================================================================================
// Exports
//==================================================================================================

pub use chmod::chmod;
pub use fchmod::fchmod;
pub use fchmodat::fchmodat;
pub use fstat::fstat;
pub use fstatat::fstatat;
pub use futimens::futimens;
pub use lstat::lstat;
pub use mkdir::mkdir;
pub use mkdirat::mkdirat;
pub use stat::stat;
pub use umask::umask;
pub use utimensat::utimensat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// This function waits for the response of the `fstatat()` system call.
///
/// # Returns
///
/// Upon successful completion, the file information is returned. Upon failure, an error is returned
/// instead.
///
fn fstatat_response(token: &RequestToken) -> Result<sys_stat::stat, Error> {
    let capacity: usize = sys_stat::stat::SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);

    let mut assembler: SystemCallLongMessage = SystemCallLongMessage::new(capacity)?;

    loop {
        let response: Message = crate::rpc::recv_response(token)?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
            ::syslog::warn!("fstatat(): failed (error={:?})", error_code);
            break Err(Error::new(error_code, "fstatat() failed"));
        } else {
            // System call succeeded, parse response.
            match SystemCallMessage::try_from_bytes(response.payload) {
                Ok(message) => match message.kind() {
                    SystemCallMessageKind::FileStatAtResponsePart => {
                        let part: SystemCallMessagePart =
                            SystemCallMessagePart::from_bytes(message.payload);

                        if let Err(e) = assembler.add_part(part) {
                            ::syslog::warn!("fstatat(): failed to add part to assembler");
                            break Err(e);
                        }

                        if !assembler.is_complete() {
                            continue;
                        }

                        let parts: Vec<SystemCallMessagePart> = assembler.take_parts();

                        let response: FileStatAtResponse = FileStatAtResponse::from_parts(&parts)?;
                        break Ok(response.stat);
                    },
                    _ => {
                        break Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "unexpected message header",
                        ))
                    },
                },
                _ => break Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
            }
        }
    }
}

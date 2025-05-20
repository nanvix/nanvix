// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fchmod;
mod fchmodat;
mod fstat;
mod fstatat;
mod futimens;
mod lstat;
mod mkdirat;
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
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use fchmod::fchmod;
pub use fchmodat::fchmodat;
pub use fstat::fstat;
pub use fstatat::fstatat;
pub use futimens::futimens;
pub use lstat::lstat;
pub use mkdirat::mkdirat;
pub use stat::stat;
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
fn fstatat_response() -> Result<sys::stat::stat, Error> {
    let capacity: usize = sys::stat::stat::SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let mut assembler: LinuxDaemonLongMessage = LinuxDaemonLongMessage::new(capacity)?;

    loop {
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
            ::syslog::error!("fstatat(): failed (error={:?})", error_code);
            break Err(Error::new(error_code, "fstatat() failed"));
        } else {
            // System call succeeded, parse response.
            match LinuxDaemonMessage::try_from_bytes(response.payload) {
                Ok(message) => match message.header {
                    LinuxDaemonMessageHeader::FileStatAtResponsePart => {
                        let part: LinuxDaemonMessagePart =
                            LinuxDaemonMessagePart::from_bytes(message.payload);

                        if let Err(e) = assembler.add_part(part) {
                            ::syslog::error!("fstatat(): failed to add part to assembler");
                            break Err(e);
                        }

                        if !assembler.is_complete() {
                            continue;
                        }

                        let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

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

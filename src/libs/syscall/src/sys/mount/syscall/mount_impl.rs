// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::mount::message::MountRequest,
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Mounts a filesystem at the given target path.
///
/// # Parameters
///
/// - `source`: Source device or path (may be empty for virtual filesystems).
/// - `target`: Target mount point in the guest VFS.
/// - `fstype`: Filesystem type string (e.g., "hostfs").
/// - `flags`: Mount flags (reserved, pass 0).
///
/// # Returns
///
/// Upon success, returns `Ok(())`. Otherwise, returns an error.
pub fn mount(source: &str, target: &str, fstype: &str, flags: u64) -> Result<(), Error> {
    ::syslog::trace!(
        "mount(): source={:?}, target={:?}, fstype={:?}, flags={}",
        source,
        target,
        fstype,
        flags
    );

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: MountRequest =
        MountRequest::new(source.to_string(), target.to_string(), fstype.to_string(), flags)?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    // Send request parts.
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "mount(): failed (source={:?}, target={:?}, fstype={:?}, error_code={:?})",
            source,
            target,
            fstype,
            { response.status }
        );
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "mount() failed")),
            Err(error) => {
                ::syslog::warn!("mount(): failed to parse error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "mount(): failed"))
            },
        }
    } else {
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        let header: SystemCallMessageKind = message.kind();
        if header != SystemCallMessageKind::HostMountResponse {
            return Err(Error::new(ErrorCode::InvalidMessage, "unexpected response header"));
        }
        Ok(())
    }
}

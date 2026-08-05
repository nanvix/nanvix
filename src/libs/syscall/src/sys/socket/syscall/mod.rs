// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod accept;
mod bind;
mod connect;
mod getpeername;
mod getsockname;
mod listen;
mod recv;
mod recvfrom;
mod send;
mod sendto;
mod shutdown;
mod socket;
mod socketpair;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    accept::accept,
    bind::bind,
    connect::connect,
    getpeername::getpeername,
    getsockname::getsockname,
    listen::listen,
    recv::recv,
    recvfrom::recvfrom,
    send::send,
    sendto::sendto,
    shutdown::shutdown,
    socket::socket,
    socketpair::socketpair,
};

//==================================================================================================
// Socket-slot registration helpers
//==================================================================================================

/// Registers a `networkd` socket endpoint as a flat descriptor slot in vfsd.
///
/// This is the second step of socket creation under the flat namespace: `networkd` owns the
/// endpoint (`remote_fd`) and remains the I/O backend, while vfsd allocates the application-visible
/// flat descriptor that routes to it. The returned flat descriptor is recorded in the resolution
/// cache (`route = Socket`, `backend_fd = remote_fd`) so socket I/O resolves to `remote_fd` without
/// a further `vfsd` round-trip. If the registration fails on either leg, the `networkd` endpoint is
/// closed so a failed creation never strands an endpoint.
pub(crate) fn register_socket_slot(
    remote_fd: ::sysapi::ffi::c_int,
) -> Result<::sysapi::ffi::c_int, ::sys::error::Error> {
    use crate::{
        unistd::message::{
            RegisterSocketRequest,
            RegisterSocketResponse,
        },
        SystemCallMessage,
        SystemCallMessageHeader,
    };
    use ::sys::{
        error::{
            Error,
            ErrorCode,
        },
        ipc::RequestToken,
    };

    let tid = match ::sys::kcall::pm::__kcall_gettid() {
        Ok(tid) => tid,
        Err(e) => {
            close_networkd_endpoint(remote_fd);
            return Err(e);
        },
    };
    let mut request = RegisterSocketRequest::build(
        tid,
        remote_fd,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = match crate::rpc::send_request(&mut request) {
        Ok(token) => token,
        Err(e) => {
            close_networkd_endpoint(remote_fd);
            return Err(e);
        },
    };
    let response = match crate::rpc::recv_response(&token) {
        Ok(response) => response,
        Err(e) => {
            close_networkd_endpoint(remote_fd);
            return Err(e);
        },
    };
    if response.status != 0 {
        close_networkd_endpoint(remote_fd);
        let error_code = ErrorCode::try_from(response.status).unwrap_or(ErrorCode::InvalidMessage);
        return Err(Error::new(error_code, "failed to register socket slot"));
    }
    let message = match SystemCallMessage::try_from_bytes(response.payload) {
        Ok(message) => message,
        Err(_) => {
            close_networkd_endpoint(remote_fd);
            return Err(Error::new(ErrorCode::InvalidMessage, "invalid response"));
        },
    };
    match message.header {
        SystemCallMessageHeader::RegisterSocketResponse => {
            let resp = RegisterSocketResponse::from_bytes(message.payload);
            // `RegisterSocketResponse` is `#[repr(C, packed)]`; read each field through a raw
            // pointer to avoid forming an unaligned reference.
            let fd: ::sysapi::ffi::c_int =
                unsafe { ::core::ptr::addr_of!(resp.fd).read_unaligned() };
            let epoch: u64 = unsafe { ::core::ptr::addr_of!(resp.epoch).read_unaligned() };
            crate::fdtable::record(fd, crate::fdtable::Route::Socket, remote_fd, epoch);
            Ok(fd)
        },
        _ => {
            close_networkd_endpoint(remote_fd);
            Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header"))
        },
    }
}

/// Closes a `networkd` socket endpoint directly, best-effort.
///
/// Used to roll back the `networkd` endpoint when binding its flat slot in vfsd fails, so a failed
/// socket creation does not strand an endpoint. The acknowledgement is drained but its status is
/// ignored: the creation has already failed and there is nothing to retry.
fn close_networkd_endpoint(remote_fd: ::sysapi::ffi::c_int) {
    use crate::unistd::message::CloseRequest;
    use ::sys::ipc::{
        MessageType,
        RequestToken,
    };

    let Ok(tid) = ::sys::kcall::pm::__kcall_gettid() else {
        return;
    };
    let mut request =
        CloseRequest::build(tid, remote_fd, crate::NETWORK_DESTINATION, MessageType::Ikc);
    if let Ok(token) = crate::rpc::send_request(&mut request) {
        let token: RequestToken = token;
        let _ = crate::rpc::recv_response(&token);
    }
}

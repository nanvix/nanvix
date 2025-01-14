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
mod send;
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
    send::send,
    shutdown::shutdown,
    socket::socket,
    socketpair::socketpair,
};

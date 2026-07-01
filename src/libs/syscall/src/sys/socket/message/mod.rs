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
    accept::{
        AcceptSocketRequest,
        AcceptSocketResponse,
    },
    bind::{
        BindSocketRequest,
        BindSocketResponse,
    },
    connect::{
        ConnectSocketRequest,
        ConnectSocketResponse,
    },
    getpeername::{
        GetPeerNameRequest,
        GetPeerNameResponse,
    },
    getsockname::{
        GetSockNameRequest,
        GetSockNameResponse,
    },
    listen::{
        ListenSocketRequest,
        ListenSocketResponse,
    },
    recv::{
        ReceiveSocketRequest,
        ReceiveSocketResponse,
    },
    recvfrom::{
        ReceiveFromSocketRequest,
        ReceiveFromSocketResponse,
    },
    send::{
        SendSocketRequest,
        SendSocketResponse,
    },
    sendto::{
        SendToSocketRequest,
        SendToSocketResponse,
    },
    shutdown::{
        ShutdownSocketRequest,
        ShutdownSocketResponse,
    },
    socket::{
        CreateSocketRequest,
        CreateSocketResponse,
    },
    socketpair::{
        CreateSocketPairRequest,
        CreateSocketPairResponse,
    },
};

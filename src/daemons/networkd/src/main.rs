// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

// The decoupled server transport is Linux-only (it depends on `syscomm`), so the whole binary is a
// no-op stub on other platforms.
#[cfg(target_os = "linux")]
mod args;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::args::Args;
#[cfg(target_os = "linux")]
use ::anyhow::Result;
#[cfg(target_os = "linux")]
use ::log::{
    error,
    info,
};
#[cfg(target_os = "linux")]
use ::net_backend::HostFilter;
#[cfg(target_os = "linux")]
use ::std::{
    env,
    str::FromStr,
};
#[cfg(target_os = "linux")]
use ::syscomm::{
    SocketListener,
    SocketStream,
    SocketType,
    UnboundSocket,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by the `RUST_LOG` environment variable if set).
#[cfg(target_os = "linux")]
const DEFAULT_LOG_LEVEL: &str = "error";

//==================================================================================================
// Implementations
//==================================================================================================

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = Args::parse(env::args().collect())?;
    ::syslog::init(args.log_to_file(), DEFAULT_LOG_LEVEL, args.log_directory(), None);

    info!("networkd: starting decoupled network daemon");

    // Bind and start listening for the single user VM this daemon serves.
    let user_vm_sockaddr: &str = args.user_vm_bind_sockaddr();
    let unbound_socket: UnboundSocket =
        UnboundSocket::new(SocketType::from_str(args.user_vm_bind_socket_type())?);
    let listener: SocketListener = match unbound_socket.bind(user_vm_sockaddr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!(
                "networkd: failed to bind user VM socket (address={user_vm_sockaddr}, error={e:?})"
            );
            ::anyhow::bail!("failed to bind user VM socket");
        },
    };
    info!("networkd: listening for the user VM on {user_vm_sockaddr:?}");

    // networkd serves exactly one user VM, so accept a single connection.
    let stream: SocketStream = match listener.accept().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("networkd: failed to accept user VM connection: {e:?}");
            ::anyhow::bail!("failed to accept user VM connection");
        },
    };
    info!("networkd: user VM connected");

    // Build the host egress policy derived from the command-line allow/block lists. With no lists,
    // `host_filter()` yields `HostFilter::AllowAll`, matching the standalone default.
    let host_filter: HostFilter = args.host_filter();
    info!("networkd: applying host egress policy {host_filter:?}");

    // Serve the connection until the user VM disconnects. The reactor owns the non-blocking
    // networking backend and multiplexes every host socket it opens on the guest's behalf through a
    // single epoll instance.
    if let Err(e) = ::networkd::reactor::run(host_filter, stream).await {
        error!("networkd: reactor terminated with an error: {e:?}");
        return Err(e);
    }

    // Fail-stop teardown: exit the process outright once the user VM session ends. This guarantees
    // the OS reclaims every host socket opened on the guest's behalf, even if a blocking backend
    // call (e.g. a parked `recvfrom`) would otherwise stall an orderly runtime shutdown. networkd
    // serves a single user VM, so there is nothing left to do once that VM is gone.
    info!("networkd: user VM session ended; shutting down");
    ::std::process::exit(0);
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("networkd: the decoupled network daemon is only supported on Linux");
    ::std::process::exit(1);
}

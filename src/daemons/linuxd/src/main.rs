// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::anyhow::Result;
use ::linuxd::{
    args,
    args::Args,
    syscalls::SyscallTable,
    LinuxDaemon,
};
use ::log::{
    error,
    info,
};
use ::std::{
    env,
    str::FromStr,
    sync::Arc,
};
use ::syscomm::{
    SocketListener,
    SocketType,
    UnboundSocket,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";

//==================================================================================================
// Private Functions
//==================================================================================================

/// Sanitizes a tenant identifier so it is safe to use as a file-name component.
///
/// Only ASCII letters, digits, '.', '_' and '-' are preserved. All other characters
/// are replaced with '_'.
fn sanitize_tenant_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

//==================================================================================================
// Implementations
//==================================================================================================

#[tokio::main]
pub async fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    let tenant_id: Option<String> = args.tenant_id().and_then(|id| {
        if id.is_empty() {
            None
        } else {
            Some(sanitize_tenant_id(id))
        }
    });
    ::syslog::init(args.log_to_file(), DEFAULT_LOG_LEVEL, args.log_file_dir(), tenant_id.clone());

    // Work-out the socket addresses.
    let control_plane_sockaddr: &str = args.control_plane_sockaddr();
    let user_vm_sockaddr: &str = args.user_vm_bind_sockaddr();

    // Deployed in an L2 VM?
    let in_l2: bool = args.l2();

    // Start listening for incoming connections from user VMs associated to this linuxd instance.

    let unbound_socket: UnboundSocket =
        UnboundSocket::new(SocketType::from_str(args.user_vm_bind_socket_type())?);
    let user_vm_listener: SocketListener = match unbound_socket.bind(user_vm_sockaddr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!(
                "failed to bind to user VM socket address (address={}, error={e:?})",
                user_vm_sockaddr
            );
            anyhow::bail!("failed to bind to user VM socket address");
        },
    };
    info!("Listening to user VMs on: {user_vm_sockaddr:?}");

    let linuxd: LinuxDaemon<()> = match LinuxDaemon::init(
        Arc::new(SyscallTable::new((), args.networking_enabled())?),
        tenant_id.as_deref().unwrap_or_default(),
        control_plane_sockaddr,
        args.control_plane_socket_type(),
        user_vm_listener,
        in_l2,
    ) {
        Ok(linuxd) => linuxd,
        Err(e) => panic!("failed to initialize process manager daemon (error={e:?})"),
    };

    let linuxd_ret = linuxd.run().await;

    // Do not panic here as we have already exited the loop. Instead, continue with clean-up.
    if linuxd_ret.is_err() {
        error!("error running procd (error={linuxd_ret:?})");
    }

    Ok(())
}

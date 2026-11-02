// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
    trace,
};
use ::std::net::{
    SocketAddr,
    TcpListener,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of ports to search when finding an available alternative port.
const PORT_RANGE_LENGTH: u16 = 1000;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks whether a TCP port is available for binding on the specified IPv4 address.
///
/// # Parameters
///
/// - `ipv4_addr`: IPv4 address to check (e.g., "127.0.0.1").
/// - `port`: TCP port number to check.
///
/// # Return Value
///
/// Returns `Ok(true)` when the port is available for binding; returns `Ok(false)` when the port
/// is already in use.
///
/// # Errors
///
/// Returns an error when `ipv4_addr` and `port` cannot be parsed into a valid socket address.
///
fn is_port_available(ipv4_addr: &str, port: u16) -> Result<bool> {
    let addr: String = format!("{ipv4_addr}:{port}");
    let socket_addr: SocketAddr = match addr.parse() {
        Ok(addr) => addr,
        Err(error) => {
            let reason: String = format!("failed to parse address (addr={addr}, error={error})");
            error!("is_port_available(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
    };

    // Attempt to bind to the address. If binding succeeds, the port is available.
    // The listener is dropped immediately after creation, releasing the port.
    match TcpListener::bind(socket_addr) {
        Ok(listener) => {
            drop(listener);
            trace!("is_port_available(): port available (addr={addr})");
            Ok(true)
        },
        Err(error) => {
            trace!("is_port_available(): port unavailable (addr={addr}, error={error})");
            Ok(false)
        },
    }
}

///
/// # Description
///
/// Finds an available TCP port within a range starting from the specified base port.
///
/// # Parameters
///
/// - `ipv4_addr`: IPv4 address to bind to (e.g., "127.0.0.1").
/// - `start_port`: First port number to check.
/// - `end_port`: Last port number to check (inclusive).
///
/// # Return Value
///
/// Returns the first available port found within the range; returns an error when no port is
/// available in the specified range.
///
fn find_available_port(ipv4_addr: &str, start_port: u16, end_port: u16) -> Result<u16> {
    if start_port > end_port {
        let reason: String =
            format!("invalid port range (start_port={start_port} > end_port={end_port})");
        error!("find_available_port(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    for port in start_port..=end_port {
        if is_port_available(ipv4_addr, port)? {
            debug!("find_available_port(): found available port (port={port})");
            return Ok(port);
        }
    }

    let reason: String =
        format!("no available port found in range {start_port}-{end_port} (ipv4_addr={ipv4_addr})");
    error!("find_available_port(): {reason}");
    Err(::anyhow::anyhow!(reason))
}

///
/// # Description
///
/// Resolves a usable TCP port for the HTTP server. If the configured port is available, it is
/// returned directly. Otherwise, an alternative port is searched within a range starting from
/// the configured port.
///
/// # Parameters
///
/// - `ipv4_addr`: IPv4 address where the HTTP server will bind.
/// - `configured_port`: Port number specified in the configuration file.
///
/// # Return Value
///
/// Returns the configured port when available, or an alternative port from the search range;
/// returns an error when no port is available.
///
/// # Note
///
/// There is a TOCTOU (time-of-check-time-of-use) race condition between finding an available
/// port and actually binding to it. Another process could bind to the port in between. This is
/// acceptable as the probability is low and the HTTP server startup will fail with a clear
/// error if this occurs.
///
pub fn resolve_http_port(ipv4_addr: &str, configured_port: u16) -> Result<u16> {
    if is_port_available(ipv4_addr, configured_port)? {
        trace!(
            "resolve_http_port(): configured port available (ipv4_addr={ipv4_addr}, \
             port={configured_port})"
        );
        return Ok(configured_port);
    }

    info!(
        "resolve_http_port(): configured port in use, searching for alternative \
         (ipv4_addr={ipv4_addr}, configured_port={configured_port})"
    );

    // Define port range based on configured port. Start from the next port since the configured
    // port was already checked and found unavailable.
    let start_port: u16 = configured_port.saturating_add(1);
    let end_port: u16 = configured_port.saturating_add(PORT_RANGE_LENGTH);

    let alternative_port: u16 = find_available_port(ipv4_addr, start_port, end_port)?;

    info!(
        "resolve_http_port(): using alternative port (ipv4_addr={ipv4_addr}, \
         configured_port={configured_port}, alternative_port={alternative_port})"
    );

    Ok(alternative_port)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;

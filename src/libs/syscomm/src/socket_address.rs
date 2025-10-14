// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Socket address that can be either TCP or Unix domain socket.
///
#[derive(Debug, Clone)]
pub enum SocketAddr {
    /// TCP socket address.
    Tcp(::std::net::SocketAddr),
    /// Unix socket address.
    Unix(::tokio::net::unix::SocketAddr),
}

impl SocketAddr {
    /// Maximum length of a string representation of a Unix socket address.
    pub const UNIX_SOCKADDR_MAX_LEN: usize = 108;

    /// Maximum length of a string representation of a TCP socket address.
    pub const TCP_SOCKADDR_MAX_LEN: usize = 21;
}

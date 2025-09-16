// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Re-exports
//==================================================================================================

pub use ::sysapi::netinet_in::{
    in6_addr,
    ip_option_names::{
        IP_HDRINCL,
        IP_MULTICAST_IF,
        IP_MULTICAST_LOOP,
        IP_MULTICAST_TTL,
        IP_TTL,
    },
    sockaddr_in,
    sockaddr_in6,
    sockopt_ipv6::IPV6_RECVHOPLIMIT,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod ipproto {
    pub use ::sysapi::netinet_in::sockopt_levels::{
        IPPROTO_ICMP,
        IPPROTO_IP,
        IPPROTO_IPV6,
        IPPROTO_TCP,
        IPPROTO_UDP,
    };
}

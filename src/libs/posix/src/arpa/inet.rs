// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts a 32-bit integer from host byte order to network byte order.
pub fn htonl(hostlong: u32) -> u32 {
    hostlong.to_be()
}

/// Converts a 16-bit integer from host byte order to network byte order.
pub fn htons(hostshort: u16) -> u16 {
    hostshort.to_be()
}

/// Converts a 32-bit integer from network byte order to host byte order.
pub fn ntohl(netlong: u32) -> u32 {
    u32::from_be(netlong)
}

/// Converts a 16-bit integer from network byte order to host byte order.
pub fn ntohs(netshort: u16) -> u16 {
    u16::from_be(netshort)
}

pub mod bindings {

    /// Converts a 32-bit integer from host byte order to network byte order.
    #[no_mangle]
    pub extern "C" fn htonl(hostlong: u32) -> u32 {
        super::htonl(hostlong)
    }

    /// Converts a 16-bit integer from host byte order to network byte order.
    #[no_mangle]
    pub extern "C" fn htons(hostshort: u16) -> u16 {
        super::htons(hostshort)
    }

    /// Converts a 32-bit integer from network byte order to host byte order.
    #[no_mangle]
    pub extern "C" fn ntohl(netlong: u32) -> u32 {
        super::ntohl(netlong)
    }

    /// Converts a 16-bit integer from network byte order to host byte order.
    #[no_mangle]
    pub extern "C" fn ntohs(netshort: u16) -> u16 {
        super::ntohs(netshort)
    }
}

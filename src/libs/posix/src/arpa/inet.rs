// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use core::mem;

//==================================================================================================
// C Interface
//==================================================================================================

/// C Bindings for `arpa/inet.h`.
pub mod bindings {

    #![allow(non_camel_case_types)]

    use super::*;

    /// Used for internet addresses.
    pub type in_addr_t = u32;

    /// Describes an internet address.
    #[repr(C, packed)]
    pub struct in_addr {
        pub s_addr: in_addr_t,
    }
    ::nvx::sys::static_assert_size!(in_addr, in_addr::_SIZE);

    impl in_addr {
        /// Size of this structure, used for static assertions.
        const _SIZE: usize = mem::size_of::<in_addr_t>(); // s_addr
    }
}

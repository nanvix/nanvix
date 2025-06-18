// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    netinet_in::{
        in_addr,
        in_addr_t,
    },
    sys_socket::socklen_t,
};

//===================================================================================================
// Functions
//===================================================================================================

unsafe extern "C" {

    pub unsafe fn inet_addr(cp: *const c_char) -> in_addr_t;
    pub unsafe fn inet_ntoa(in_addr: in_addr) -> *const c_char;
    pub unsafe fn inet_ntop(
        af: c_int,
        src: *const c_void,
        dst: *mut c_char,
        size: socklen_t,
    ) -> *const c_char;
}

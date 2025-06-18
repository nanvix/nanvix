// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

/// 32-Bit Foreign Function Interface
mod bits32 {
    // Equivalent to C’s signed char type.
    pub type c_schar = i8;
    // Equivalent to C’s char type.
    pub type c_char = i8;
    // Equivalent to C’s short type.
    pub type c_short = i16;
    // Equivalent to C’s signed int type.
    pub type c_int = i32;
    // Equivalent to C’s signed long type.
    pub type c_long = i32;
    // Equivalent to C’s signed long long type.
    pub type c_longlong = i64;
    // Equivalent to C’s unsigned char type.
    pub type c_uchar = u8;
    // Equivalent to C’s unsigned short type.
    pub type c_ushort = u16;
    // Equivalent to C’s unsigned int type.
    pub type c_uint = u32;
    // Equivalent to C’s unsigned long type.
    pub type c_ulong = u32;
    // Equivalent to C’s unsigned long long type.
    pub type c_ulonglong = u64;
    // Equivalent to C’s void pointer type.
    pub type c_void = core::ffi::c_void;
}

pub use bits32::*;

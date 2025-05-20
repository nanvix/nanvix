// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_char,
    sys::utsname::{
        utsname,
        UTSNAME_LENGTH,
    },
};
use ::alloc::ffi::CString;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Encodes a Rust string into an array of c-style characters.
///
/// # Parameters
///
/// - `string`: The Rust string to encode.
///
/// # Returns
///
/// If successful, the encoded string is returned. Otherwise, an error is returned instead.
///
fn encode_str<const N: usize>(string: &str) -> Result<[c_char; N], Error> {
    // Attempt to convert Rust string to C string and check for errors.
    let c_string = match CString::new(string) {
        // Success.
        Ok(s) => s,
        // Failure.
        Err(error) => {
            let reason: &str = "failed to convert string";
            ::syslog::error!("encode_str(): {} (error={:?})", reason, error);
            return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
        },
    };

    // Check if string is too long.
    if c_string.as_bytes_with_nul().len() > N {
        let reason: &str = "string is too long";
        ::syslog::error!("encode_str(): {} (string={:?})", reason, string);
        return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
    }

    let mut arr: [i8; N] = [0; N];
    let bytes = c_string.as_bytes_with_nul();
    for (i, &b) in bytes.iter().enumerate() {
        arr[i] = b as i8;
    }

    Ok(arr)
}

///
/// # Description
///
/// Get information of the current system.
///
/// # Returns
///
/// If successful, a structure with the system information is returned. Otherwise, an error is
/// returned instead.
///
pub fn uname() -> Result<utsname, Error> {
    Ok(utsname {
        sysname: encode_str::<UTSNAME_LENGTH>(env!("NANVIX_SYSNAME"))?,
        nodename: encode_str::<UTSNAME_LENGTH>(env!("NANVIX_NODENAME"))?,
        release: encode_str::<UTSNAME_LENGTH>(env!("CARGO_PKG_VERSION_MAJOR"))?,
        version: encode_str::<UTSNAME_LENGTH>(env!("CARGO_PKG_VERSION_MINOR"))?,
        machine: encode_str::<UTSNAME_LENGTH>(env!("NANVIX_MACHINE"))?,
    })
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::sys::utsname;
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::sys::error::Error;

//===================================================================================================
// System
//===================================================================================================

pub struct System;

impl System {
    ///
    /// # Description
    ///
    /// Returns the system name.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the system name is returned. Otherwise, an error is returned
    /// instead.
    ///
    pub fn system_name() -> Result<String, Error> {
        let utsname: utsname::utsname = utsname::uname()?;

        let cstr_bytes: Vec<u8> = utsname
            .sysname
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();

        let system_name = match String::from_utf8(cstr_bytes) {
            Ok(name) => name,
            Err(_error) => {
                let reason: &str = "failed to convert system name";
                ::syslog::error!("system_name(): {}", reason);
                return Err(Error::new(sys::error::ErrorCode::ValueOutOfRange, reason));
            },
        };

        Ok(system_name)
    }
}

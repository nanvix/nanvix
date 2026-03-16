// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod config;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::sys::config::{
        SysConfigName,
        SysConfigValue,
    },
    sys::utsname,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::arch::mem::PAGE_SIZE;
use ::config::kernel::NUM_PROCESSORS;
use ::sys::error::{
    Error,
    ErrorCode,
};

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

    ///
    /// # Description
    ///
    /// Gets the value of a system configuration variable.
    ///
    /// # Parameters
    ///
    /// - `name` - Name of the system configuration variable to query.
    ///
    /// # Return Value
    ///
    /// On success, this function returns the value of the specified system configuration variable.
    /// Otherwise, it returns an error that indicates the reason of the failure.
    ///
    /// # Errors
    ///
    /// The following errors can be returned by this function:
    ///
    /// - [`ErrorCode::InvalidArgument`] if the specified system configuration name is invalid.
    /// - [`ErrorCode::OperationNotSupported`] if the specified system configuration name is not
    ///   supported.
    /// - [`ErrorCode::ValueOutOfRange`] if the value of the specified system configuration variable
    ///   cannot be represented by the return type.
    ///
    pub fn config(name: SysConfigName) -> Result<SysConfigValue, Error> {
        // Get system configuration variable.
        match name {
            SysConfigName::PageSize => Ok(PAGE_SIZE.try_into()?),
            SysConfigName::NumProcessorsAvailable => Ok(NUM_PROCESSORS.try_into()?),
            _ => {
                let reason: &str = "unsupported system configuration name";
                ::syslog::trace!("config(): {reason}");
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
        }
    }
}

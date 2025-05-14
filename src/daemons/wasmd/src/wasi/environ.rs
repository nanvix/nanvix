// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wasi::{
    types::Errno,
    Size,
    WasiCtxInner,
};
use ::alloc::{
    ffi::CString,
    string::String,
    vec::Vec,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    /// Reads command-line argument data.
    pub(super) fn args_get(&self) -> Result<Vec<String>, Errno> {
        Ok(self.args.clone())
    }

    /// Returns command-line argument data sizes.
    pub(super) fn args_sizes_get(&self) -> Result<(Size, Size), Errno> {
        // Calculate the number of command-line arguments.
        let argc: Size = self.args.len().into();

        // Calculate the size of the command-line data.
        let argv_size: Size = self
            .args
            .iter()
            .map(|s| match CString::new(s.as_str()) {
                Ok(arg_cstr) => arg_cstr.into_bytes_with_nul().len(),
                Err(_) => {
                    ::syslog::error!("args_sizes_get(): skipping invalid command-line argument");
                    0
                },
            })
            .sum::<usize>()
            .into();

        Ok((argc, argv_size))
    }

    /// Read environment variable data.
    pub(super) fn environ_get(&self) -> Result<Vec<String>, Errno> {
        Ok(self.envs.clone())
    }

    /// Returns environment variable data sizes.
    pub(super) fn environ_sizes_get(&self) -> Result<(Size, Size), Errno> {
        // Calculate the number of environment variables.
        let environ_count: Size = self.envs.len().into();

        // Calculate the size of the environment data.
        let environ_data_size: Size = self
            .envs
            .iter()
            .map(|s| match CString::new(s.as_str()) {
                Ok(env_cstr) => env_cstr.into_bytes_with_nul().len(),
                Err(_) => {
                    ::syslog::error!("environ_sizes_get(): skipping invalid environment variable");
                    0
                },
            })
            .sum::<usize>()
            .into();

        Ok((environ_count, environ_data_size))
    }
}

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
                    ::nvx::log!("environ_sizes_get(): skipping invalid environment variable");
                    0
                },
            })
            .sum::<usize>()
            .into();

        Ok((environ_count, environ_data_size))
    }
}

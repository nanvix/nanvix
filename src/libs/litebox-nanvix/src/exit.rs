// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NanvixUserland;
use ::litebox::platform::ExitProvider;

//==================================================================================================
// Implementations
//==================================================================================================

impl ExitProvider for NanvixUserland {
    type ExitCode = i32;
    const EXIT_SUCCESS: Self::ExitCode = 0;
    const EXIT_FAILURE: Self::ExitCode = 1;

    fn exit(&self, _code: Self::ExitCode) -> ! {
        unimplemented!("exit() not implemented in userland");
    }
}

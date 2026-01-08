// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt::{
    self,
    Debug,
    Formatter,
};
use ::sys::pm::{
    ProcessIdentifier,
    ThreadIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores identifiers and up to four raw arguments describing a kernel call request.
///
pub struct KcallArgs {
    pub pid: ProcessIdentifier,
    pub tid: ThreadIdentifier,
    pub number: u32,
    pub arg0: u32,
    pub arg1: u32,
    pub arg2: u32,
    pub arg3: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Debug for KcallArgs {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "KcallArgs {{ pid: {:?}, tid: {:?}, number: {}, arg0: {:#010x}, arg1: {:#010x}, arg2: \
             {:#010x}, arg3: {:#010x} }}",
            self.pid, self.tid, self.number, self.arg0, self.arg1, self.arg2, self.arg3
        )
    }
}

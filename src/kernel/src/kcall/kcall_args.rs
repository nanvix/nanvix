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
    /// Identifier of the calling process.
    pub pid: ProcessIdentifier,
    /// Identifier of the calling thread.
    pub tid: ThreadIdentifier,
    /// Kernel call number to execute.
    pub number: u32,
    /// First kernel call argument.
    pub arg0: u32,
    /// Second kernel call argument.
    pub arg1: u32,
    /// Third kernel call argument.
    pub arg2: u32,
    /// Fourth kernel call argument.
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

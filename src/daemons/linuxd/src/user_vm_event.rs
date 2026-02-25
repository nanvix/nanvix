// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::io::ErrorKind;
use ::sys::ipc::IkcFrame;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// This enum captures the different events that can be sent from a user VM's reading task to the
/// main linuxd thread for processing.
///
pub enum UserVmEvent {
    /// A transfer (message or bulk) received from a user VM.
    Transfer {
        uvm_id: UserVmIdentifier,
        transfer: IkcFrame,
    },
    ConnectionClosed {
        uvm_id: UserVmIdentifier,
    },
    ConnectionError {
        uvm_id: UserVmIdentifier,
        kind: ErrorKind,
    },
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::io::ErrorKind;
use ::sys::ipc::Message;
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
    Message {
        uvm_id: UserVmIdentifier,
        message: Message,
    },
    ConnectionClosed {
        uvm_id: UserVmIdentifier,
    },
    ConnectionError {
        uvm_id: UserVmIdentifier,
        kind: ErrorKind,
    },
}

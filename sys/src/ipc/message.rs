// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::pm::ProcessIdentifier;

//==================================================================================================
//  Structures
//==================================================================================================

pub struct Message {
    pub source: ProcessIdentifier,
    pub destination: ProcessIdentifier,
    pub payload: [u8; Self::SIZE],
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Message {
    pub const SIZE: usize = 64;
}

impl Default for Message {
    fn default() -> Self {
        Self {
            source: ProcessIdentifier::KERNEL,
            destination: ProcessIdentifier::KERNEL,
            payload: [0; Self::SIZE],
        }
    }
}

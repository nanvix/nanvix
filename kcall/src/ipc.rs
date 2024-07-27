// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::{
    Error,
    ErrorCode,
    KcallNumber,
    ProcessIdentifier,
};

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

//==================================================================================================
// Send Message
//==================================================================================================

pub fn send(message: &Message) -> Result<(), Error> {
    let result: i32 = unsafe {
        crate::kcall1(KcallNumber::Send.into(), message as *const Message as usize as u32)
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to send()"))
    }
}

//==================================================================================================
// Receive Message
//==================================================================================================

pub fn recv() -> Result<Message, Error> {
    let mut message: Message = Default::default();

    let result: i32 = unsafe {
        crate::kcall1(KcallNumber::Recv.into(), &mut message as *mut Message as usize as u32)
    };

    if result == 0 {
        Ok(message)
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to recv()"))
    }
}

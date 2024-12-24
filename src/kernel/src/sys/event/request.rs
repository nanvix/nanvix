// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Event Control Request
//==================================================================================================

#[derive(Debug, Clone, Copy)]
pub enum EventCtrlRequest {
    Register,
    Unregister,
}

impl From<EventCtrlRequest> for u32 {
    fn from(req: EventCtrlRequest) -> u32 {
        match req {
            EventCtrlRequest::Register => 0,
            EventCtrlRequest::Unregister => 1,
        }
    }
}

impl TryFrom<u32> for EventCtrlRequest {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Register),
            1 => Ok(Self::Unregister),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid event control request")),
        }
    }
}

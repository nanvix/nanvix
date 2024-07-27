// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventDescriptor,
    pm::ProcessIdentifier,
};
use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug)]
pub struct EventInformation {
    pub id: EventDescriptor,
    pub pid: ProcessIdentifier,
    pub number: Option<usize>,
    pub code: Option<usize>,
    pub address: Option<usize>,
    pub instruction: Option<usize>,
}

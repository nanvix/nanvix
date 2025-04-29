// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::litebox::platform::{
    trivial_providers::{
        TransparentConstPtr,
        TransparentMutPtr,
    },
    RawPointerProvider,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod exit;
pub mod instant;
pub mod mutex;
pub mod network;
pub mod page;
pub mod stdio;

//==================================================================================================

pub struct NanvixUserland;

impl RawPointerProvider for NanvixUserland {
    type RawConstPointer<T: Clone> = TransparentConstPtr<T>;
    type RawMutPointer<T: Clone> = TransparentMutPtr<T>;
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Low-level FAT32 filesystem library for Nanvix guest applications.
//!
//! This crate provides a `no_std`-compatible FAT32 filesystem implementation
//! that operates on in-memory FAT images. It exposes the raw FAT filesystem
//! operations — file I/O, directory management, and metadata queries — on a
//! single FAT image.
//!
//! Mount management, global state, path normalization, and the POSIX-like
//! high-level API live in the `vfs` crate, which uses this crate as a backend.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// External Crate Imports
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Modules
//==================================================================================================

pub mod error;
pub mod fat;

//==================================================================================================
// Public Re-exports
//==================================================================================================

pub use crate::{
    error::Fat32Error,
    fat::{
        Fat,
        FatFile,
        RawMemoryStorage,
    },
};

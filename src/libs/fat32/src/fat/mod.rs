// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! FAT filesystem backend for guest memory regions.
//!
//! This module provides:
//! - [`RawMemoryStorage`]: Low-level adapter implementing fatfs I/O traits
//!   over raw memory.
//! - [`NanvixTimeProvider`]: TimeProvider returning fixed 1980-01-01 timestamp.
//! - [`Fat`]: High-level FAT filesystem wrapper for guest operations.
//! - [`FatFile`]: File handle for FAT files.

//==================================================================================================
// Modules
//==================================================================================================

mod error;
mod file;
mod filesystem;
mod storage;
mod time;

//==================================================================================================
// Public Re-exports
//==================================================================================================

pub use self::{
    file::FatFile,
    filesystem::Fat,
    storage::RawMemoryStorage,
};

//==================================================================================================
// Type Aliases
//==================================================================================================

/// Type alias for the FAT filesystem with our storage and time provider.
pub(crate) type InternalFatFs =
    ::fatfs::FileSystem<RawMemoryStorage, time::NanvixTimeProvider, ::fatfs::LossyOemCpConverter>;

/// Type alias for a FAT file handle.
pub(crate) type InternalFatFile<'a> =
    ::fatfs::File<'a, RawMemoryStorage, time::NanvixTimeProvider, ::fatfs::LossyOemCpConverter>;

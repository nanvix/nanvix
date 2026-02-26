// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! FAT32 filesystem library for nanvix guest applications.
//!
//! This library provides a `no_std`-compatible FAT32 filesystem implementation
//! with a POSIX-like interface. It operates on in-memory FAT images and is
//! designed for use in guest applications running on the nanvix kernel.
//!
//! # Usage
//!
//! ## Initialization
//!
//! ```ignore
//! // Initialize the filesystem.
//! fat32::init()?;
//!
//! // Create a 1MB FAT mount at /data.
//! fat32::create_mount("/data", 1024 * 1024)?;
//! ```
//!
//! ## File Operations
//!
//! ```ignore
//! use fat32::OpenOptions;
//!
//! // Create and write a file.
//! let mut file = OpenOptions::new()
//!     .write(true)
//!     .create(true)
//!     .open("/data/hello.txt")?;
//! file.write(b"Hello, nanvix!")?;
//! file.flush()?;
//! drop(file);
//!
//! // Read the file back.
//! let mut file = fat32::open("/data/hello.txt")?;
//! let content = file.read_to_vec()?;
//! ```
//!
//! ## Directory Operations
//!
//! ```ignore
//! fat32::mkdir("/data/subdir")?;
//!
//! for entry in fat32::read_dir("/data")? {
//!     // Process each entry.
//! }
//!
//! fat32::rmdir("/data/subdir")?;
//! ```
//!
//! ## Metadata
//!
//! ```ignore
//! let info = fat32::stat("/data/hello.txt")?;
//! // info.size, info.is_dir
//! ```

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// External Crate Imports
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Modules
//==================================================================================================

pub mod error;
mod fat;
pub mod file;
mod state;
pub mod vfs;

//==================================================================================================
// Public Re-exports
//==================================================================================================

pub use crate::{
    error::FsError,
    file::{
        chdir,
        cwd,
        file_raw_region,
        mkdir,
        open,
        read_dir,
        rename,
        rmdir,
        stat,
        unlink,
        DirEntry,
        File,
        OpenOptions,
        Stat,
        SEEK_CUR,
        SEEK_END,
        SEEK_SET,
    },
    state::{
        create_mount,
        init,
        is_initialized,
        mount,
        unmount,
        MAX_FAT_SIZE,
        MIN_FAT_SIZE,
    },
};

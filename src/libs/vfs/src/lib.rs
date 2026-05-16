// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Virtual file system for Nanvix guest applications.
//!
//! This crate provides the unified VFS layer that manages mount points,
//! path resolution, file descriptors, and routes operations to concrete
//! filesystem backends (currently FAT32, extensible to ext4, NTFS, etc.).
//!
//! # Usage
//!
//! ## Initialization
//!
//! ```ignore
//! // Initialize the VFS.
//! vfs::init()?;
//!
//! // Create a 1MB FAT mount at /data.
//! vfs::create_mount("/data", 1024 * 1024)?;
//! ```
//!
//! ## File Operations
//!
//! ```ignore
//! use vfs::OpenOptions;
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
//! let mut file = vfs::open("/data/hello.txt")?;
//! let content = file.read_to_vec()?;
//! ```
//!
//! ## POSIX FD Operations (for system call interposition)
//!
//! ```ignore
//! let fd = vfs::fd::vfs_open("/data/hello.txt", 0)?;
//! let mut buf = [0u8; 256];
//! let n = vfs::fd::vfs_read(fd, &mut buf)?;
//! vfs::fd::vfs_close(fd)?;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// External Crate Imports
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Modules
//==================================================================================================

/// FAT32 backend: translates VFS operations to `fat32` crate calls.
pub mod fat32_backend;

/// File descriptor table and POSIX-compatible FD operations.
pub mod fd;

/// High-level file handle and OpenOptions builder.
pub mod file;

/// Mount table and path resolution.
pub mod mount;

/// Global VFS state management.
pub mod state;

//==================================================================================================
// Public Re-exports
//==================================================================================================

pub use crate::{
    file::{
        chdir,
        cwd,
        file_raw_region,
        mkdir,
        normalize,
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
    },
    state::{
        create_mount,
        init,
        is_initialized,
        mount_image,
        unmount,
        MAX_FAT_SIZE,
        MIN_FAT_SIZE,
    },
};

pub use ::fat32::Fat32Error;

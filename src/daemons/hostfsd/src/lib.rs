// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host filesystem daemon (`hostfsd`).
//!
//! This daemon runs on the host side and serves filesystem operations requested by the guest-side
//! VFS daemon (`vfsd`). It receives IKC messages encoded with the `hostfs-api` wire format,
//! performs the corresponding operations on the real host filesystem, and sends responses back.
//!
//! # Security
//!
//! All guest paths are resolved relative to a configured root directory. Path traversal attempts
//! (e.g., `../`) that escape the root are rejected. Symlinks pointing outside the root are also
//! rejected.

mod fd_table;
mod handler;
mod sandbox;

pub use handler::HostFsHandler;
pub use sandbox::Sandbox;

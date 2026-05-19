// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mount/umount request handlers for vfsd.
//!
//! These handlers process mount() and umount() system calls, enabling or disabling
//! the hostfs forwarding subsystem. Only the "hostfs" filesystem type is supported,
//! and only the "/mnt" mount point is valid.

extern crate alloc;

use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    error::build_error,
    hostfs,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::sys::mount::message::{
    MountRequest,
    MountResponse,
    UmountRequest,
    UmountResponse,
};

//==================================================================================================
// Mount Handler
//==================================================================================================

/// Handles a mount() request.
///
/// Currently only supports mounting "hostfs" at "/mnt".
pub(crate) fn handle_mount(source: ThreadIdentifier, request: MountRequest) -> Vec<Message> {
    // Validate filesystem type.
    if request.fstype != "hostfs" {
        ::syslog::warn!(
            "handle_mount(): unsupported fstype {:?} (only \"hostfs\" supported)",
            request.fstype
        );
        return vec![build_error(source, ErrorCode::InvalidArgument)];
    }

    // Validate target mount point.
    if request.target != hostfs::HOSTFS_MOUNT_PATH {
        ::syslog::warn!(
            "handle_mount(): invalid target {:?} (expected {:?})",
            request.target,
            hostfs::HOSTFS_MOUNT_PATH,
        );
        return vec![build_error(source, ErrorCode::InvalidArgument)];
    }

    // Check if already mounted.
    if hostfs::is_enabled() {
        ::syslog::warn!("handle_mount(): hostfs already mounted at {}", hostfs::HOSTFS_MOUNT_PATH);
        return vec![build_error(source, ErrorCode::ResourceBusy)];
    }

    // Enable hostfs forwarding.
    //
    // PRECONDITION: A hostfsd worker must be servicing IKC requests on the host side.
    // In standalone mode this is guaranteed by the io_handler thread spawning the
    // hostfsd-worker before the VM boots. In managed mode, the orchestrator must ensure
    // hostfsd is running before the guest issues mount("hostfs"). If no worker is present,
    // IKC requests will remain unserviced and callers will block on their pending operations
    // indefinitely (there is currently no timeout on the pending queue).
    hostfs::enable();
    ::syslog::info!("handle_mount(): mounted hostfs at {}", hostfs::HOSTFS_MOUNT_PATH);

    vec![MountResponse::build(
        source,
        0,
        ProcessIdentifier::VFSD,
        MessageType::Ipc,
    )]
}

//==================================================================================================
// Umount Handler
//==================================================================================================

/// Handles an umount() request.
///
/// Disables the hostfs forwarding subsystem for the given mount point.
pub(crate) fn handle_umount(source: ThreadIdentifier, request: UmountRequest) -> Vec<Message> {
    // Validate target mount point.
    if request.target != hostfs::HOSTFS_MOUNT_PATH {
        ::syslog::warn!(
            "handle_umount(): invalid target {:?} (expected {:?})",
            request.target,
            hostfs::HOSTFS_MOUNT_PATH,
        );
        return vec![build_error(source, ErrorCode::InvalidArgument)];
    }

    // Check if not mounted.
    if !hostfs::is_enabled() {
        ::syslog::warn!("handle_umount(): hostfs not mounted at {}", hostfs::HOSTFS_MOUNT_PATH);
        return vec![build_error(source, ErrorCode::InvalidArgument)];
    }

    // Disable hostfs forwarding.
    hostfs::disable();
    ::syslog::info!("handle_umount(): unmounted hostfs from {}", hostfs::HOSTFS_MOUNT_PATH);

    vec![UmountResponse::build(
        source,
        0,
        ProcessIdentifier::VFSD,
        MessageType::Ipc,
    )]
}

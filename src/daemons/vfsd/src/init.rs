// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    mm::Address,
    pm::Capability,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

/// Mount path for the RAMFS image (root filesystem).
const RAMFS_MOUNT_PATH: &str = "/";

/// Mount path for the host-mounted directory image.
const MOUNT_DIR_PATH: &str = "/mnt";

//==================================================================================================
// VFS Initialization
//==================================================================================================

pub(crate) fn vfs_init_ramfs() {
    // Initialize the VFS (idempotent — ignore AlreadyInitialized).
    if ::vfs::init().is_err() && !::vfs::is_initialized() {
        ::syslog::warn!("vfs_init_ramfs(): failed to initialize VFS");
        return;
    }

    // Acquire IO management capability.
    if ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true).is_err() {
        ::syslog::warn!("vfs_init_ramfs(): failed to acquire IoManagement capability");
        return;
    }

    // Attempt to allocate and mount the RAMFS MMIO region.
    let mounted: bool = (|| -> bool {
        if ::sys::kcall::mm::__kcall_mmio_alloc(RAMFS_MMIO_TAG).is_err() {
            return false;
        }

        let info: ::sys::mm::MmioRegionInfo =
            match ::sys::kcall::mm::__kcall_mmio_info(RAMFS_MMIO_TAG) {
                Ok(i) => i,
                Err(_) => {
                    let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                    return false;
                },
            };
        let total_size: usize = info.size();
        let base_ptr: *mut u8 = info.base().into_raw_value() as *mut u8;

        let region_slice: &[u8] = unsafe { core::slice::from_raw_parts(base_ptr, total_size) };
        if ::multiimage::is_multiimage(region_slice) {
            let header = match ::multiimage::parse_header(region_slice) {
                Ok(h) => h,
                Err(_) => {
                    ::syslog::warn!("vfs_init_ramfs(): failed to parse multi-image header");
                    let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                    return false;
                },
            };

            let entries = match ::multiimage::parse_entries(region_slice, header.num_images) {
                Ok(e) => e,
                Err(_) => {
                    ::syslog::warn!("vfs_init_ramfs(): failed to parse multi-image entries");
                    let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                    return false;
                },
            };

            if header.total_size as usize > total_size {
                ::syslog::warn!(
                    "vfs_init_ramfs(): header total_size ({}) exceeds MMIO region ({})",
                    header.total_size,
                    total_size
                );
                let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                return false;
            }
            if ::multiimage::validate_entries(entries, header.total_size).is_err() {
                ::syslog::warn!("vfs_init_ramfs(): multi-image entry validation failed");
                let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                return false;
            }

            // Mount ROOTFS at "/".
            if let Some(rootfs) =
                ::multiimage::find_entry_by_tag(entries, &::multiimage::ROOTFS_MMIO_TAG)
            {
                let sub_ptr: *mut u8 = unsafe { base_ptr.add(rootfs.offset as usize) };
                let sub_size: usize = rootfs.size as usize;
                if unsafe { ::vfs::mount_image(RAMFS_MOUNT_PATH, sub_ptr, sub_size, false) }
                    .is_err()
                {
                    ::syslog::warn!("vfs_init_ramfs(): failed to mount ROOTFS at /");
                }
            }
        } else {
            // Legacy single-image path.
            if unsafe { ::vfs::mount_image(RAMFS_MOUNT_PATH, base_ptr, total_size, false) }.is_err()
            {
                ::syslog::warn!("vfs_init_ramfs(): failed to mount RAMFS image");
                let _ = ::sys::kcall::mm::__kcall_mmio_free(RAMFS_MMIO_TAG);
                return false;
            }
        }

        true
    })();

    // Release IO management capability.
    let _ = ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false);

    if mounted {
        ::syslog::info!("vfs_init_ramfs(): mounted RAMFS at {}", RAMFS_MOUNT_PATH);
    }

    // NOTE: hostfs is no longer enabled unconditionally at init. User processes must
    // explicitly call mount("", "/mnt", "hostfs", 0) to enable hostfs forwarding.
    // This ensures that hostfsd is only activated when a mount() system call is issued.
    ::syslog::info!(
        "vfs_init_ramfs(): hostfs available at {} (requires explicit mount)",
        MOUNT_DIR_PATH
    );
}

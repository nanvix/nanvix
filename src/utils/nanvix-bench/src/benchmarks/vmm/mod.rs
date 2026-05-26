// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod boot_time;
#[cfg(feature = "profile-time")]
mod snapshot_restore;
#[cfg(not(feature = "profile-time"))]
mod snapshot_restore_stub;
mod warm_start_vmm;

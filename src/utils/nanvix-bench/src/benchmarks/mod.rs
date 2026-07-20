// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod vmm;

mod gateway;
mod socket_echo;
mod standalone;
mod standalone_socket;
mod standalone_vfs;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Sleep duration (in ms) to wait for the system to clean up after a benchmark run.
///
pub(crate) const CLEANUP_SLEEP_DURATION: u64 = 10;

///
/// # Description
///
/// Sleep duration (in ms) after the warmup echo, before timed iterations begin.
///
pub(crate) const WARMUP_SLEEP_DURATION: u64 = CLEANUP_SLEEP_DURATION;

///
/// # Description
///
/// Maximum number messages that can be queued in a channel.
///
pub(crate) const CHANNEL_CAPACITY: usize = 1024;

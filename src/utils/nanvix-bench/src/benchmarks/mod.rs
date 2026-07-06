// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(any(feature = "multi-process", feature = "single-process"))]
mod system;
mod vmm;

#[cfg(feature = "standalone")]
mod standalone;

#[cfg(feature = "standalone")]
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

///
/// # Description
///
/// Default size of the message we are sending to the user VM.
///
pub(crate) const DEFAULT_PAYLOAD_SIZE: usize = 32;

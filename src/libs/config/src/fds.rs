// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File descriptor range definitions for Nanvix subsystems.

//==================================================================================================
// File Descriptor Ranges
//==================================================================================================

/// Upper bound (exclusive) of the flat per-process descriptor namespace.
///
/// `vfsd` hands out the lowest free descriptor in `[0, MAX_OPEN_FDS)`. Every object — sockets,
/// regular files, pipes, and console streams — draws from this single range, so there is no
/// carved-out sub-range. Once a process exhausts it, allocating a descriptor fails with `EMFILE`.
pub const MAX_OPEN_FDS: i32 = 2048;

/// Base value of `networkd`'s internal socket descriptor space.
///
/// `networkd` numbers the endpoints it owns by offsetting its host-side descriptors by this base.
/// These numbers are an implementation detail of `networkd` and are never seen by applications:
/// under the flat descriptor namespace, `vfsd` owns each socket's application-visible slot and a
/// socket is allocated the lowest free flat descriptor like any other object. This base only keeps
/// `networkd`'s own descriptor space from clashing with the standard streams it inherits.
pub const SOCKET_FD_BASE: i32 = 2048;

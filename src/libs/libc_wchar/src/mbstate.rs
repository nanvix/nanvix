// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// Conversion state for the restartable multibyte functions.
///
/// The layout mirrors the conventional `glibc` definition: a pending-byte counter followed by a
/// four-byte holding buffer. For the stateless UTF-8 encoding the only state that needs to persist
/// across calls is the prefix of an incomplete multibyte sequence.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mbstate_t {
    /// Number of pending bytes held in `bytes`.
    pub count: ::sysapi::ffi::c_int,
    /// Buffered leading bytes of an incomplete sequence.
    pub bytes: [u8; 4],
}

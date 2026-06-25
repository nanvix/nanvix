// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::VirtualAddress;
use ::alloc::boxed::Box;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Parameters of a user-space signal handler installed via `sigaction()`.
///
// Boxed by [`SignalDisposition::Handler`] so that a disposition is only pointer-sized. Storing this
// payload inline in every one of the 64 disposition slots would push the per-process table past the
// kernel heap's maximum slab size (512 bytes); see [`crate::mm::kheap`].
#[allow(dead_code)]
#[derive(Debug)]
pub struct SignalHandler {
    /// Entry point of the user-space handler.
    pub entry: VirtualAddress,
    /// Additional signals to block while the handler runs.
    pub mask: u64,
    /// Handler flags.
    pub flags: i32,
}

///
/// # Description
///
/// Disposition of a single signal.
///
// Variants other than `Default` are constructed by later phases of the signals effort.
#[allow(dead_code)]
#[derive(Debug)]
pub enum SignalDisposition {
    /// Take the default action for the signal.
    Default,
    /// Ignore the signal.
    Ignore,
    /// Run a user-space handler installed via `sigaction()`.
    Handler(Box<SignalHandler>),
}

///
/// # Description
///
/// Per-process signal control block.
///
// Inert plumbing for the signal subsystem: the block is default-initialized so existing behavior
// is unchanged, and its fields are read by later phases of the signals effort.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SignalControl {
    /// Disposition for each of the 64 signals, indexed by `signum - 1`.
    ///
    // Boxed so the 64-entry table is a single heap allocation that stays within the kernel heap's
    // maximum slab size (512 bytes) and keeps `ProcessState` in its original size class.
    dispositions: Box<[SignalDisposition; 64]>,
    /// Process-directed pending signals not yet claimed by a thread.
    pending: u64,
    /// Address of the user-space return trampoline (restorer).
    restorer: Option<VirtualAddress>,
}

//==================================================================================================
// Compile-Time Assertions
//==================================================================================================

// Enforce, at build time, that the boxed dispositions table fits the kernel heap's largest slab
// size class (512 bytes; see `crate::mm::kheap`). The kernel heap rejects any allocation whose size
// or alignment exceeds that bound, which would turn `SignalControl::default()` into a runtime
// allocation failure. The `Box<[SignalDisposition; 64]>` field above relies on `SignalDisposition`
// staying small enough; this assertion fails the compile if a layout change (e.g. dropping the
// niche optimization for the boxed `Handler` payload) ever pushes the table past the bound.
::static_assert::assert_eq!(::core::mem::size_of::<[SignalDisposition; 64]>() <= 512);
::static_assert::assert_eq!(::core::mem::align_of::<[SignalDisposition; 64]>() <= 512);

//==================================================================================================
// Implementations
//==================================================================================================

impl Default for SignalControl {
    fn default() -> Self {
        Self {
            dispositions: Box::new(::core::array::from_fn(|_| SignalDisposition::Default)),
            pending: 0,
            restorer: None,
        }
    }
}

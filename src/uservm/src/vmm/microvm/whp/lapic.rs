// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// LAPIC Constants
//==================================================================================================

/// Hardcoded timer interrupt vector. The Nanvix kernel remaps IRQ0 to
/// vector 0x20 via PIC ICW2, so we inject this vector directly.
pub const TIMER_VECTOR: u32 = 0x20;

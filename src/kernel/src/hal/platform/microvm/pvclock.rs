// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! KVM paravirtualized clock — platform-specific helpers.
//!
//! Reads TSC calibration data and boot time from the shared pvclock page
//! that KVM populates when `MSR_KVM_SYSTEM_TIME_NEW` is enabled.
//!
//! Reference: <https://docs.kernel.org/virt/kvm/x86/msr.html#pvclock>

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    fence,
    Ordering,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// KVM pvclock VCPU time info structure (defined by the KVM ABI).
///
/// The hypervisor writes to this structure, and the guest reads it alongside
/// the CPU's TSC to compute the current time in nanoseconds.
///
/// Reference: Linux kernel `arch/x86/include/asm/pvclock.h`
///
#[repr(C)]
#[derive(Clone, Copy)]
struct KvmPvclockVcpuTimeInfo {
    /// Version counter — odd means update in progress.
    /// Guest must re-read if this changes during a read sequence.
    version: u32,
    _pad0: u32,
    /// TSC value when `system_time` was captured.
    tsc_timestamp: u64,
    /// System time in nanoseconds at `tsc_timestamp`.
    system_time: u64,
    /// Multiplier for TSC → nanoseconds conversion.
    tsc_to_system_mul: u32,
    /// Shift for TSC → nanoseconds conversion (can be negative).
    tsc_shift: i8,
    /// Flags (e.g., TSC stable bit).
    _flags: u8,
    _pad: [u8; 2],
}

// Ensure the pvclock structure matches the KVM ABI (32 bytes).
static_assert::assert_eq_size!(KvmPvclockVcpuTimeInfo, 32);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes `(a * b) >> 32` without 128-bit arithmetic.
///
/// This is the standard algorithm used by the Linux kernel for the pvclock
/// TSC → nanoseconds conversion on 32-bit platforms.
///
/// # Parameters
///
/// - `a`: 64-bit multiplicand (shifted TSC delta).
/// - `b`: 32-bit multiplier (`tsc_to_system_mul`).
///
/// # Returns
///
/// The upper 64 bits of the 96-bit product.
///
fn mul_u64_u32_shr32(a: u64, b: u32) -> u64 {
    let a_lo: u64 = a & 0xFFFF_FFFF;
    let a_hi: u64 = a >> 32;
    let lo_result: u64 = a_lo * u64::from(b);
    let hi_result: u64 = a_hi * u64::from(b);
    (lo_result >> 32) + hi_result
}

///
/// # Description
///
/// Reads the monotonic time in nanoseconds from the KVM pvclock page.
///
/// Uses the seqlock protocol: reads the version, reads the calibration data,
/// reads the version again. If the version changed or was odd, retries.
///
/// # Returns
///
/// - `Some(ns)`: Monotonic nanoseconds since VM start.
/// - `None`: The pvclock page is not initialized (version == 0).
///
pub fn monotonic_time_ns() -> Option<u64> {
    // SAFETY: The pvclock page address is a well-known identity-mapped GPA
    // that KVM populates when the MSR_KVM_SYSTEM_TIME_NEW MSR is enabled.
    let page: *const KvmPvclockVcpuTimeInfo =
        ::config::microvm::DEFAULT_PVCLOCK_PAGE as *const KvmPvclockVcpuTimeInfo;

    loop {
        // Read version counter.
        // SAFETY: The page is mapped in guest memory at the well-known GPA.
        let version1: u32 = unsafe { core::ptr::read_volatile(&(*page).version) };

        // If version is 0, pvclock is not initialized.
        if version1 == 0 {
            return None;
        }

        // If version is odd, an update is in progress — spin and retry.
        if version1 & 1 != 0 {
            core::hint::spin_loop();
            continue;
        }

        // Ensure all subsequent reads see values written before the version.
        fence(Ordering::Acquire);

        // Read calibration data.
        // SAFETY: The page is mapped and version was even (stable snapshot).
        let info: KvmPvclockVcpuTimeInfo = unsafe { core::ptr::read_volatile(page) };

        // Read TSC inside the seqlock region so that the calibration
        // parameters and the TSC snapshot form a consistent pair.
        let tsc: u64 = ::arch::cpu::rdtsc();

        // Ensure we've finished reading before re-checking the version.
        fence(Ordering::Acquire);

        // Re-read version counter to verify consistency.
        // SAFETY: Same page, checking for concurrent updates.
        let version2: u32 = unsafe { core::ptr::read_volatile(&(*page).version) };
        if version1 != version2 {
            // Data was modified during our read — retry.
            core::hint::spin_loop();
            continue;
        }

        // Compute TSC delta since calibration point.
        let delta: u64 = tsc.wrapping_sub(info.tsc_timestamp);

        // Use checked shifts to avoid panics if the hypervisor provides
        // an out-of-range shift (e.g., >= 64 for a u64). Treat such cases
        // as "pvclock unavailable".
        let shift: u32 = u32::from(info.tsc_shift.unsigned_abs());
        let shifted: u64 = if info.tsc_shift >= 0 {
            delta.checked_shl(shift)?
        } else {
            delta.checked_shr(shift)?
        };

        // Convert shifted TSC delta to nanoseconds and add the base system time.
        let delta_ns: u64 = mul_u64_u32_shr32(shifted, info.tsc_to_system_mul);
        return Some(info.system_time.wrapping_add(delta_ns));
    }
}

///
/// # Description
///
/// Reads the boot time in nanoseconds since the Unix epoch.
///
/// This value is written by the VMM during VM initialization at a well-known
/// offset within the pvclock page.
///
/// # Returns
///
/// UTC nanoseconds since 1970-01-01 00:00:00 at the time the VM was started.
///
pub fn boot_time_ns() -> u64 {
    let offset: usize =
        ::config::microvm::DEFAULT_PVCLOCK_PAGE + ::config::microvm::PVCLOCK_BOOT_TIME_NS_OFFSET;
    // SAFETY: This address is identity-mapped in guest memory and written by the VMM.
    unsafe { core::ptr::read_volatile(offset as *const u64) }
}

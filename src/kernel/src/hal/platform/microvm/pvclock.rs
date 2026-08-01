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
// Constants
//==================================================================================================

/// Number of nanoseconds elapsed per LAPIC timer tick (1 kHz timer, thus 1 ms per tick).
#[cfg(any(feature = "whp", feature = "test"))]
const NS_PER_TICK: u64 = 1_000_000;

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
#[cfg(not(feature = "whp"))]
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
/// Reconciles a new host pvclock sample with the time already interpolated by the guest.
///
/// # Parameters
///
/// - `host_time_ns`: New host-provided time in nanoseconds.
/// - `previous_base_ns`: Previous guest snapshot time in nanoseconds.
/// - `previous_base_ticks`: Guest tick count at the previous snapshot.
/// - `snapshot_ticks`: Guest tick count for the new snapshot.
///
/// # Returns
///
/// A snapshot time that does not precede the guest's previously interpolated timeline.
///
#[cfg(any(feature = "whp", feature = "test"))]
fn reconcile_snapshot_time(
    host_time_ns: u64,
    previous_base_ns: u64,
    previous_base_ticks: u32,
    snapshot_ticks: u32,
) -> u64 {
    let elapsed_ticks: u32 = snapshot_ticks.wrapping_sub(previous_base_ticks);
    let interpolated_ns: u64 =
        previous_base_ns.wrapping_add(u64::from(elapsed_ticks) * NS_PER_TICK);

    // Monotonicity outranks accuracy here: a guest tick rate that runs slightly fast makes the
    // clock drift ahead of the host, but it never exposes a backwards jump to user space.
    host_time_ns.max(interpolated_ns)
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
    // On WHP, the VMM sets tsc_to_system_mul=0 so TSC-based computation
    // always yields system_time unchanged. Instead, use a hybrid approach:
    // the host timer (100 Hz) periodically updates system_time on the pvclock
    // page. Between host updates, we interpolate using the LAPIC timer tick
    // count (1 kHz) to achieve ~1 ms accuracy with zero VM exits.
    #[cfg(feature = "whp")]
    {
        use core::sync::atomic::AtomicU32;

        // Snapshot state for tick interpolation. Split u64 values into
        // two AtomicU32s because AtomicU64 is unavailable on i686.
        // Single-vCPU guest, so no concurrent writer races.
        static SNAP_VERSION: AtomicU32 = AtomicU32::new(0);
        static SNAP_SYSTEM_TIME_LO: AtomicU32 = AtomicU32::new(0);
        static SNAP_SYSTEM_TIME_HI: AtomicU32 = AtomicU32::new(0);
        static SNAP_TICKS: AtomicU32 = AtomicU32::new(0);

        let page: *const KvmPvclockVcpuTimeInfo =
            ::config::microvm::DEFAULT_PVCLOCK_PAGE as *const KvmPvclockVcpuTimeInfo;

        // SAFETY: The pvclock page is identity-mapped in guest memory.
        let version: u32 = unsafe { core::ptr::read_volatile(&(*page).version) };

        if version == 0 {
            return None;
        }

        let snapshot_version: u32 = SNAP_VERSION.load(Ordering::Relaxed);

        // If the host timer updated pvclock (version changed), take a new snapshot.
        if version != snapshot_version && version & 1 == 0 {
            fence(Ordering::Acquire);
            // SAFETY: Page is mapped and version was even (stable snapshot).
            let info: KvmPvclockVcpuTimeInfo = unsafe { core::ptr::read_volatile(page) };
            fence(Ordering::Acquire);
            // SAFETY: Re-check version for consistency.
            let version2: u32 = unsafe { core::ptr::read_volatile(&(*page).version) };
            if version == version2 {
                let snapshot_ticks: u32 = crate::pm::clock::ticks() as u32;
                // Reconcile even the first stable host sample because an earlier odd-version read
                // may have already exposed interpolation from the zero-initialized snapshot.
                let previous_base_ns: u64 = (SNAP_SYSTEM_TIME_HI.load(Ordering::Relaxed) as u64)
                    << 32
                    | SNAP_SYSTEM_TIME_LO.load(Ordering::Relaxed) as u64;
                let previous_base_ticks: u32 = SNAP_TICKS.load(Ordering::Relaxed);
                let snapshot_system_time: u64 = reconcile_snapshot_time(
                    info.system_time,
                    previous_base_ns,
                    previous_base_ticks,
                    snapshot_ticks,
                );

                SNAP_SYSTEM_TIME_LO.store(snapshot_system_time as u32, Ordering::Relaxed);
                SNAP_SYSTEM_TIME_HI.store((snapshot_system_time >> 32) as u32, Ordering::Relaxed);
                SNAP_TICKS.store(snapshot_ticks, Ordering::Relaxed);
                SNAP_VERSION.store(version, Ordering::Relaxed);
                return Some(snapshot_system_time);
            }
            // Version changed mid-read — fall through to interpolation.
        }

        // Interpolate from snapshot using tick delta (1 tick = 1 ms = 1,000,000 ns).
        let base_ns: u64 = (SNAP_SYSTEM_TIME_HI.load(Ordering::Relaxed) as u64) << 32
            | SNAP_SYSTEM_TIME_LO.load(Ordering::Relaxed) as u64;
        let base_ticks: u32 = SNAP_TICKS.load(Ordering::Relaxed);
        let delta_ticks: u32 = (crate::pm::clock::ticks() as u32).wrapping_sub(base_ticks);
        Some(base_ns.wrapping_add(delta_ticks as u64 * NS_PER_TICK))
    }

    // KVM/QEMU path: use TSC-based computation with the seqlock protocol.
    #[cfg(not(feature = "whp"))]
    {
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "test")]
mod test {
    use super::reconcile_snapshot_time;

    /// Verifies that a lagging host sample cannot move the guest timeline backwards.
    fn test_reconcile_snapshot_time_clamps_lagging_host_sample() -> bool {
        let host_time_ns: u64 = 110_300_000;
        let previous_base_ns: u64 = 100_800_000;
        let previous_base_ticks: u32 = 100;
        let snapshot_ticks: u32 = 110;
        let expected_time_ns: u64 = 110_800_000;

        let actual_time_ns: u64 = reconcile_snapshot_time(
            host_time_ns,
            previous_base_ns,
            previous_base_ticks,
            snapshot_ticks,
        );
        if actual_time_ns != expected_time_ns {
            error!(
                "lagging host sample was not clamped (expected={expected_time_ns}, \
                 actual={actual_time_ns})"
            );
            return false;
        }

        true
    }

    /// Verifies that a host sample ahead of guest interpolation is adopted.
    fn test_reconcile_snapshot_time_adopts_leading_host_sample() -> bool {
        let host_time_ns: u64 = 111_200_000;
        let previous_base_ns: u64 = 100_800_000;
        let previous_base_ticks: u32 = 100;
        let snapshot_ticks: u32 = 110;

        let actual_time_ns: u64 = reconcile_snapshot_time(
            host_time_ns,
            previous_base_ns,
            previous_base_ticks,
            snapshot_ticks,
        );
        if actual_time_ns != host_time_ns {
            error!(
                "leading host sample was not adopted (expected={host_time_ns}, \
                 actual={actual_time_ns})"
            );
            return false;
        }

        true
    }

    /// Runs all pvclock in-kernel tests.
    pub(super) fn test() -> bool {
        let mut passed: bool = true;
        passed &= run_test!(test_reconcile_snapshot_time_clamps_lagging_host_sample);
        passed &= run_test!(test_reconcile_snapshot_time_adopts_leading_host_sample);
        passed
    }
}

///
/// # Description
///
/// Runs the pvclock in-kernel tests.
///
/// # Returns
///
/// `true` if every test passed, `false` otherwise.
///
#[cfg(feature = "test")]
pub fn test() -> bool {
    test::test()
}

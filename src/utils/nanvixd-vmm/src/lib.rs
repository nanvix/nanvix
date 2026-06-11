// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Boots the Nanvix guest in standalone mode on top of the OpenVMM
//! virtualization stack, while reusing the real Nanvix host-side daemons.
//!
//! This crate lives in the Nanvix workspace so it can depend directly on the
//! production `hostfsd` and `networkd` crates (and the shared
//! `sys`/`syscall`/`config` types) instead of re-implementing their wire
//! protocols. The OpenVMM virtualization libraries (`virt`, `virt_kvm`,
//! `guestmem`, ...) are consumed via cross-workspace path dependencies.
//!
//! The single binary it ships, `nanvixd-vmm`, mirrors the Nanvix `nanvixd`
//! standalone deployment CLI, adding `-mount <dir>` (served by the reused
//! `hostfsd`) and `-allow-host-networking` (served by the reused `networkd`).

pub mod device;
pub mod ikc;
pub mod io;
pub mod load;
pub mod stdin;
pub mod vmm;

// Make the in-tree legacy 8259 PIC and 8254 PIT chipset devices resolvable by
// any `ResourceResolver` in this binary, so the VM core can instantiate them
// from their device handles.
vm_resource::register_static_resolvers! {
    chipset::pic::resolver::PicResolver,
    chipset::pit::resolver::PitResolver,
}

/// Default guest RAM size: 128 MiB, matching the Nanvix microvm default.
pub const DEFAULT_MEM_SIZE: u64 = 128 * 1024 * 1024;

/// A host-side sink for the guest's kernel console output.
pub type ConsoleSink = Box<dyn std::io::Write + Send>;

/// Initializes host-side logging, reading its filter from `env_var`.
///
/// Logs are emitted to stderr so they never intermingle with guest application
/// output on stdout. The default level is `info`; it can be overridden through
/// the given environment variable (e.g. `NANVIXD_VMM_LOG=debug`).
pub fn init_logging(env_var: &str) {
    let mut builder = env_logger::Builder::new();
    builder
        .target(env_logger::Target::Stderr)
        .filter_level(log::LevelFilter::Info);
    if let Ok(filter) = std::env::var(env_var) {
        builder.parse_filters(&filter);
    }
    let _ = builder.try_init();
}

/// Returns the host TSC base frequency in MHz, used by the guest to calibrate
/// its LAPIC timer via `RDTSC`.
///
/// Prefers CPUID leaf `0x16` (processor frequency information); falls back to
/// parsing `/proc/cpuinfo`, and finally to a conservative default. The guest's
/// calibration is self-correcting, so an approximate value is sufficient as
/// long as it is nonzero and in the right ballpark.
pub fn host_tsc_freq_mhz() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        // `__cpuid` is part of the x86_64 baseline; the leaves read here have no
        // side effects and the results are only used as a calibration hint.
        let leaf = std::arch::x86_64::__cpuid(0);
        if leaf.eax >= 0x16 {
            let freq = std::arch::x86_64::__cpuid(0x16);
            let mhz = freq.eax & 0xffff;
            if mhz != 0 {
                return mhz;
            }
        }
    }

    if let Some(mhz) = cpuinfo_mhz() {
        return mhz;
    }

    // Conservative default (2 GHz); calibration corrects for the real rate.
    2000
}

/// Parses the first `cpu MHz` value from `/proc/cpuinfo`, if available.
fn cpuinfo_mhz() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "cpu MHz" {
                if let Ok(mhz) = value.trim().parse::<f64>() {
                    if mhz > 0.0 {
                        return Some(mhz as u32);
                    }
                }
            }
        }
    }
    None
}

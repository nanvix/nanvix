// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(unix)]
use ::libc::{
    cpu_set_t,
    sched_setaffinity,
    CPU_SET,
    CPU_ZERO,
};
use ::log::error;
use ::serde::Deserialize;
#[cfg(unix)]
use ::std::mem;
use anyhow::Result;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Number of CPUs that a CPU mask can refer to. CPU indices in a CPU mask must be less than this
/// value, which is the number of CPUs that the CPU set of the GNU C Library can hold
/// (`CPU_SETSIZE`).
///
pub const MAX_CPUS: usize = 1024;

// Every CPU that a CPU mask can refer to must fit in the CPU set that pins threads.
#[cfg(unix)]
const _: () = assert!(MAX_CPUS <= 8 * mem::size_of::<cpu_set_t>());

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Hardware locality configuration, which lists the CPUs on which each Nanvix component runs.
///
/// It is loaded from a JSON file, in which each entry is the CPU mask of a component (see
/// [`parse_cpu_mask()`]). Every entry is optional, because each component reads only its own
/// entry. Unknown entries, such as those of components that no longer exist, are ignored.
///
#[derive(Clone, Debug, Deserialize)]
pub struct HwLoc {
    /// CPU mask of the benchmark client (`nanvix-bench`).
    client_core_str: Option<String>,
    /// CPU mask of the Nanvix Daemon (`nanvixd`), including the User VMs that it runs.
    nanvixd_core_str: Option<String>,
}

impl HwLoc {
    /// Returns the CPU mask of the benchmark client, if any.
    pub fn get_client_core_str(&self) -> Option<&str> {
        self.client_core_str.as_deref()
    }

    /// Returns the CPU mask of the Nanvix Daemon, if any.
    pub fn get_nanvixd_core_str(&self) -> Option<&str> {
        self.nanvixd_core_str.as_deref()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses a CPU mask into the indices of the CPUs that it lists.
///
/// A CPU mask is a comma-separated list of entries, each of which is either a CPU index or an
/// inclusive range of CPU indices (e.g., `0-3,5,7-8`). Whitespace around entries and range bounds
/// is ignored.
///
/// # Parameters
///
/// - `mask`: CPU mask.
///
/// # Returns
///
/// On success, returns the indices of the CPUs that the mask lists, in the order in which they
/// appear. On failure, returns an error that describes the first malformed entry: an empty
/// entry, an entry that is neither a CPU index nor a range of CPU indices, a range whose start is
/// greater than its end, or a CPU index that is not less than [`MAX_CPUS`].
///
pub fn parse_cpu_mask(mask: &str) -> Result<Vec<usize>> {
    let mut cpus: Vec<usize> = Vec::new();
    for entry in mask.split(',') {
        if entry.trim().is_empty() {
            anyhow::bail!("invalid CPU mask entry {entry:?}: entry is empty");
        }
        let (start, end): (usize, usize) = match entry.split_once('-') {
            Some((start, end)) => (parse_cpu_index(entry, start)?, parse_cpu_index(entry, end)?),
            None => {
                let cpu: usize = parse_cpu_index(entry, entry)?;
                (cpu, cpu)
            },
        };
        if start > end {
            anyhow::bail!(
                "invalid CPU mask entry {entry:?}: range start is greater than range end"
            );
        }
        cpus.extend(start..=end);
    }
    Ok(cpus)
}

///
/// # Description
///
/// Parses a CPU index of a CPU mask entry.
///
/// # Parameters
///
/// - `entry`: CPU mask entry that contains the CPU index.
/// - `index`: CPU index.
///
/// # Returns
///
/// On success, returns the CPU index. On failure, returns an error: `index` is not a CPU index,
/// or it is not less than [`MAX_CPUS`].
///
fn parse_cpu_index(entry: &str, index: &str) -> Result<usize> {
    let index: &str = index.trim();
    let cpu: usize = index.parse().map_err(|_| {
        anyhow::anyhow!("invalid CPU mask entry {entry:?}: {index:?} is not a CPU index")
    })?;
    if cpu >= MAX_CPUS {
        anyhow::bail!(
            "invalid CPU mask entry {entry:?}: CPU index {cpu} is out of range (maximum is {})",
            MAX_CPUS - 1
        );
    }
    Ok(cpu)
}

///
/// # Description
///
/// Pins the calling thread to a set of CPUs.
///
/// # Parameters
///
/// - `cpus`: Indices of the CPUs. Each index must be less than [`MAX_CPUS`].
///
/// # Returns
///
/// On success, returns empty. On failure, returns an error that describes why the CPU affinity of
/// the calling thread could not be changed.
///
#[cfg(unix)]
fn pin_current_thread(cpus: &[usize]) -> Result<()> {
    // SAFETY: `cpu_set_t` is a plain bit array, for which all zeroes is a valid (empty) value.
    let mut set: cpu_set_t = unsafe { mem::zeroed() };
    // SAFETY: `set` is a valid CPU set, and every CPU index is less than `MAX_CPUS`, which fits in
    // the CPU set, as asserted at compile time.
    unsafe {
        CPU_ZERO(&mut set);
        for cpu in cpus {
            CPU_SET(*cpu, &mut set);
        }
    }
    // SAFETY: `set` is a valid CPU set of the given size, and pid 0 refers to the calling thread.
    let ret: ::libc::c_int = unsafe { sched_setaffinity(0, mem::size_of::<cpu_set_t>(), &set) };
    if ret != 0 {
        anyhow::bail!("{}", ::std::io::Error::last_os_error());
    }
    Ok(())
}

///
/// # Description
///
/// Pins the calling thread to a set of CPUs. CPU pinning is not supported on this platform, so
/// this function leaves the CPU affinity of the calling thread unchanged.
///
/// # Parameters
///
/// - `cpus`: Indices of the CPUs (unused).
///
/// # Returns
///
/// This function always succeeds.
///
#[cfg(not(unix))]
fn pin_current_thread(cpus: &[usize]) -> Result<()> {
    let _ = cpus;
    Ok(())
}

///
/// # Description
///
/// Pins the calling thread to the CPUs that a CPU mask lists.
///
/// Threads that the calling thread creates afterwards inherit its CPU affinity, so pinning the
/// main thread of a program before it creates other threads pins the whole program.
///
/// CPU pinning is only supported on Unix hosts. On other hosts, the CPU mask is still validated,
/// but the CPU affinity of the calling thread is left unchanged.
///
/// # Parameters
///
/// - `mask`: CPU mask (see [`parse_cpu_mask()`]).
///
/// # Returns
///
/// On success, returns empty. On failure, returns an error: the CPU mask is malformed, or the CPU
/// affinity of the calling thread could not be changed.
///
pub fn pin_main_thread(mask: String) -> Result<()> {
    if let Err(error) = parse_cpu_mask(&mask).and_then(|cpus| pin_current_thread(&cpus)) {
        let reason: String = format!("failed to pin thread to CPU mask {mask:?}: {error}");
        error!("pin_main_thread(): {reason}");
        anyhow::bail!(reason);
    }
    Ok(())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;

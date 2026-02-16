// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::error;
use anyhow::Result;
use libc::{
    cpu_set_t,
    sched_setaffinity,
    CPU_SET,
    CPU_ZERO,
};
use serde::Deserialize;
use std::{
    mem,
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Debug, Deserialize)]
pub struct HwLoc {
    client_core_str: String,
    linuxd_core_str: String,
    nanovm_core_str: String,
}

impl HwLoc {
    pub fn get_client_core_str(&self) -> String {
        self.client_core_str.clone()
    }

    pub fn get_linuxd_core_str(&self) -> String {
        self.linuxd_core_str.clone()
    }

    pub fn get_nanovm_core_str(&self) -> String {
        self.nanovm_core_str.clone()
    }
}

/// Parses a CPU mask string (e.g., "0-3,5,7-8") into a Vec of CPU indices.
fn parse_cpu_mask(mask: &str) -> Result<Vec<usize>> {
    let mut cpus = Vec::new();
    for part in mask.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let start = usize::from_str(start.trim())
                .map_err(|_| format!("Invalid start: {part}"))
                .unwrap();
            let end = usize::from_str(end.trim())
                .map_err(|_| format!("Invalid end: {part}"))
                .unwrap();
            cpus.extend(start..=end);
        } else {
            let cpu = usize::from_str(part.trim())
                .map_err(|_| format!("Invalid CPU: {part}"))
                .unwrap();
            cpus.push(cpu);
        }
    }
    Ok(cpus)
}

/// Pins the current thread to a given CPU set parsed from a mask string.
fn pin_current_thread_to_mask(mask: &str) -> Result<()> {
    match parse_cpu_mask(mask) {
        Ok(cpus) => unsafe {
            let mut set: cpu_set_t = mem::zeroed();
            CPU_ZERO(&mut set);
            for cpu in cpus {
                CPU_SET(cpu, &mut set);
            }
            let ret = sched_setaffinity(0, mem::size_of::<cpu_set_t>(), &set);
            if ret != 0 {
                error!(
                    "Failed to set CPU affinity for thread: {}",
                    std::io::Error::last_os_error()
                );
                return Err(anyhow::anyhow!("Failed to set CPU affinity for thread"));
            }

            Ok(())
        },
        Err(e) => {
            error!("Failed to parse CPU mask '{mask}': {e}");
            Err(anyhow::anyhow!("Failed to parse CPU mask"))
        },
    }
}

/// This method pins the client (running thread) to a pre-defined CPU core.
pub fn pin_main_thread(mask: String) -> Result<()> {
    pin_current_thread_to_mask(&mask)
}

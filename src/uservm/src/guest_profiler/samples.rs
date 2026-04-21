// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Guest stack sampling and frame-pointer walking.

//==================================================================================================
// Imports
//==================================================================================================

use super::gva;
use ::std::sync::{
    Arc,
    Mutex,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum frame-pointer chain depth per sample.
const MAX_STACK_DEPTH: usize = 128;

/// Kernel/user boundary. Addresses below this are kernel (identity-mapped).
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const USER_BASE: u32 = config::memory_layout::KERNEL_END_RAW as u32;

/// Minimum valid kernel code address (1 MiB — the x86 kernel boot/load address).
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const KERNEL_CODE_MIN: u32 = config::constants::MEGABYTE as u32;
/// Minimum valid stack address (64 KiB — above the real-mode IVT/BDA region).
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const STACK_ADDR_MIN: u32 = (64 * config::constants::KILOBYTE) as u32;

//==================================================================================================
// Stack Sample
//==================================================================================================

/// A single stack sample captured from the guest.
#[derive(Clone)]
pub struct StackSample {
    /// Return addresses from the frame-pointer chain (deepest first).
    pub addresses: Vec<u32>,
}

//==================================================================================================
// Guest Profiler
//==================================================================================================

/// Collects guest stack samples from the host side.
///
/// Thread-safe: the sampling thread pushes samples, the main thread
/// reads them after VM exit.
pub struct GuestProfiler {
    samples: Arc<Mutex<Vec<StackSample>>>,
}

impl GuestProfiler {
    /// Creates a new profiler with pre-allocated sample storage.
    pub fn new(expected_samples: usize) -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::with_capacity(expected_samples))),
        }
    }

    /// Returns a clone of the inner Arc for use by the sampling thread.
    pub fn handle(&self) -> Arc<Mutex<Vec<StackSample>>> {
        self.samples.clone()
    }

    /// Captures one stack sample from the guest.
    ///
    /// # Safety
    ///
    /// The caller must ensure the guest vCPU is stopped (not executing) when
    /// this function is called, so that guest memory (page tables, stack
    /// frames) is stable and will not be concurrently modified.
    ///
    /// # Parameters
    ///
    /// - `vmem_ptr`: Host pointer to guest physical memory.
    /// - `vmem_size`: Total guest physical memory size.
    /// - `eip`: Guest instruction pointer.
    /// - `ebp`: Guest base pointer (frame pointer).
    /// - `cr3`: Guest CR3 (page directory physical address).
    pub fn capture_sample(
        samples: &Mutex<Vec<StackSample>>,
        vmem_ptr: *const u8,
        vmem_size: usize,
        eip: u32,
        ebp: u32,
        cr3: u32,
    ) {
        let mut addrs = Vec::with_capacity(MAX_STACK_DEPTH);

        // Only record if EIP looks like a valid code address.
        if !is_valid_code_addr(eip) {
            return;
        }

        addrs.push(eip);

        let mut current_ebp: u32 = ebp;
        for _ in 0..MAX_STACK_DEPTH {
            // EBP must be 4-byte aligned and in a valid range.
            if !is_valid_stack_addr(current_ebp) {
                break;
            }

            // Translate EBP to GPA.
            let gpa = if current_ebp < USER_BASE {
                // Kernel: identity-mapped.
                current_ebp
            } else {
                // User space: walk guest page tables.
                match gva::translate_gva(vmem_ptr, vmem_size, cr3, current_ebp) {
                    Some(gpa) => gpa,
                    None => break,
                }
            };

            // Read saved EBP and return address.
            let saved_ebp: u32 = match gva::read_gpa_u32(vmem_ptr, vmem_size, gpa) {
                Some(v) => v,
                None => break,
            };
            let return_addr = match gva::read_gpa_u32(vmem_ptr, vmem_size, gpa + 4) {
                Some(v) => v,
                None => break,
            };

            if return_addr == 0 || !is_valid_code_addr(return_addr) {
                break;
            }

            addrs.push(return_addr);

            // EBP should move up the stack (higher addresses) and remain valid.
            if saved_ebp != 0 && (!is_valid_stack_addr(saved_ebp) || saved_ebp <= current_ebp) {
                break;
            }
            current_ebp = saved_ebp;
        }

        if !addrs.is_empty()
            && let Ok(mut s) = samples.lock()
        {
            s.push(StackSample { addresses: addrs });
        }
    }

    /// Returns all collected samples and clears the buffer.
    pub fn drain_samples(&self) -> Vec<StackSample> {
        if let Ok(mut s) = self.samples.lock() {
            std::mem::take(&mut *s)
        } else {
            Vec::new()
        }
    }

    /// Writes samples as folded stacks to a file.
    ///
    /// Each line: `func1;func2;func3 count`
    pub fn write_folded<R: Fn(u32) -> String>(
        &self,
        path: &str,
        resolve: R,
    ) -> std::io::Result<()> {
        use std::{
            collections::HashMap,
            io::Write,
        };

        // Open the file before draining samples so that data is not lost if
        // file creation fails.
        let mut file = std::fs::File::create(path)?;

        let samples = self.drain_samples();
        let mut folded: HashMap<String, u64> = HashMap::new();

        for sample in &samples {
            // Build the stack string (deepest frame first → reverse for flamegraph).
            let stack: String = sample
                .addresses
                .iter()
                .rev()
                .map(|&addr| resolve(addr))
                .collect::<Vec<_>>()
                .join(";");

            *folded.entry(stack).or_insert(0) += 1;
        }

        let mut entries: Vec<_> = folded.into_iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        for (stack, count) in entries {
            writeln!(file, "{} {}", stack, count)?;
        }

        Ok(())
    }
}

/// Returns true if the address looks like valid kernel or user code.
///
/// Rejects addresses below `KERNEL_CODE_MIN` (the first 1 MiB), which
/// contains real-mode IVT, BIOS data, and other non-code regions.
fn is_valid_code_addr(addr: u32) -> bool {
    addr >= KERNEL_CODE_MIN
}

/// Returns true if the address looks like a valid stack frame pointer.
fn is_valid_stack_addr(ebp: u32) -> bool {
    ebp >= STACK_ADDR_MIN && ebp != 0xFFFF_FFFF && (ebp & 3) == 0
}

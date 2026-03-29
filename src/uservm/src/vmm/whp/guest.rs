// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The WHP guest uses pointer-to-usize casts for address arithmetic.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    elf,
    pal::FileMapping,
    vmm::whp::{
        vcpu::VirtualProcessor,
        vmem::VirtualMemory,
    },
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    mem,
    ptr,
};

/// Page size in bytes (4 KiB, matching the x86 architecture).
const PAGE_SIZE: usize = 4096;

//==================================================================================================
// Structures
//==================================================================================================

/// Serializable guest state for WHP snapshot/restore.
#[derive(Serialize, Deserialize)]
pub struct GuestState {
    /// Kernel location and size.
    kernel: Option<(usize, usize)>,
    /// Initial RAM disk location and size.
    initrd: Option<(usize, usize)>,
    /// IKC credits counter.
    credits: u32,
    /// Kernel entry point address.
    entry: usize,
}

#[derive(Debug, Default)]
pub struct Guest {
    /// Kernel location and size.
    kernel: Option<(usize, usize)>,
    /// Initial RAM disk location and size.
    initrd: Option<(usize, usize)>,
    /// Control register used to inform the guest about the number of messages ready to be consumed.
    credits: u32,
    /// Entry point of the guest.
    entry: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Guest {
    ///
    /// # Description
    ///
    /// Loads the kernel into the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `kernel_filename`: Path to the kernel binary file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the entry point of the kernel that was
    /// loaded into the virtual memory. Otherwise, it returns an error.
    ///
    pub fn load_kernel(&mut self, vmem: &mut VirtualMemory, kernel_filename: &str) -> Result<()> {
        trace!("load_kernel(): {kernel_filename}");

        let (ptr, size) = (vmem.get_raw_ptr(), vmem.get_size());

        let elf: FileMapping = FileMapping::open(kernel_filename)?;
        let (entry, first_address, size): (usize, usize, usize) =
            unsafe { elf::load(ptr.cast::<::std::ffi::c_void>(), elf.ptr(), size)? };

        self.kernel = Some((first_address, size));
        self.entry = entry;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads the initial RAM disk into the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `initrd_filename`: Path to the initial RAM disk.
    /// - `initrd_args`: Optional arguments forwarded to the initrd payload.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns. Otherwise, it returns an error.
    ///
    pub fn load_initrd(
        &mut self,
        vmem: &mut VirtualMemory,
        initrd_filename: &str,
        initrd_args: Option<String>,
    ) -> Result<()> {
        trace!("load_initrd(): initrd_filename={}, initrd_args={:?}", initrd_filename, initrd_args);

        debug!("load_initrd(): mapping initrd file");
        let initrd: FileMapping = FileMapping::open(initrd_filename)?;

        // Check if initrd would overlap with kernel in guest physical address space.
        if let Some((kernel_base, kernel_size)) = self.kernel
            && ::config::microvm::DEFAULT_INITRD_BASE < (kernel_base + kernel_size)
        {
            let reason: String = "initrd overlaps with kernel".to_string();
            error!("load_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Check if initrd fits into virtual memory.
        let (ptr, size) = (vmem.get_raw_ptr(), vmem.get_size());
        if (::config::microvm::DEFAULT_INITRD_BASE + initrd.size()) > size {
            let reason: String = "initrd does not fit into virtual memory".to_string();
            error!("load_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        if (ptr as usize) + ::config::microvm::DEFAULT_INITRD_BASE + initrd.size()
            > (ptr as usize) + size
        {
            let reason: String = "initrd would cause address overflow".to_string();
            error!("load_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            let src: *const u8 = initrd.ptr();
            let dst: *mut u8 = ptr.add(::config::microvm::DEFAULT_INITRD_BASE);

            if src.is_null() || dst.is_null() {
                let reason: String = "null pointer encountered while copying initrd".to_string();
                error!("load_initrd(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            debug!(
                "load_initrd(): copying initrd into virtual memory (src={:?}, dst={:?}, size={})",
                src,
                dst,
                initrd.size()
            );

            ptr::copy_nonoverlapping(src, dst, initrd.size());
        }

        debug!("load_initrd(): adjusting initrd size to page size");

        // Ensure initrd size is aligned to page size.
        let initrd_size: usize = if !initrd.size().is_multiple_of(PAGE_SIZE) {
            debug!("load_initrd(): aligning initrd size to page size");
            initrd.size() + (PAGE_SIZE - (initrd.size() % PAGE_SIZE))
        } else {
            initrd.size()
        };

        self.initrd = Some((::config::microvm::DEFAULT_INITRD_BASE, initrd_size));

        // Write arguments to the virtual machine.
        let mut args: String = ::std::path::Path::new(initrd_filename)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(initrd_filename)
            .to_string();

        if let Some(ref initrd_args) = initrd_args {
            args.push_str(&format!(" {initrd_args}"));
        }

        debug!("load_initrd(): writing args to virtual memory: {}", args);
        self.write_args(vmem, &args)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Returns the base address and size (in bytes) of the initrd currently loaded in memory.
    ///
    pub fn initrd_region(&self) -> Option<(usize, usize)> {
        self.initrd
    }

    /// Writes command line arguments into the virtual memory.
    fn write_args(&mut self, vmem: &mut VirtualMemory, args: &str) -> Result<()> {
        trace!("write_args(): {args}");
        let args_bytes: &[u8] = args.as_bytes();

        let initrd_end: usize = match self.initrd {
            Some((initrd_base, initrd_size)) => initrd_base + initrd_size,
            None => {
                let reason: String = "initrd not loaded".to_string();
                error!("write_args(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        let (ptr, size) = (vmem.get_raw_ptr(), vmem.get_size());

        if initrd_end + mem::size_of::<u8>() + args_bytes.len() > size {
            let reason: String = "not enough space to write command line arguments".to_string();
            error!("write_args(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            trace!(
                "write_args(): initrd_end={initrd_end:#010x}, args_bytes_len={:?}, \
                 args_bytes={args_bytes:?}",
                args_bytes.len(),
            );

            let args_len: u8 = match u8::try_from(args_bytes.len()) {
                Ok(v) => v,
                Err(_) => {
                    let reason: String =
                        format!("command line arguments too long (len={})", args_bytes.len());
                    error!("write_args(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };

            ptr::copy_nonoverlapping(&args_len, ptr.add(initrd_end), 1);
            ptr::copy_nonoverlapping(
                args_bytes.as_ptr(),
                ptr.add(initrd_end + mem::size_of::<u8>()),
                args_bytes.len(),
            );
        }

        Ok(())
    }

    /// Resets the value of the credits control register.
    fn reset_credits(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("reset_credits()");
        self.credits = 0;
        vmem.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64,
            &self.credits.to_le_bytes(),
        )
    }

    ///
    /// # Description
    ///
    /// Resets the virtual machine.
    ///
    pub fn reset(&mut self, vmem: &mut VirtualMemory, vcpu: &mut VirtualProcessor) -> Result<()> {
        trace!("reset(): {:?}", &self);
        let rax: u64 = ::config::microvm::DEFAULT_BOOT_MAGIC as u64;

        self.reset_credits(vmem)?;

        // Check if initrd is too large.
        let nzeros: usize = ::config::microvm::DEFAULT_INITRD_BASE.trailing_zeros() as usize;
        let max_initrd_size: usize = (1 << 12) * ((1 << nzeros) - 1);
        if let Some((_, initrd_size)) = self.initrd
            && initrd_size > max_initrd_size
        {
            return Err(anyhow::anyhow!(
                "initrd is too large (initrd_size={initrd_size}, \
                 max_initrd_size={max_initrd_size:?})",
            ));
        }

        // Retrieve initrd information.
        let (initrd_base, initrd_size): (u64, u64) = match self.initrd {
            Some((base, size)) => {
                assert_eq!(base % PAGE_SIZE, 0, "initrd base is not aligned to page size");
                assert_eq!(size % PAGE_SIZE, 0, "initrd size is not aligned to page size");

                let base: u64 = match u64::try_from(base) {
                    Ok(v) => v,
                    Err(_) => {
                        let reason: String = format!("invalid initrd base address ({base:#010x})");
                        error!("reset(): {reason}");
                        return Err(anyhow::anyhow!(reason));
                    },
                };
                let size: u64 = match u64::try_from(size) {
                    Ok(v) => v,
                    Err(_) => {
                        let reason: String = format!("invalid initrd size ({size:#010x})");
                        error!("reset(): {reason}");
                        return Err(anyhow::anyhow!(reason));
                    },
                };

                (base, size)
            },
            None => (0, 0),
        };

        let rbx: u64 =
            (initrd_base & !((1 << nzeros) - 1)) | ((initrd_size >> 12) & ((1 << nzeros) - 1));

        vcpu.reset(self.entry as u64, rax, rbx)
    }

    /// Adds a credit to the credits control register.
    pub fn add_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("add_credits()");
        if self.credits == u32::MAX {
            let reason: String = "credits overflow".to_string();
            error!("add_credits(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        self.credits += 1;
        vmem.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64,
            &self.credits.to_le_bytes(),
        )
    }

    /// Consumes a credit from the credits control register.
    pub fn consume_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("consume_credits()");
        if self.credits == 0 {
            let reason: String = "no credits available".to_string();
            error!("consume_credits(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        self.credits -= 1;
        vmem.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64,
            &self.credits.to_le_bytes(),
        )
    }

    pub fn pause_vm(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("pause_vm()");
        vmem.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as u64,
            &::config::microvm::PAUSE_REQUEST.to_le_bytes(),
        )
    }

    pub fn resume_vm(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("resume_vm()");
        vmem.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as u64,
            &::config::microvm::RUNNING.to_le_bytes(),
        )
    }

    /// Saves the guest state for snapshot serialization.
    pub fn save_state(&self) -> Result<GuestState> {
        Ok(GuestState {
            kernel: self.kernel,
            initrd: self.initrd,
            credits: self.credits,
            entry: self.entry,
        })
    }

    /// Restores the guest state from a snapshot.
    pub fn restore_state(&mut self, state: &GuestState) -> Result<()> {
        self.kernel = state.kernel;
        self.initrd = state.initrd;
        self.credits = state.credits;
        self.entry = state.entry;
        Ok(())
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    elf,
    pal::FileMapping,
    vmm::kvm::{
        vcpu::VirtualProcessor,
        vmem::VirtualMemory,
    },
};
use ::anyhow::Result;
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    mem,
    ptr,
};
use arch::mem::PAGE_SIZE;
use syslog::{
    debug,
    error,
    trace,
};

//==================================================================================================
// Structures
//==================================================================================================

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

#[derive(Serialize, Deserialize)]
pub struct GuestState {
    // Kernel location and size.
    kernel: Option<(usize, usize)>,
    // Initial RAM disk location and size.
    initrd: Option<(usize, usize)>,
    // Control register used to inform the guest about the number of messages ready to be consumed.
    credits: u32,
    // Entry point of the guest.
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

        let elf: FileMapping = FileMapping::mmap(kernel_filename)?;
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
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns a tuple with the base address and size of
    /// the initial RAM disk that was loaded into the virtual memory. Otherwise, it returns an
    /// error.
    ///
    pub fn load_initrd(
        &mut self,
        vmem: &mut VirtualMemory,
        initrd_filename: &str,
        initrd_args: Option<String>,
    ) -> Result<()> {
        trace!("load_initrd(): initrd_filename={}, initrd_args={:?}", initrd_filename, initrd_args);

        debug!("load_initrd(): mapping initrd file");
        let initrd: FileMapping = FileMapping::mmap(initrd_filename)?;

        // Check if initrd would overlap with kernel.
        if let Some((kernel_base, kernel_size)) = self.kernel {
            if (initrd.ptr() as usize) < (kernel_base + kernel_size) {
                let reason: String = "initrd overlaps with kernel".to_string();
                error!("load_initrd(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }
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
            let src = initrd.ptr();
            let dst = ptr.add(::config::microvm::DEFAULT_INITRD_BASE);

            // Check if pointers are valid.
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
        let initrd_size: usize = if initrd.size() % PAGE_SIZE != 0 {
            debug!("load_initrd(): aligning initrd size to page size");
            initrd.size() + (PAGE_SIZE - (initrd.size() % PAGE_SIZE))
        } else {
            initrd.size()
        };

        self.initrd = Some((::config::microvm::DEFAULT_INITRD_BASE, initrd_size));

        // Write arguments to the virtual machine. For now, just pass the initrd filename.
        let mut args: String = initrd_filename
            .split('/')
            .next_back()
            .unwrap_or(initrd_filename)
            .to_string();

        // Add initrd arguments if provided.
        if let Some(ref initrd_args) = initrd_args {
            args.push_str(&format!(" {initrd_args}"));
        }

        debug!("load_initrd(): writing args to virtual memory: {}", args);
        self.write_args(vmem, &args)?;

        Ok(())
    }

    /// # Description
    ///
    /// Writes command line arguments into the virtual memory, right after the initrd file.
    ///
    /// # Parameters
    ///
    /// - `args`: Command line arguments to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
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

        let (ptr, size) = { (vmem.get_raw_ptr(), vmem.get_size()) };

        // Check if there is enough space to write the arguments.
        if initrd_end + mem::size_of::<u8>() + args_bytes.len() > size {
            let reason: String = "not enough space to write command line arguments".to_string();
            error!("write_args(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Write the command line arguments into the virtual memory.
        unsafe {
            // Write length of command line arguments.

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
            // Write command line arguments.
            ptr::copy_nonoverlapping(
                args_bytes.as_ptr(),
                ptr.add(initrd_end + mem::size_of::<u8>()),
                args_bytes.len(),
            );
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resets the value of the credits control register.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
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
    /// # Parameters
    ///
    /// - `rip`: Entry point of the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn reset(&mut self, vmem: &mut VirtualMemory, vcpu: &mut VirtualProcessor) -> Result<()> {
        trace!("reset(): {:?}", &self);
        let rax: u64 = ::config::microvm::DEFAULT_BOOT_MAGIC as u64;

        self.reset_credits(vmem)?;

        // Check if initrd is too large.
        let nzeros: usize = ::config::microvm::DEFAULT_INITRD_BASE.trailing_zeros() as usize;
        let max_initrd_size: usize = (1 << 12) * ((1 << nzeros) - 1);
        if let Some((_, initrd_size)) = self.initrd {
            if initrd_size > max_initrd_size {
                return Err(anyhow::anyhow!(
                    "initrd is too large (initrd_size={initrd_size}, \
                     max_initrd_size={max_initrd_size:?})",
                ));
            }
        }

        // Retrieve initrd information.
        let (initrd_base, initrd_size): (u64, u64) = match self.initrd {
            Some((base, size)) => {
                // Ensure that the initrd base and size are aligned to page size boundaries.
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

        // Encode initrd location and size:
        // - Lower bits encode the size in 4KB pages
        // - Higher bits encode the base address
        let rbx: u64 =
            (initrd_base & !((1 << nzeros) - 1)) | ((initrd_size >> 12) & ((1 << nzeros) - 1));

        vcpu.reset(self.entry as u64, rax, rbx)
    }

    ///
    /// # Description
    ///
    /// Adds a credit to the credits control register.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn add_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("add_credits()");
        // Check for overflow.
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

    ///
    /// # Description
    ///
    /// Consumes a credit from the credits control register.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn consume_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        trace!("consume_credits()");
        // Check for overflow.
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

    pub fn save_state(&self) -> Result<GuestState> {
        trace!("save_state()");
        Ok(GuestState {
            kernel: self.kernel,
            initrd: self.initrd,
            credits: self.credits,
            entry: self.entry,
        })
    }

    pub fn restore_state(&mut self, state: &GuestState) -> Result<()> {
        trace!("restore_state()");
        self.kernel = state.kernel;
        self.initrd = state.initrd;
        self.credits = state.credits;
        self.entry = state.entry;
        Ok(())
    }
}

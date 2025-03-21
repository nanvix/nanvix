// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    elf,
    vmm::microvm::{
        kvm::partition::VirtualPartition,
        pal::FileMapping,
    },
};
use ::anyhow::Result;
use ::kvm_bindings::kvm_userspace_memory_region;
use ::std::{
    mem,
    ptr::{
        self,
    },
    sync::{
        Arc,
        Mutex,
    },
};
use ::sys::arch::mem::PAGE_SIZE;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents the memory of a virtual machine.
///
pub struct VirtualMemory {
    /// Underlying virtual partition.
    partition: Arc<Mutex<VirtualPartition>>,
    /// Virtual memory.
    ptr: *mut u8,
    /// Size of the virtual memory.
    size: usize,
    /// Kernel location and size.
    kernel: Option<(u64, usize)>,
    /// Initial RAM disk location and size.
    _initrd: Option<(u64, usize)>,
    /// Control register used to inform the guest about the number of messages ready to be consumed.
    credits: u32,
}

unsafe impl Send for VirtualMemory {}
unsafe impl Sync for VirtualMemory {}

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualMemory {
    ///
    /// # Description
    ///
    /// Creates a new virtual memory.
    ///
    /// # Parameters
    ///
    /// - `partition`: Virtual partition that hosts the virtual machine.
    /// - `memory_size`: Size of the virtual memory.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the function returns the new virtual memory. Otherwise, it
    /// returns an error.
    ///
    pub fn new(partition: Arc<Mutex<VirtualPartition>>, memory_size: usize) -> Result<Self> {
        trace!("new(): memory_size={}", memory_size);
        crate::timer!("vmem_creation");

        // Allocate memory.
        let ptr: *mut u8 = unsafe {
            libc::mmap(
                ptr::null_mut(),
                memory_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_NORESERVE,
                -1,
                0,
            ) as *mut u8
        };

        // Check if we failed to allocate memory for the virtual machine.
        if ptr.is_null() {
            let reason: String = "failed to allocate memory for the virtual machine".to_string();
            error!("new(): {} (memory_size={:?})", reason, memory_size);
            return Err(anyhow::anyhow!(reason));
        }

        // Create virtual memory. If we fail, destructor will free memory.
        let vmem: Self = Self {
            partition,
            ptr,
            size: memory_size,
            kernel: None,
            _initrd: None,
            credits: 0,
        };

        // Map memory into virtual machine.
        let mem_region: kvm_userspace_memory_region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: 0,
            memory_size: memory_size as u64,
            userspace_addr: ptr as u64,
        };
        unsafe {
            vmem.partition
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
                .vm()
                .set_user_memory_region(mem_region)?
        };

        Ok(vmem)
    }

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
    pub fn load_kernel(&mut self, kernel_filename: &str) -> Result<u64> {
        crate::timer!("vmem_load_kernel");
        trace!("load_kernel(): {}", kernel_filename);

        let elf: FileMapping = FileMapping::mmap(kernel_filename)?;
        let (entry, first_address, size): (usize, usize, usize) =
            unsafe { elf::load(self.ptr as *mut ::std::ffi::c_void, elf.ptr(), self.size)? };

        self.kernel = Some((first_address as u64, size));

        Ok(entry as u64)
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
    pub fn load_initrd(&mut self, initrd_filename: &str) -> Result<(u64, usize)> {
        crate::timer!("vmem_load_initrd");
        trace!("load_initrd(): {}", initrd_filename);

        let initrd: FileMapping = FileMapping::mmap(initrd_filename)?;

        // Check if initrd would overlap with kernel.
        if let Some((kernel_base, kernel_size)) = self.kernel {
            if (initrd.ptr() as usize) < (kernel_base as usize + kernel_size) {
                let reason: String = "initrd overlaps with kernel".to_string();
                error!("load_initrd(): {}", reason);
                return Err(anyhow::anyhow!(reason));
            }
        }

        unsafe {
            ptr::copy_nonoverlapping(
                initrd.ptr(),
                self.ptr.add(::config::microvm::DEFAULT_INITRD_BASE),
                initrd.size(),
            );
        }

        // Ensure initrd size is aligned to page size.
        let initrd_size: usize = if initrd.size() % PAGE_SIZE != 0 {
            debug!("load_initrd(): aligning initrd size to page size");
            initrd.size() + (PAGE_SIZE - (initrd.size() % PAGE_SIZE))
        } else {
            initrd.size()
        };

        self._initrd = Some((::config::microvm::DEFAULT_INITRD_BASE as u64, initrd_size));

        Ok((::config::microvm::DEFAULT_INITRD_BASE as u64, initrd_size))
    }

    ///
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
    pub fn write_args(&mut self, args: &str) -> Result<()> {
        trace!("write_args(): {}", args);
        let args_bytes: &[u8] = args.as_bytes();

        let initrd_end: usize = match self._initrd {
            Some((initrd_base, initrd_size)) => initrd_base as usize + initrd_size,
            None => {
                let reason: String = "initrd not loaded".to_string();
                error!("write_args(): {}", reason);
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if there is enough space to write the arguments.
        if initrd_end + mem::size_of::<u8>() + args_bytes.len() > self.size {
            let reason: String = "not enough space to write command line arguments".to_string();
            error!("write_args(): {}", reason);
            return Err(anyhow::anyhow!(reason));
        }

        // Write the command line arguments into the virtual memory.
        unsafe {
            // Write length of command line arguments.

            trace!(
                "write_args(): initrd_end={:#010x}, args_bytes_len={:?}, args_bytes={:?}",
                initrd_end,
                args_bytes.len(),
                args_bytes
            );

            ptr::copy_nonoverlapping(&(args_bytes.len() as u8), self.ptr.add(initrd_end), 1);
            // Write command line arguments.
            ptr::copy_nonoverlapping(
                args_bytes.as_ptr(),
                self.ptr.add(initrd_end + mem::size_of::<u8>()),
                args_bytes.len(),
            );
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes bytes into the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address in the virtual memory.
    /// - `data`: Data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<()> {
        // Check if region lies within the virtual memory.
        if addr as usize + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={:#010x})", addr);
            error!("write_bytes(): {}", reason);
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.offset(addr as isize), data.len());
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
    pub fn reset_credits(&mut self) -> Result<()> {
        trace!("reset_credits()");
        self.credits = 0;
        self.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64,
            &self.credits.to_le_bytes(),
        )
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
    pub fn add_credit(&mut self) -> Result<()> {
        trace!("add_credits()");
        // Check for overflow.
        if self.credits == u32::MAX {
            let reason: String = "credits overflow".to_string();
            error!("add_credits(): {}", reason);
            return Err(anyhow::anyhow!(reason));
        }

        self.credits += 1;

        self.write_bytes(
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
    pub fn consume_credit(&mut self) -> Result<()> {
        trace!("consume_credits()");
        // Check for overflow.
        if self.credits == 0 {
            let reason: String = "no credits available".to_string();
            error!("consume_credits(): {}", reason);
            return Err(anyhow::anyhow!(reason));
        }

        self.credits -= 1;

        self.write_bytes(
            config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64,
            &self.credits.to_le_bytes(),
        )
    }

    ///
    /// # Description
    ///
    /// Reads bytes from the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address in the virtual memory.
    /// - `data`: Data to read.
    /// - `data`: Data to read.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn read_bytes(&self, addr: u64, data: &mut [u8]) -> Result<()> {
        // Check if region lies within the virtual memory.
        if addr as usize + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={:#010x})", addr);
            error!("read_bytes(): {}", reason);
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ptr::copy_nonoverlapping(self.ptr.offset(addr as isize), data.as_mut_ptr(), data.len());
        }

        Ok(())
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        unsafe {
            let ret: libc::c_int = libc::munmap(self.ptr as *mut libc::c_void, self.size);
            if ret != 0 {
                error!("munmap() failed (ret={})", ret);
            }
        }
    }
}

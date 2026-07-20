// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Guest uses pointer-to-usize casts for address arithmetic.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::vmm::kvm::{
    vcpu::VirtualProcessor,
    vmem::VirtualMemory,
};
#[cfg(target_os = "windows")]
use crate::vmm::microvm::whp::{
    vcpu::VirtualProcessor,
    vmem::VirtualMemory,
};
use crate::{
    elf,
    pal::FileMapping,
};
use ::anyhow::Result;
use ::config::system::{
    CmdlineArgsLen,
    MAX_CMDLINE_ARGS_LEN,
};
use ::log::{
    debug,
    error,
    trace,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::ptr;
use arch::mem::PAGE_SIZE;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Default)]
pub struct Guest {
    /// Kernel location and size.
    kernel: Option<(usize, usize)>,
    /// Initial RAM disk location and size.
    initrd: Option<(usize, usize)>,
    /// Whether the initrd is file-backed (zero-copy mapped) rather than copied.
    /// When `true`, EPT population skips the initrd to avoid copy-on-write faults.
    #[cfg(target_os = "windows")]
    initrd_file_backed: bool,
    /// Control register used to inform the guest about the number of messages ready to be consumed.
    credits: u32,
    /// Entry point of the guest.
    entry: usize,
}

///
/// # Description
///
/// Holds the prepared state of an initrd file for deferred zero-copy mapping on Windows.
///
/// Instead of copying the initrd into guest memory immediately, the file is opened and
/// validated, and the remap is deferred so it can be combined with the RAMFS remap into a
/// single `remap_files_at()` call.
///
#[cfg(target_os = "windows")]
pub struct PreparedInitrd {
    /// Open file handle for the initrd (must stay alive for the file-backed mapping).
    pub file: ::std::fs::File,
    /// Guest-physical base address where the initrd will be mapped.
    pub base: usize,
    /// Page-rounded size of the initrd in guest memory.
    pub size_rounded: usize,
    /// Command-line arguments to write after the remap, or `None` for multibinary images.
    pub args: Option<String>,
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

        #[cfg(target_os = "linux")]
        let elf: FileMapping = FileMapping::mmap(kernel_filename)?;
        #[cfg(target_os = "windows")]
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
        #[cfg(target_os = "linux")]
        let initrd: FileMapping = FileMapping::mmap(initrd_filename)?;
        #[cfg(target_os = "windows")]
        let initrd: FileMapping = FileMapping::open(initrd_filename)?;

        // Check if initrd would overlap with kernel.
        if let Some((kernel_base, kernel_size)) = self.kernel
            && (::config::microvm::DEFAULT_INITRD_BASE) < (kernel_base + kernel_size)
        {
            let reason: String = "initrd overlaps with kernel".to_string();
            error!("load_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Check if initrd would overlap with user mmap region.
        let initrd_end: usize = ::config::microvm::DEFAULT_INITRD_BASE
            .checked_add(initrd.size())
            .ok_or_else(|| {
                let reason: String = "initrd bounds overflow".to_string();
                error!("load_initrd(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        if initrd_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
            let reason: String = "initrd overlaps with user mmap region".to_string();
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

        // SAFETY: FileMapping guarantees that ptr() is valid for size() bytes.
        let mapped_bytes: &[u8] =
            unsafe { ::std::slice::from_raw_parts(initrd.ptr(), initrd.size()) };

        // Detect whether this is a multibinary NVMB image or a single ELF binary.
        let is_multibinary: bool = initrd.size() >= ::multibin::MAGIC.len()
            && mapped_bytes[..::multibin::MAGIC.len()] == ::multibin::MAGIC;

        // Copy the initrd file into guest memory (zero-copy from mmap).
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
        let initrd_size: usize = if !initrd.size().is_multiple_of(PAGE_SIZE) {
            debug!("load_initrd(): aligning initrd size to page size");
            initrd.size() + (PAGE_SIZE - (initrd.size() % PAGE_SIZE))
        } else {
            initrd.size()
        };

        self.initrd = Some((::config::microvm::DEFAULT_INITRD_BASE, initrd_size));

        if is_multibinary {
            // Multibinary image: cmdlines are embedded in the image, no write_args needed.
            debug!("load_initrd(): multibinary format detected, skipping write_args");
        } else {
            // Single ELF binary: write length-prefixed args after the initrd.
            let mut args: String = ::std::path::Path::new(initrd_filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(initrd_filename)
                .to_string();

            // Add initrd arguments if provided.
            if let Some(ref initrd_args) = initrd_args {
                args.push_str(&format!(" {initrd_args}"));
            }

            debug!("load_initrd(): writing args to virtual memory: {}", args);
            self.write_args(vmem, &args)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Prepares an initrd file for deferred zero-copy mapping on Windows.
    ///
    /// Opens the file, validates placement constraints, and records the initrd region in the
    /// guest state, but does NOT copy data or write arguments. The returned [`PreparedInitrd`]
    /// holds the open file handle and metadata needed to include the initrd in a subsequent
    /// combined `remap_files_at()` call alongside the RAMFS.
    ///
    /// After the remap, the caller must write the initrd arguments via [`Self::write_args()`]
    /// and keep the file handle alive for the VM's lifetime.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory (used only for bounds checking).
    /// - `initrd_filename`: Path to the initial RAM disk.
    /// - `initrd_args`: Optional command line arguments for the initrd program.
    ///
    /// # Returns
    ///
    /// Upon success, returns a [`PreparedInitrd`] with the open file and metadata.
    ///
    #[cfg(target_os = "windows")]
    pub fn prepare_initrd(
        &mut self,
        vmem: &VirtualMemory,
        initrd_filename: &str,
        initrd_args: Option<String>,
    ) -> Result<PreparedInitrd> {
        use ::std::io::{
            Read,
            Seek,
            SeekFrom,
        };

        trace!(
            "prepare_initrd(): initrd_filename={}, initrd_args={:?}",
            initrd_filename, initrd_args
        );

        let mut file: ::std::fs::File = ::std::fs::OpenOptions::new()
            .read(true)
            .open(initrd_filename)
            .map_err(|e| {
                let reason: String = format!("failed to open initrd file (error={e})");
                error!("prepare_initrd(): {reason} (filename={initrd_filename})");
                anyhow::anyhow!(reason)
            })?;

        let file_size: usize = usize::try_from(
            file.metadata()
                .map_err(|e| {
                    let reason: String = format!("failed to get initrd metadata (error={e})");
                    error!("prepare_initrd(): {reason}");
                    anyhow::anyhow!(reason)
                })?
                .len(),
        )
        .map_err(|_| {
            let reason: &str = "initrd file size exceeds addressable range";
            error!("prepare_initrd(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        if file_size == 0 {
            let reason: &str = "cannot map zero-sized initrd";
            error!("prepare_initrd(): {reason}");
            anyhow::bail!(reason);
        }

        // Check if initrd would overlap with kernel.
        if let Some((kernel_base, kernel_size)) = self.kernel
            && (::config::microvm::DEFAULT_INITRD_BASE) < (kernel_base + kernel_size)
        {
            let reason: String = "initrd overlaps with kernel".to_string();
            error!("prepare_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Check if initrd would overlap with user mmap region.
        let initrd_end: usize = ::config::microvm::DEFAULT_INITRD_BASE
            .checked_add(file_size)
            .ok_or_else(|| {
                let reason: String = "initrd bounds overflow".to_string();
                error!("prepare_initrd(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        if initrd_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
            let reason: String = "initrd overlaps with user mmap region".to_string();
            error!("prepare_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Check if initrd fits into virtual memory.
        let vm_size: usize = vmem.get_size();
        if (::config::microvm::DEFAULT_INITRD_BASE + file_size) > vm_size {
            let reason: String = "initrd does not fit into virtual memory".to_string();
            error!("prepare_initrd(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Detect whether this is a multibinary NVMB image by reading the file header.
        let mut header_buf: [u8; 8] = [0u8; 8];
        let is_multibinary: bool = if file_size >= ::multibin::MAGIC.len() {
            file.read_exact(&mut header_buf[..::multibin::MAGIC.len()])
                .map_err(|e| {
                    let reason: String = format!("failed to read initrd header (error={e})");
                    error!("prepare_initrd(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            let result: bool = header_buf[..::multibin::MAGIC.len()] == ::multibin::MAGIC;
            // Seek back to start — file position is irrelevant for CreateFileMappingW but
            // we reset it for hygiene.
            let _ = file.seek(SeekFrom::Start(0));
            result
        } else {
            false
        };

        // Compute page-rounded size for guest memory.
        let size_rounded: usize =
            file_size
                .checked_next_multiple_of(PAGE_SIZE)
                .ok_or_else(|| {
                    let reason: String =
                        "initrd size overflows when rounded to page boundary".to_string();
                    error!("prepare_initrd(): {reason}");
                    anyhow::anyhow!(reason)
                })?;

        self.initrd = Some((::config::microvm::DEFAULT_INITRD_BASE, size_rounded));
        self.initrd_file_backed = true;

        // Build args string (same logic as load_initrd).
        let args: Option<String> = if is_multibinary {
            debug!("prepare_initrd(): multibinary format detected, skipping args");
            None
        } else {
            let mut args_str: String = ::std::path::Path::new(initrd_filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(initrd_filename)
                .to_string();
            if let Some(ref initrd_args) = initrd_args {
                args_str.push_str(&format!(" {initrd_args}"));
            }
            Some(args_str)
        };

        debug!(
            "prepare_initrd(): prepared (base={:#x}, file_size={}, size_rounded={}, \
             is_multibinary={})",
            ::config::microvm::DEFAULT_INITRD_BASE,
            file_size,
            size_rounded,
            is_multibinary
        );

        Ok(PreparedInitrd {
            file,
            base: ::config::microvm::DEFAULT_INITRD_BASE,
            size_rounded,
            args,
        })
    }

    ///
    /// # Description
    ///
    /// Returns the base address and size (in bytes) of the initrd currently loaded in memory.
    ///
    /// # Returns
    ///
    /// `Some((base, size))` if an initrd is present, or `None` otherwise.
    ///
    pub fn initrd_region(&self) -> Option<(usize, usize)> {
        self.initrd
    }

    ///
    /// # Description
    ///
    /// Returns the base address and size (in bytes) of the kernel image loaded in memory.
    ///
    /// # Returns
    ///
    /// `Some((base, size))` if the kernel has been loaded, or `None` otherwise.
    ///
    pub fn kernel_region(&self) -> Option<(usize, usize)> {
        self.kernel
    }

    ///
    /// # Description
    ///
    /// Computes GPA ranges that should be pre-populated in the EPT/SLAT before guest execution.
    /// The returned ranges cover the kernel image and initrd.
    ///
    /// Each region is listed individually so that future memory layout changes do not silently
    /// leave a region un-populated.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the list of `(gpa, size)` ranges.
    /// Otherwise, it returns an error.
    ///
    pub fn ept_populate_ranges(&self) -> Result<Vec<(u64, u64)>> {
        // Each region is listed explicitly for defensive programming so that future memory
        // layout changes do not silently leave a region un-populated.
        let mut ranges: Vec<(u64, u64)> = Vec::new();

        // Helper: align a (base, size) pair to page boundaries.
        // Rounds base down and extends size to cover the original range after alignment.
        let page_align = |base: usize, size: usize| -> (u64, u64) {
            let aligned_base: usize = base & !(PAGE_SIZE - 1);
            let end: usize = (base + size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            (aligned_base as u64, (end - aligned_base) as u64)
        };

        if let Some((base, size)) = self.kernel {
            ranges.push(page_align(base, size));
        }

        // Skip file-backed initrd on Windows: write-based EPT population would trigger
        // copy-on-write for every PAGE_WRITECOPY page. File-backed initrd pages are
        // pre-warmed separately with read-only EPT population.
        #[cfg(target_os = "windows")]
        if let Some((base, size)) = self.initrd
            && !self.initrd_file_backed
        {
            ranges.push(page_align(base, size));
        }

        #[cfg(not(target_os = "windows"))]
        if let Some((base, size)) = self.initrd {
            ranges.push(page_align(base, size));
        }

        Ok(ranges)
    }

    /// Returns GPA ranges for file-backed initrd pages that should be EPT-populated with
    /// **read-only** access. This pre-faults the pages from the file cache without triggering
    /// copy-on-write on the PAGE_WRITECOPY mapping.
    #[cfg(target_os = "windows")]
    pub fn ept_populate_read_ranges(&self) -> Vec<(u64, u64)> {
        let page_align = |base: usize, size: usize| -> (u64, u64) {
            let aligned_base: usize = base & !(PAGE_SIZE - 1);
            let end: usize = (base + size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            (aligned_base as u64, (end - aligned_base) as u64)
        };

        if self.initrd_file_backed
            && let Some((base, size)) = self.initrd
        {
            return vec![page_align(base, size)];
        }
        Vec::new()
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
    pub fn write_args(&mut self, vmem: &mut VirtualMemory, args: &str) -> Result<()> {
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

        // Compute end of the args region with checked arithmetic to prevent overflow.
        let args_region_end: usize = initrd_end
            .checked_add(CmdlineArgsLen::WIRE_SIZE)
            .and_then(|v| v.checked_add(args_bytes.len()))
            .ok_or_else(|| {
                let reason: String = "command line arguments region bounds overflow".to_string();
                error!("write_args(): {reason}");
                anyhow::anyhow!(reason)
            })?;

        // Check if there is enough space to write the arguments.
        if args_region_end > size {
            let reason: String = "not enough space to write command line arguments".to_string();
            error!("write_args(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Check that the args region does not overlap with the user mmap region.
        if args_region_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
            let reason: String = format!(
                "command line arguments overlap with user mmap region \
                 (args_end={args_region_end:#010x}, mmap_base={:#010x})",
                ::config::memory_layout::USER_MMAP_BASE_RAW
            );
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

            let args_len: CmdlineArgsLen = match CmdlineArgsLen::new(args_bytes.len()) {
                Some(v) => v,
                None => {
                    let reason: String = format!(
                        "command line arguments too long (len={}, max={})",
                        args_bytes.len(),
                        MAX_CMDLINE_ARGS_LEN
                    );
                    error!("write_args(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };

            let args_len_le: [u8; CmdlineArgsLen::WIRE_SIZE] = args_len.to_le_bytes();
            ptr::copy_nonoverlapping(
                args_len_le.as_ptr(),
                ptr.add(initrd_end),
                CmdlineArgsLen::WIRE_SIZE,
            );
            // Write command line arguments.
            ptr::copy_nonoverlapping(
                args_bytes.as_ptr(),
                ptr.add(initrd_end + CmdlineArgsLen::WIRE_SIZE),
                args_bytes.len(),
            );
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes kernel arguments into the control register page in guest memory.
    ///
    /// The length (u16 LE) is written at
    /// [`config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN`] and the UTF-8 data at
    /// [`config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA`]. Both offsets reside inside
    /// the kernel ELF `.zero` section, which `load_kernel()` zero-fills by default, so these
    /// offsets must be written **after** it. (With the `nightly-performance-optimizations`
    /// feature the loader skips that zeroing and relies on the freshly allocated guest memory
    /// already being zero, but writing after `load_kernel()` remains correct.)
    ///
    /// # Parameters
    ///
    /// - `vmem`: The guest virtual memory.
    /// - `kernel_args`: The kernel arguments string.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_kernel_args(vmem: &mut VirtualMemory, kernel_args: &str) -> Result<()> {
        trace!("write_kernel_args(): {:?}", kernel_args);

        let args_bytes: &[u8] = kernel_args.as_bytes();

        if args_bytes.len() > ::config::microvm::MAX_KERNEL_ARGS_LEN {
            let reason: String = format!(
                "kernel arguments too long (len={}, max={})",
                args_bytes.len(),
                ::config::microvm::MAX_KERNEL_ARGS_LEN
            );
            error!("write_kernel_args(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Write length as u16 LE.
        let len_bytes: [u8; 2] = (args_bytes.len() as u16).to_le_bytes();
        vmem.write_bytes(
            ::config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN as u64,
            &len_bytes,
        )?;

        // Write data.
        if !args_bytes.is_empty() {
            vmem.write_bytes(
                ::config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA as u64,
                args_bytes,
            )?;
        }

        Ok(())
    }

    ///
    /// # Note
    ///
    /// The credits register at [`config::microvm::DEFAULT_MICROVM_CTRL_CREDITS`] (GPA `0x4`)
    /// falls inside the kernel ELF's `.zero` section (`LOAD` segment at GPA `0x0` with
    /// `MemSiz=0x8000`), which `load_kernel()` zero-fills by default. This method — and all
    /// other writes to the control registers at GPA `0x0`–`0x10` — must therefore execute
    /// **after** the ELF has been loaded, so that the VMM-written values are not overwritten.
    /// (With the `nightly-performance-optimizations` feature the loader skips that zeroing and
    /// relies on the freshly allocated guest memory already being zero, but running after
    /// `load_kernel()` remains correct.)
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
        trace!("reset(): {:?}", self);
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

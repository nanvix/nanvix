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
use ::arch::mem::PAGE_SIZE;
use ::kvm_bindings::kvm_userspace_memory_region;
use ::std::{
    fs::File,
    io::{
        Read,
        Write,
    },
    mem,
    path::Path,
    ptr::{
        self,
    },
    slice,
    sync::{
        Arc,
        Mutex,
    },
};
use ::syslog::{
    debug,
    error,
    trace,
};

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
    initrd: Option<(u64, usize)>,
    /// Control register used to inform the guest about the number of messages ready to be consumed.
    credits: u32,
}

///
/// # Description
///
/// A structure that represents the header in virtual memory snapshot files.
///
#[repr(C)]
struct SnapshotHeader {
    /// Memory size (8 bytes): usize
    memory_size: usize,
    /// Kernel base (8 bytes): u64
    kernel_base: u64,
    /// Kernel size (8 bytes): usize
    kernel_size: usize,
    /// Initrd base (8 bytes): u64
    initrd_base: u64,
    /// Initrd size (8 bytes): usize
    initrd_size: usize,
    /// Credits (4 bytes): u32
    credits: u32,
    /// Padding (SNAPSHOT_HEADER_PADDING bytes): zeros
    padding: [u8; Self::SNAPSHOT_HEADER_PADDING],
}

unsafe impl Send for VirtualMemory {}
unsafe impl Sync for VirtualMemory {}

//==================================================================================================
// Constants
//==================================================================================================

const SIZE_OF_HEADER: usize = mem::size_of::<SnapshotHeader>();
// The virtual memory contents come right after the header. If the header is 64-byte aligned, then
// the contents of the virtual memory inside the snapshot file are 64-byte aligned. Reading the
// snapshot file leads to 64-byte aligned data.
static_assert::assert_eq_size!(SnapshotHeader, 64);

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
        trace!("new(): memory_size={memory_size}");
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
            error!("new(): {reason} (memory_size={memory_size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        // Create virtual memory. If we fail, destructor will free memory.
        let vmem: Self = Self {
            partition,
            ptr,
            size: memory_size,
            kernel: None,
            initrd: None,
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
                .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
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
        trace!("load_kernel(): {kernel_filename}");

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
        trace!("load_initrd(): {initrd_filename}");

        let initrd: FileMapping = FileMapping::mmap(initrd_filename)?;

        // Check if initrd would overlap with kernel.
        if let Some((kernel_base, kernel_size)) = self.kernel {
            if (initrd.ptr() as usize) < (kernel_base as usize + kernel_size) {
                let reason: String = "initrd overlaps with kernel".to_string();
                error!("load_initrd(): {reason}");
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

        self.initrd = Some((::config::microvm::DEFAULT_INITRD_BASE as u64, initrd_size));

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
        trace!("write_args(): {args}");
        let args_bytes: &[u8] = args.as_bytes();

        let initrd_end: usize = match self.initrd {
            Some((initrd_base, initrd_size)) => initrd_base as usize + initrd_size,
            None => {
                let reason: String = "initrd not loaded".to_string();
                error!("write_args(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if there is enough space to write the arguments.
        if initrd_end + mem::size_of::<u8>() + args_bytes.len() > self.size {
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
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("write_bytes(): {reason}");
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
            error!("add_credits(): {reason}");
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
            error!("consume_credits(): {reason}");
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
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("read_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ptr::copy_nonoverlapping(self.ptr.offset(addr as isize), data.as_mut_ptr(), data.len());
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Saves the current state of the virtual memory to a snapshot file.
    /// The snapshot includes the memory contents and metadata about the kernel and initrd.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn save_snapshot(&self, path: &Path) -> Result<()> {
        trace!("save_snapshot(): writing to {:?}", path);
        crate::timer!("vmem_save_snapshot");

        let mut file: File = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed creating virtual memory snapshot file (error={e:?})");
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Kernel metadata (guaranteed to exist)
        let (kernel_base, kernel_size) = match self.kernel {
            Some((base, size)) => (base, size),
            None => {
                let reason: &str = "kernel not loaded";
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Initrd metadata (guaranteed to exist)
        let (initrd_base, initrd_size) = match self.initrd {
            Some((base, size)) => (base, size),
            None => {
                let reason: &str = "initrd not loaded";
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let header = SnapshotHeader::new(
            self.size,
            kernel_base,
            kernel_size,
            initrd_base,
            initrd_size,
            self.credits,
        );

        if let Err(e) = file.write_all(header.as_bytes()) {
            let reason: String =
                format!("failed writing the header to virtual memory snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Check alignment. Due to `mmap()` allocation, this should be PAGE_SIZE.
        if self.ptr as usize % PAGE_SIZE != 0 {
            let reason: &str = "memory pointer is not aligned to page size";
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        // SAFETY: The pointer points to the virtual memory, which is `self.size` bytes long. We've
        // just checked alignment.
        let memory_slice: &[u8] = unsafe { slice::from_raw_parts(self.ptr, self.size) };
        // Write the actual memory contents.
        if let Err(e) = file.write_all(memory_slice) {
            let reason: String = format!("failed to write memory contents (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        if let Err(e) = file.sync_all() {
            let reason: String = format!("failed to sync snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        trace!("save_snapshot(): successfully saved snapshot to {:?}", path);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads a virtual memory snapshot from a snapshot file.
    /// This restores the memory contents and metadata.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn load_snapshot(&mut self, path: &Path) -> Result<()> {
        trace!("load_snapshot(): reading from {:?}", path);
        crate::timer!("vmem_load_snapshot");

        let mut file: File = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed opening virtual memory snapshot file (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Read the header
        let mut header_bytes: [u8; SIZE_OF_HEADER] = [0u8; SIZE_OF_HEADER];
        let header: SnapshotHeader = match file.read_exact(&mut header_bytes) {
            Ok(()) => SnapshotHeader::from_bytes(&header_bytes),
            Err(e) => {
                let reason: String = format!(
                    "failed reading header from virtual memory snapshot file (error={e:?})"
                );
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Validate that the memory size matches
        if header.memory_size != self.size {
            let reason: String =
                format!("memory size mismatch: expected {}, got {}", self.size, header.memory_size);
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Read the memory contents
        // SAFETY: We're making a slice of the size of the virtual memory starting at its base.
        let memory_slice: &mut [u8] = unsafe { slice::from_raw_parts_mut(self.ptr, self.size) };
        if let Err(e) = file.read_exact(memory_slice) {
            let reason: String = format!("failed to read memory contents (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        // TODO: validate header checksum matches the checksum computed off of the memory slice https://github.com/nanvix/nanvix/issues/1014

        // Restore metadata
        self.kernel = Some((header.kernel_base, header.kernel_size));
        self.initrd = Some((header.initrd_base, header.initrd_size));
        self.credits = header.credits;

        trace!("load_snapshot(): successfully loaded snapshot from {:?}", path);
        Ok(())
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        unsafe {
            let ret: libc::c_int = libc::munmap(self.ptr as *mut libc::c_void, self.size);
            if ret != 0 {
                error!("munmap() failed (ret={ret})");
            }
        }
    }
}

impl SnapshotHeader {
    /// Padding for alignment. This makes the memory contents of the snapshot also aligned.
    /// Adds up to 64 bytes.
    const SNAPSHOT_HEADER_PADDING: usize = 20;

    fn new(
        memory_size: usize,
        kernel_base: u64,
        kernel_size: usize,
        initrd_base: u64,
        initrd_size: usize,
        credits: u32,
    ) -> Self {
        SnapshotHeader {
            memory_size,
            kernel_base,
            kernel_size,
            initrd_base,
            initrd_size,
            credits,
            padding: [0u8; Self::SNAPSHOT_HEADER_PADDING],
        }
    }

    ///
    /// # Description
    ///
    /// Serializes the snapshot header (which has `repr(C)`) as a slice of bytes.
    ///
    /// # Returns
    ///
    /// A slice of bytes containing the snapshot header.
    ///
    fn as_bytes(&self) -> &[u8; SIZE_OF_HEADER] {
        // SAFETY: Size and alignment are guaranteed by being a `SnapshotHeader` method.
        // The struct has #[repr(C)].
        unsafe { mem::transmute::<&SnapshotHeader, &[u8; SIZE_OF_HEADER]>(self) }
    }

    ///
    /// # Description
    ///
    /// Deserializes a snapshot header from a slice of bytes.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Slice of bytes containing the snapshot header.
    ///
    /// # Returns
    ///
    /// The type-safe form of the header.
    ///
    fn from_bytes(bytes: &[u8; SIZE_OF_HEADER]) -> Self {
        // SAFETY: Size is guaranteed to match SIZE_OF_HEADER.
        // The struct has #[repr(C)] so memory layout is guaranteed.
        // The header is at the beginning of the file, so alignment is guaranteed.
        unsafe { mem::transmute::<[u8; SIZE_OF_HEADER], SnapshotHeader>(*bytes) }
        // TODO: #1014 Add a magic number to the header and check it after deserialization.
    }
}

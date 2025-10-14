// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
};
use ::syslog::{
    error,
    trace,
};
use kvm_ioctls::{
    Kvm,
    VmFd,
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
    /// Virtual memory.
    ptr: *mut u8,
    /// Size of the virtual memory.
    size: usize,
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
static_assert::assert_eq_size!(SnapshotHeader, 8);

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
    pub fn new(kvm_fd: &mut Kvm, vm_fd: &mut VmFd, size: usize) -> Result<Self> {
        trace!("new(): size={size}");

        // Check if the KVM supports the required features.
        let has_sync_mmu_support: bool = kvm_fd.check_extension(kvm_ioctls::Cap::SyncMmu);
        if !has_sync_mmu_support {
            let reason: &str = "sync mmu is not supported";
            error!("new(): {reason}");
            anyhow::bail!(reason);
        }

        // Allocate memory.
        let ptr: *mut u8 = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_NORESERVE,
                -1,
                0,
            )
            .cast::<u8>()
        };

        // Check if we failed to allocate memory for the virtual machine.
        if ptr.is_null() {
            let reason: String = "failed to allocate memory for the virtual machine".to_string();
            error!("new(): {reason} (memory_size={size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        // Create virtual memory. If we fail, destructor will free memory.
        let vmem: Self = Self { ptr, size };

        // Map memory into virtual machine.
        let mem_region: kvm_userspace_memory_region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: 0,
            memory_size: size as u64,
            userspace_addr: ptr as u64,
        };

        unsafe { vm_fd.set_user_memory_region(mem_region)? };

        Ok(vmem)
    }

    pub fn get_raw_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn get_size(&self) -> usize {
        self.size
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
        let addr: usize = match usize::try_from(addr) {
            Ok(v) => v,
            Err(_) => {
                let reason: String = format!("invalid address (addr={addr:#010x})");
                error!("write_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if region lies within the virtual memory.
        if addr + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("write_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(addr), data.len());
        }

        Ok(())
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
        let addr: usize = match usize::try_from(addr) {
            Ok(v) => v,
            Err(_) => {
                let reason: String = format!("invalid address (addr={addr:#010x})");
                error!("read_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if region lies within the virtual memory.
        if addr as usize + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("read_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ptr::copy_nonoverlapping(self.ptr.add(addr), data.as_mut_ptr(), data.len());
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

        let mut file: File = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed creating virtual memory snapshot file (error={e:?})");
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let header = SnapshotHeader::new(self.size);

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

        trace!("load_snapshot(): successfully loaded snapshot from {:?}", path);
        Ok(())
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        unsafe {
            let ret: libc::c_int = libc::munmap(self.ptr.cast::<libc::c_void>(), self.size);
            if ret != 0 {
                error!("munmap() failed (ret={ret})");
            }
        }
    }
}

impl SnapshotHeader {
    fn new(memory_size: usize) -> Self {
        SnapshotHeader { memory_size }
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

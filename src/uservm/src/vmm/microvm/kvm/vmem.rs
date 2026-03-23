// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::ramfs::RamFs;
use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::kvm_bindings::kvm_userspace_memory_region;
use ::log::{
    error,
    trace,
};
use ::std::{
    fs::File,
    io::{
        Read,
        Write,
    },
    mem,
    os::unix::io::AsRawFd,
    path::Path,
    ptr::{
        self,
    },
    slice,
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
    /// Optional RAMFS descriptor that keeps metadata and the backing file alive.
    ramfs: Option<RamFs>,
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

/// The offset within the snapshot file where memory contents begin.
/// This is page-aligned to enable direct MAP_FIXED remapping during restore.
const SNAPSHOT_DATA_OFFSET: usize = PAGE_SIZE;

// Compile-time assertion that the snapshot header fits before the data section.
static_assert::assert_eq!(SIZE_OF_HEADER <= SNAPSHOT_DATA_OFFSET);

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
        let vmem: Self = Self {
            ptr,
            size,
            ramfs: None,
        };

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
    /// Attaches a RAM filesystem descriptor to the virtual memory so the backing file remains
    /// alive for the VM's lifetime.
    ///
    /// # Parameters
    ///
    /// - `ramfs`: RAM filesystem descriptor to keep alive.
    ///
    /// # Returns
    ///
    /// This method always succeeds.
    ///
    pub fn attach_ramfs(&mut self, ramfs: RamFs) {
        self.ramfs = Some(ramfs);
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

        // Pad to SNAPSHOT_DATA_OFFSET so the memory contents start at a page boundary.
        // NOTE: The compile-time assertion at module level guarantees
        // SIZE_OF_HEADER <= SNAPSHOT_DATA_OFFSET, so this subtraction cannot underflow.
        let padding: [u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER] =
            [0u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER];
        if let Err(e) = file.write_all(&padding) {
            let reason: String = format!("failed writing padding to snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Check alignment. Due to `mmap()` allocation, this should be PAGE_SIZE.
        if !(self.ptr as usize).is_multiple_of(PAGE_SIZE) {
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

        let file: File = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed opening virtual memory snapshot file (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Read the header.
        let mut header_bytes: [u8; SIZE_OF_HEADER] = [0u8; SIZE_OF_HEADER];
        if let Err(e) = (&file).read_exact(&mut header_bytes) {
            let reason: String =
                format!("failed reading header from virtual memory snapshot file (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        let header: SnapshotHeader = SnapshotHeader::from_bytes(&header_bytes);

        // Validate that the memory size matches.
        if header.memory_size != self.size {
            let reason: String =
                format!("memory size mismatch: expected {}, got {}", self.size, header.memory_size);
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Validate that the snapshot file is large enough for the header plus all memory contents.
        // If the file is truncated, a MAP_FIXED mmap could later cause SIGBUS on guest access.
        {
            let file_len: u64 = match file.metadata() {
                Ok(m) => m.len(),
                Err(e) => {
                    let reason: String =
                        format!("failed reading snapshot file metadata (error={e:?})");
                    error!("load_snapshot(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            let required: u64 = (SNAPSHOT_DATA_OFFSET + self.size) as u64;
            if file_len < required {
                let reason: String = format!(
                    "snapshot file too small: expected at least {required} bytes, got {file_len}"
                );
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            }
        }

        // Remap the KVM guest memory region directly onto the snapshot file using MAP_FIXED.
        // Memory contents start at SNAPSHOT_DATA_OFFSET (page-aligned) in the file.
        // MAP_PRIVATE gives copy-on-write semantics: only pages the guest writes to are copied.
        // MAP_FIXED atomically replaces the existing anonymous mapping at self.ptr, so the KVM
        // memory region is now backed by the snapshot file with demand paging — only pages the
        // guest accesses are faulted in from disk. If called more than once, each MAP_FIXED call
        // atomically replaces the previous mapping without requiring an explicit munmap.
        let file_offset: libc::off_t = match libc::off_t::try_from(SNAPSHOT_DATA_OFFSET) {
            Ok(v) => v,
            Err(_) => {
                let reason: &str = "snapshot data offset exceeds off_t range";
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let mmap_ptr: *mut u8 = unsafe {
            libc::mmap(
                self.ptr.cast::<libc::c_void>(),
                self.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_FIXED,
                file.as_raw_fd(),
                file_offset,
            )
            .cast::<u8>()
        };

        if mmap_ptr == libc::MAP_FAILED.cast::<u8>() {
            let reason: String = format!(
                "failed to mmap snapshot file with MAP_FIXED (error={})",
                ::std::io::Error::last_os_error()
            );
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        // MAP_FIXED guarantees the returned address equals the requested address.
        debug_assert_eq!(mmap_ptr, self.ptr, "MAP_FIXED should return the exact requested address");
        // TODO: validate header checksum matches the checksum computed off of the memory slice https://github.com/nanvix/nanvix/issues/1014

        trace!("load_snapshot(): successfully loaded snapshot from {:?}", path);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Replaces a slice of guest physical memory with a shared file-backed mapping.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the backing file.
    /// - `base`: Guest physical base address where the file should be mapped.
    /// - `length`: Number of bytes to map.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub fn map_shared_file_region(
        &mut self,
        path: &Path,
        base: usize,
        length: usize,
    ) -> Result<()> {
        trace!("map_shared_file_region(): path={:?}, base={:#010x}, length={length}", path, base);

        if (base & (PAGE_SIZE - 1)) != 0 {
            let reason: String = format!("mapping base is not page-aligned (base={base:#010x})");
            error!("map_shared_file_region(): {reason}");
            anyhow::bail!(reason)
        }

        if (length & (PAGE_SIZE - 1)) != 0 {
            let reason: String = format!("mapping length is not page-aligned (length={length})");
            error!("map_shared_file_region(): {reason}");
            anyhow::bail!(reason)
        }

        if base.saturating_add(length) > self.size {
            let reason: String =
                format!("mapping exceeds guest memory (base={base:#010x}, length={length})");
            error!("map_shared_file_region(): {reason}");
            anyhow::bail!(reason)
        }

        let file: File = File::options()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| anyhow::anyhow!("failed to open shared mapping file {:?}: {e}", path))?;

        let file_len: u64 = file
            .metadata()
            .map_err(|e| anyhow::anyhow!("failed to stat shared mapping file {:?}: {e}", path))?
            .len();
        let required: u64 = length as u64;
        if file_len < required {
            let reason: String = format!(
                "shared mapping file too small: expected at least {required} bytes, got {file_len}"
            );
            error!("map_shared_file_region(): {reason}");
            anyhow::bail!(reason)
        }

        // SAFETY: `base` and `length` were validated above and point into the current KVM userspace
        // mapping. MAP_FIXED atomically replaces only that guest-memory slice with a shared
        // file-backed mapping.
        let dst: *mut libc::c_void = unsafe { self.ptr.add(base).cast::<libc::c_void>() };
        let mapped_ptr: *mut libc::c_void = unsafe {
            libc::mmap(
                dst,
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_FIXED,
                file.as_raw_fd(),
                0,
            )
        };

        if mapped_ptr.is_null() || mapped_ptr == libc::MAP_FAILED || mapped_ptr != dst {
            let reason: String = format!(
                "failed to map shared file into guest memory (path={:?}, error={})",
                path,
                ::std::io::Error::last_os_error()
            );
            error!("map_shared_file_region(): {reason}");
            anyhow::bail!(reason)
        }

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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::std::{
        env,
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    /// Minimum VM memory size used by tests (1 page).
    const TEST_MEM_SIZE: usize = PAGE_SIZE;

    /// Creates a KVM VM with a `VirtualMemory` region for testing.
    /// Returns the `Kvm` and `VmFd` handles alongside the `VirtualMemory` so that the KVM
    /// file descriptors remain open for the lifetime of the test.
    fn create_test_vmem() -> AnyResult<(Kvm, VmFd, VirtualMemory)> {
        let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
        let vmem: VirtualMemory =
            VirtualMemory::new(&mut kvm, &mut vm, TEST_MEM_SIZE).expect("failed to create vmem");
        Ok((kvm, vm, vmem))
    }

    /// Returns a unique temporary file path for snapshot tests.
    fn unique_snapshot_path(suffix: &str) -> PathBuf {
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed to compute timestamp")
            .as_nanos();
        let mut path: PathBuf = env::temp_dir();
        path.push(format!("nanvix-vmem-{suffix}-{nanos}.vmem"));
        path
    }

    /// Verifies that `save_snapshot` creates a file with the expected size.
    #[test]
    fn save_snapshot_creates_correctly_sized_file() -> AnyResult<()> {
        let (_kvm, _vm, vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let path: PathBuf = unique_snapshot_path("save-size");

        vmem.save_snapshot(&path).expect("save_snapshot failed");

        let file_len: u64 = fs::metadata(&path).expect("failed to read metadata").len();
        let expected: u64 = (SNAPSHOT_DATA_OFFSET + TEST_MEM_SIZE) as u64;
        assert_eq!(file_len, expected, "snapshot file should be header + padding + memory");

        fs::remove_file(&path).ok();
        Ok(())
    }

    /// Verifies that a save → load round trip preserves memory contents.
    #[test]
    fn save_load_round_trip_preserves_contents() -> AnyResult<()> {
        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let path: PathBuf = unique_snapshot_path("round-trip");

        // Write a recognizable pattern into guest memory.
        // NOTE: 251 is prime and fits in u8, so (i % 251) is always in [0, 250].
        let pattern: Vec<u8> = (0..TEST_MEM_SIZE)
            .map(|i| u8::try_from(i % 251).expect("i % 251 always fits in u8"))
            .collect();
        vmem.write_bytes(0, &pattern).expect("write_bytes failed");

        // Save snapshot.
        vmem.save_snapshot(&path).expect("save_snapshot failed");

        // Zero out memory so we can confirm load restores it.
        let zeros: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
        vmem.write_bytes(0, &zeros)
            .expect("write_bytes (zero) failed");

        // Load snapshot.
        vmem.load_snapshot(&path).expect("load_snapshot failed");

        // Read back and verify.
        let mut readback: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
        vmem.read_bytes(0, &mut readback)
            .expect("read_bytes failed");
        assert_eq!(readback, pattern, "memory contents should match after save-load round trip");

        fs::remove_file(&path).ok();
        Ok(())
    }

    /// Verifies that `load_snapshot` rejects a file with a mismatched memory size.
    #[test]
    fn load_snapshot_rejects_size_mismatch() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("size-mismatch");

        // Write a snapshot with a different memory size in the header.
        let bad_header: SnapshotHeader = SnapshotHeader::new(TEST_MEM_SIZE * 2);
        let mut file: File = File::create(&path).expect("failed to create file");
        file.write_all(bad_header.as_bytes())
            .expect("failed to write header");
        // Pad to SNAPSHOT_DATA_OFFSET.
        let padding: [u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER] =
            [0u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER];
        file.write_all(&padding).expect("failed to write padding");
        // Write dummy memory contents (matching the *bad* header size).
        let dummy: Vec<u8> = vec![0u8; TEST_MEM_SIZE * 2];
        file.write_all(&dummy)
            .expect("failed to write dummy memory");
        drop(file);

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should reject size mismatch");

        fs::remove_file(&path).ok();
        Ok(())
    }

    /// Verifies that `load_snapshot` rejects a truncated file.
    #[test]
    fn load_snapshot_rejects_truncated_file() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("truncated");

        // Write a valid header but no memory contents.
        let header: SnapshotHeader = SnapshotHeader::new(TEST_MEM_SIZE);
        let mut file: File = File::create(&path).expect("failed to create file");
        file.write_all(header.as_bytes())
            .expect("failed to write header");
        drop(file);

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should reject a truncated file");

        fs::remove_file(&path).ok();
        Ok(())
    }

    /// Verifies that `load_snapshot` fails gracefully when the file does not exist.
    #[test]
    fn load_snapshot_rejects_missing_file() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("missing");

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should fail for a non-existent file");

        Ok(())
    }
}

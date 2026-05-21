// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::AnonymousMapping,
    vmm::microvm::ramfs::RamFs,
};
use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::kvm_bindings::kvm_userspace_memory_region;
use ::log::{
    error,
    trace,
};
use ::std::{
    fs::File,
    io::Write,
    os::unix::io::AsRawFd,
    path::Path,
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
    /// Anonymous memory mapping backing guest physical memory.
    mapping: AnonymousMapping,
    /// Optional RAMFS descriptor that keeps metadata and the backing file alive.
    ramfs: Option<RamFs>,
    /// Additional file handles for multi-image backing files. Kept alive so that
    /// `mmap(MAP_FIXED)` file-backed regions remain valid for the VM's lifetime.
    backing_files: Vec<File>,
}

//==================================================================================================
// Constants
//==================================================================================================

/// The offset within the snapshot file where memory contents begin.
/// This is page-aligned to enable direct MAP_FIXED remapping during restore.
const SNAPSHOT_DATA_OFFSET: usize = PAGE_SIZE;

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
        let mapping: AnonymousMapping = AnonymousMapping::new(size, true)?;

        // Create virtual memory. If we fail, destructor will free memory.
        let vmem: Self = Self {
            mapping,
            ramfs: None,
            backing_files: Vec::new(),
        };

        // Map memory into virtual machine.
        let mem_region: kvm_userspace_memory_region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: 0,
            memory_size: size as u64,
            userspace_addr: vmem.mapping.ptr() as u64,
        };

        unsafe { vm_fd.set_user_memory_region(mem_region)? };

        Ok(vmem)
    }

    pub fn get_raw_ptr(&self) -> *mut u8 {
        self.mapping.ptr()
    }

    pub fn get_size(&self) -> usize {
        self.mapping.size()
    }

    ///
    /// # Description
    ///
    /// Replaces a sub-region of guest memory with a file-backed mapping.
    ///
    /// # Parameters
    ///
    /// - `start`: Byte offset into guest memory (must be page-aligned).
    /// - `file`: File to map from (size must be page-aligned).
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub fn remap_file_at(&mut self, start: usize, file: &File) -> Result<()> {
        let len: usize = {
            let file_len: u64 = file
                .metadata()
                .map_err(|e| {
                    let reason: String = format!("failed to query file metadata (error={e:?})");
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?
                .len();
            usize::try_from(file_len).map_err(|_| {
                let reason: String =
                    format!("file size exceeds platform address space (size={file_len})");
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?
        };
        self.mapping.remap_file_at(start, len, file.as_raw_fd(), 0)
    }

    ///
    /// # Description
    ///
    /// Replaces multiple sub-regions of guest memory with file-backed mappings (zero-copy).
    ///
    /// Each region is specified as a `(guest_offset, file)` pair. The file is mapped at the
    /// given offset using `mmap(MAP_FIXED)`, replacing the anonymous pages in that range.
    ///
    /// # Parameters
    ///
    /// - `regions`: Slice of `(guest_offset, file)` pairs. Must be non-overlapping.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub fn remap_files_at(&mut self, regions: &[(usize, &File)]) -> Result<()> {
        for &(start, file) in regions {
            self.remap_file_at(start, file)?;
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Issues an `madvise` hint for a sub-region of the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `start`: Byte offset from the start of the mapping (must be page-aligned).
    /// - `len`: Size of the region in bytes.
    /// - `advice`: madvise advice constant (e.g., `MADV_SEQUENTIAL`, `MADV_WILLNEED`).
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub fn madvise_at(&self, start: usize, len: usize, advice: i32) -> Result<()> {
        self.mapping.madvise_at(start, len, advice)
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
    /// Attaches multiple backing file handles whose memory-mapped regions must remain valid
    /// for the VM's lifetime. Used by the multi-image RAMFS path where each sub-image file
    /// is mapped individually.
    ///
    /// # Parameters
    ///
    /// - `files`: File handles to keep alive.
    ///
    pub fn attach_backing_files(&mut self, files: Vec<File>) {
        self.backing_files.extend(files);
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
        if addr + data.len() > self.mapping.size() {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("write_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ::std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.mapping.ptr().add(addr),
                data.len(),
            );
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Pre-populates host-side backing pages for the given GPA ranges using
    /// `madvise(MADV_POPULATE_WRITE)`. This faults in host pages before guest execution so that
    /// KVM's EPT fault path only needs to install SLAT entries without also incurring host page
    /// faults, reducing cold-start latency.
    ///
    /// Pre-populating moves page-fault costs to partition setup time where they are measured
    /// separately and do not inflate guest execution latency.
    ///
    /// # Parameters
    ///
    /// - `gpa_ranges`: Slice of `(gpa, size)` pairs. Each GPA and size must be page-aligned
    ///   and the range must lie within the mapped guest RAM. Zero-sized entries are skipped.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn populate_ept(&self, gpa_ranges: &[(u64, u64)]) -> Result<()> {
        trace!("populate_ept(): {} range(s)", gpa_ranges.len());

        let page_size: u64 = PAGE_SIZE as u64;
        let ram_size: u64 = self.mapping.size() as u64;

        for &(gpa, size) in gpa_ranges {
            if size == 0 {
                continue;
            }

            if gpa % page_size != 0 || size % page_size != 0 {
                let reason: String =
                    format!("gpa and size must be page-aligned (gpa={gpa:#x}, size={size:#x})");
                error!("populate_ept(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            if gpa.checked_add(size).is_none_or(|end| end > ram_size) {
                let reason: String = format!(
                    "range exceeds mapped guest RAM (gpa={gpa:#x}, size={size:#x}, \
                     ram_size={ram_size:#x})"
                );
                error!("populate_ept(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            // Compute host virtual address for this GPA offset.
            // SAFETY: bounds checked above — `gpa + size <= ram_size` and `ram_size` equals the
            // mapping length, so `ptr.add(gpa)` through `ptr.add(gpa + size - 1)` are within the
            // allocated region.
            let gpa_offset: usize = usize::try_from(gpa)
                .map_err(|_| anyhow::anyhow!("gpa {gpa:#x} exceeds usize range"))?;
            let host_addr: *mut u8 = unsafe { self.mapping.ptr().add(gpa_offset) };

            // SAFETY: `host_addr` points into the anonymous mapping and the range
            // `[host_addr, host_addr + size)` lies within the mapping (bounds checked above).
            // `MADV_POPULATE_WRITE` faults in pages with write access, ensuring the host kernel
            // allocates backing pages so subsequent KVM EPT faults only need to install SLAT
            // entries.
            let range_len: usize = usize::try_from(size)
                .map_err(|_| anyhow::anyhow!("size {size:#x} exceeds usize range"))?;
            let ret: i32 =
                unsafe { ::libc::madvise(host_addr.cast(), range_len, libc::MADV_POPULATE_WRITE) };
            if ret != 0 {
                let reason: String = format!(
                    "madvise(MADV_POPULATE_WRITE) failed (gpa={gpa:#x}, size={size:#x}, error={})",
                    ::std::io::Error::last_os_error()
                );
                error!("populate_ept(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }
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
        if addr as usize + data.len() > self.mapping.size() {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("read_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        unsafe {
            ::std::ptr::copy_nonoverlapping(
                self.mapping.ptr().add(addr),
                data.as_mut_ptr(),
                data.len(),
            );
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

        let memory_slice: &[u8] = self.mapping.as_slice();

        // Write zero padding so the memory contents start at a page boundary, enabling direct
        // MAP_FIXED remapping during restore.
        let padding: [u8; SNAPSHOT_DATA_OFFSET] = [0u8; SNAPSHOT_DATA_OFFSET];
        if let Err(e) = file.write_all(&padding) {
            let reason: String = format!("failed writing padding to snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Stream memory contents to file.
        for chunk in memory_slice.chunks(64 * 1024) {
            if let Err(e) = file.write_all(chunk) {
                let reason: String = format!("failed to write memory contents (error={e:?})");
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            }
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

        // Verify the snapshot file is large enough to back the entire guest memory mapping.
        // Without this check, mmap() with MAP_FIXED may succeed on a short file and later guest
        // accesses past EOF would raise SIGBUS and crash the UserVM process.
        let required_size: u64 =
            match (SNAPSHOT_DATA_OFFSET as u64).checked_add(self.mapping.size() as u64) {
                Some(v) => v,
                None => {
                    let reason: &str = "snapshot required size overflows u64";
                    error!("load_snapshot(): {reason}");
                    anyhow::bail!(reason)
                },
            };
        let file_len: u64 = match file.metadata() {
            Ok(m) => m.len(),
            Err(e) => {
                let reason: String =
                    format!("failed querying snapshot file metadata (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };
        if file_len < required_size {
            let reason: String = format!(
                "snapshot file too small (size={file_len} bytes, required={required_size} bytes)"
            );
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
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
        self.mapping.remap_file(file.as_raw_fd(), file_offset)?;

        trace!("load_snapshot(): successfully loaded snapshot from {:?}", path);
        Ok(())
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

    /// Verifies that `load_snapshot` fails gracefully when the file does not exist.
    #[test]
    fn load_snapshot_rejects_missing_file() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("missing");

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should fail for a non-existent file");

        Ok(())
    }

    /// Verifies that `load_snapshot` rejects a snapshot file that is smaller than the guest
    /// memory mapping it must back. This guards against MAP_FIXED-on-short-file successes that
    /// would otherwise cause SIGBUS on later guest accesses past EOF. The boundary case
    /// (`required_size - 1`) is exercised to pin the strict `<` comparison in the size check.
    #[test]
    fn load_snapshot_rejects_undersized_file() -> AnyResult<()> {
        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let required_size: usize = SNAPSHOT_DATA_OFFSET + TEST_MEM_SIZE;

        // Case 1: file containing only the data-offset padding, no memory contents.
        let path_empty: PathBuf = unique_snapshot_path("undersized-empty");
        let truncated_empty: Vec<u8> = vec![0u8; SNAPSHOT_DATA_OFFSET];
        fs::write(&path_empty, &truncated_empty).expect("failed to write undersized snapshot file");
        let result_empty: AnyResult<()> = vmem.load_snapshot(&path_empty);
        fs::remove_file(&path_empty).ok();
        assert!(result_empty.is_err(), "load_snapshot should fail for an empty-data file");

        // Case 2: boundary — exactly one byte short of the required size.
        let path_boundary: PathBuf = unique_snapshot_path("undersized-boundary");
        let truncated_boundary: Vec<u8> = vec![0u8; required_size - 1];
        fs::write(&path_boundary, &truncated_boundary)
            .expect("failed to write boundary-sized snapshot file");
        let result_boundary: AnyResult<()> = vmem.load_snapshot(&path_boundary);
        fs::remove_file(&path_boundary).ok();
        assert!(
            result_boundary.is_err(),
            "load_snapshot should fail for a file one byte short of required size"
        );

        Ok(())
    }
}

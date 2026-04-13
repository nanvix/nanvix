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
    io::{
        Read,
        Seek,
        SeekFrom,
        Write,
    },
    mem,
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
}

///
/// # Description
///
/// A structure that represents the header in virtual memory snapshot files.
///
#[repr(C)]
struct SnapshotHeader {
    /// Magic number identifying the file as a Nanvix virtual memory snapshot.
    magic: u64,
    /// Snapshot format version.
    version: u32,
    /// Compression method applied to memory contents (0 = none).
    compression: u32,
    /// Size of the guest memory in bytes.
    memory_size: u64,
    /// FNV-1a checksum of the memory contents.
    checksum: u64,
}

//==================================================================================================
// Constants
//==================================================================================================

const SIZE_OF_HEADER: usize = mem::size_of::<SnapshotHeader>();

/// The offset within the snapshot file where memory contents begin.
/// This is page-aligned to enable direct MAP_FIXED remapping during restore.
const SNAPSHOT_DATA_OFFSET: usize = PAGE_SIZE;

/// Magic number for snapshot files: ASCII "NVXVMEM!" in little-endian byte order.
const SNAPSHOT_MAGIC: u64 = u64::from_le_bytes(*b"NVXVMEM!");

/// Current snapshot format version.
const SNAPSHOT_VERSION: u32 = 1;

/// Compression type: no compression applied.
const COMPRESSION_NONE: u32 = 0;

// Compile-time assertion that the snapshot header fits before the data section.
static_assert::assert_eq!(SIZE_OF_HEADER <= SNAPSHOT_DATA_OFFSET);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes an FNV-1a checksum of the given data.
///
/// # Parameters
///
/// - `data`: Byte slice to checksum.
///
/// # Returns
///
/// A 64-bit FNV-1a hash of `data`.
///
fn compute_checksum(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash: u64 = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

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

        // Check alignment. Due to `mmap()` allocation, this should be PAGE_SIZE.
        if !(self.mapping.ptr() as usize).is_multiple_of(PAGE_SIZE) {
            let reason: &str = "memory pointer is not aligned to page size";
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        let memory_slice: &[u8] = self.mapping.as_slice();

        // Write a placeholder header; the checksum field will be filled in after streaming.
        let placeholder_header = SnapshotHeader::new(self.mapping.size(), 0);
        if let Err(e) = file.write_all(placeholder_header.as_bytes()) {
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

        // Stream memory contents to file while computing the FNV-1a checksum in a single pass,
        // avoiding a separate full traversal for checksum computation.
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x00000100000001B3;
        let mut checksum: u64 = FNV_OFFSET_BASIS;
        for chunk in memory_slice.chunks(64 * 1024) {
            for &byte in chunk {
                checksum ^= byte as u64;
                checksum = checksum.wrapping_mul(FNV_PRIME);
            }
            if let Err(e) = file.write_all(chunk) {
                let reason: String = format!("failed to write memory contents (error={e:?})");
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            }
        }

        // Seek back and write the final header with the computed checksum.
        if let Err(e) = file.seek(SeekFrom::Start(0)) {
            let reason: String =
                format!("failed to seek to beginning of snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }
        let final_header = SnapshotHeader::new(self.mapping.size(), checksum);
        if let Err(e) = file.write_all(final_header.as_bytes()) {
            let reason: String =
                format!("failed to write final header to snapshot file (error={e:?})");
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

        // Validate the magic number.
        if header.magic != SNAPSHOT_MAGIC {
            let reason: String = format!(
                "invalid snapshot magic: expected {:#018x}, got {:#018x}",
                SNAPSHOT_MAGIC, header.magic
            );
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Validate the version number.
        if header.version != SNAPSHOT_VERSION {
            let reason: String = format!(
                "unsupported snapshot version: expected {}, got {}",
                SNAPSHOT_VERSION, header.version
            );
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Validate the compression mode.
        if header.compression != COMPRESSION_NONE {
            let reason: String = format!(
                "unsupported snapshot compression: expected {}, got {}",
                COMPRESSION_NONE, header.compression
            );
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Validate that the memory size matches.
        if header.memory_size != self.mapping.size() as u64 {
            let reason: String = format!(
                "memory size mismatch: expected {}, got {}",
                self.mapping.size(),
                header.memory_size
            );
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
            let required: u64 = (SNAPSHOT_DATA_OFFSET + self.mapping.size()) as u64;
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
        self.mapping.remap_file(file.as_raw_fd(), file_offset)?;

        // Validate that the checksum matches the memory contents.
        // NOTE: this eagerly reads all pages from the snapshot file, trading demand-paging latency
        // for upfront integrity verification.
        let memory_slice: &[u8] = self.mapping.as_slice();
        let actual_checksum: u64 = compute_checksum(memory_slice);
        if actual_checksum != header.checksum {
            let reason: String = format!(
                "checksum mismatch: expected {:#018x}, got {:#018x}",
                header.checksum, actual_checksum
            );
            error!("load_snapshot(): {reason}");

            // The checksum verification failed, but at this point the mapping has already been
            // remapped with MAP_FIXED to the snapshot file. To avoid exposing callers to this
            // partially-applied state, restore a neutral anonymous mapping at the same address.
            if let Err(e) = self.mapping.remap_anonymous() {
                error!("load_snapshot(): failed to restore anonymous mapping ({e})");
            }

            anyhow::bail!(reason)
        }

        trace!("load_snapshot(): successfully loaded snapshot from {:?}", path);
        Ok(())
    }
}

impl SnapshotHeader {
    fn new(memory_size: usize, checksum: u64) -> Self {
        SnapshotHeader {
            magic: SNAPSHOT_MAGIC,
            version: SNAPSHOT_VERSION,
            compression: COMPRESSION_NONE,
            memory_size: memory_size as u64,
            checksum,
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
        let dummy: Vec<u8> = vec![0u8; TEST_MEM_SIZE * 2];
        let bad_header: SnapshotHeader =
            SnapshotHeader::new(TEST_MEM_SIZE * 2, compute_checksum(&dummy));
        let mut file: File = File::create(&path).expect("failed to create file");
        file.write_all(bad_header.as_bytes())
            .expect("failed to write header");
        // Pad to SNAPSHOT_DATA_OFFSET.
        let padding: [u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER] =
            [0u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER];
        file.write_all(&padding).expect("failed to write padding");
        // Write dummy memory contents (matching the *bad* header size).
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
        let header: SnapshotHeader = SnapshotHeader::new(TEST_MEM_SIZE, 0);
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

    /// Verifies that header round-trip serialization preserves all fields.
    #[test]
    fn header_round_trip_serialization() {
        let header: SnapshotHeader =
            SnapshotHeader::new(0x1234_5678_9abc_def0, 0xfeed_face_dead_beef);
        let bytes: &[u8; SIZE_OF_HEADER] = header.as_bytes();
        let restored: SnapshotHeader = SnapshotHeader::from_bytes(bytes);
        assert_eq!(restored.magic, SNAPSHOT_MAGIC);
        assert_eq!(restored.version, SNAPSHOT_VERSION);
        assert_eq!(restored.compression, COMPRESSION_NONE);
        assert_eq!(restored.memory_size, 0x1234_5678_9abc_def0);
        assert_eq!(restored.checksum, 0xfeed_face_dead_beef);
    }

    /// Verifies that `compute_checksum` is deterministic and sensitive to content.
    #[test]
    fn checksum_is_deterministic_and_content_sensitive() {
        let data_a: [u8; 4] = [1, 2, 3, 4];
        let data_b: [u8; 4] = [4, 3, 2, 1];
        let empty: [u8; 0] = [];

        // Same input produces the same checksum.
        assert_eq!(compute_checksum(&data_a), compute_checksum(&data_a));

        // Different inputs produce different checksums.
        assert_ne!(compute_checksum(&data_a), compute_checksum(&data_b));

        // Empty data has a well-defined checksum (FNV-1a offset basis).
        assert_eq!(compute_checksum(&empty), 0xcbf29ce484222325);
    }

    /// Verifies that `load_snapshot` rejects a file with an invalid magic number.
    #[test]
    fn load_snapshot_rejects_bad_magic() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("bad-magic");

        // Write a header with a corrupted magic number.
        let mut header: SnapshotHeader = SnapshotHeader::new(TEST_MEM_SIZE, 0);
        header.magic = 0xBAD0_BAD0_BAD0_BAD0;
        let mut file: File = File::create(&path).expect("failed to create file");
        file.write_all(header.as_bytes())
            .expect("failed to write header");
        let padding: [u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER] =
            [0u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER];
        file.write_all(&padding).expect("failed to write padding");
        let dummy: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
        file.write_all(&dummy)
            .expect("failed to write dummy memory");
        drop(file);

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should reject bad magic");

        fs::remove_file(&path).ok();
        Ok(())
    }

    /// Verifies that `load_snapshot` rejects a file with an unsupported version.
    #[test]
    fn load_snapshot_rejects_bad_version() -> AnyResult<()> {
        let path: PathBuf = unique_snapshot_path("bad-version");

        // Write a header with an unsupported version number.
        let mut header: SnapshotHeader = SnapshotHeader::new(TEST_MEM_SIZE, 0);
        header.version = SNAPSHOT_VERSION + 1;
        let mut file: File = File::create(&path).expect("failed to create file");
        file.write_all(header.as_bytes())
            .expect("failed to write header");
        let padding: [u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER] =
            [0u8; SNAPSHOT_DATA_OFFSET - SIZE_OF_HEADER];
        file.write_all(&padding).expect("failed to write padding");
        let dummy: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
        file.write_all(&dummy)
            .expect("failed to write dummy memory");
        drop(file);

        let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
        let result: AnyResult<()> = vmem.load_snapshot(&path);
        assert!(result.is_err(), "load_snapshot should reject bad version");

        fs::remove_file(&path).ok();
        Ok(())
    }
}

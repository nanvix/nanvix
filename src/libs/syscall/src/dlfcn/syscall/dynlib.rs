// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::safe::{
    mem::segment::MemorySegment,
    FileSystem,
    FileSystemAttributes,
    FileSystemPath,
    FileType,
    RegularFile,
    RegularFileOffset,
    RegularFileOpenFlags,
};
use ::alloc::{
    collections::{
        btree_map::BTreeMap,
        btree_set::BTreeSet,
        vec_deque::VecDeque,
    },
    ffi::CString,
    fmt,
    string::{
        String,
        ToString,
    },
    sync::Arc,
    vec,
    vec::Vec,
};
use ::arch::mem::PAGE_ALIGNMENT;
use ::core::{
    mem,
    sync::atomic::{
        AtomicBool,
        AtomicI32,
        AtomicU32,
        Ordering,
    },
};
use ::elf::{
    GnuHashTable,
    Lookup,
    RelocationEntry,
    RelocationTable,
    RelocationType,
    StringTable,
    Symbol,
    SymbolHashTable,
    SymbolTable,
    SysvHashTable,
};
use ::goblin::elf::{
    Elf,
    SectionHeader,
};
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        self,
        AccessPermission,
        Address,
        VirtualAddress,
    },
};
use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::type_safe::UnalignedPointer;

//==================================================================================================
// DlHandle
//==================================================================================================

///
/// # Description
///
/// A structure that represents a handle to a dynamic library file.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct DlHandle(c_int);

/// First loader-generated library handle.
const FIRST_GENERATED_HANDLE: u32 = 1;

/// Last loader-generated library handle, keeping [`DlHandle::GLOBAL`] reserved.
const LAST_GENERATED_HANDLE: u32 = c_int::MAX as u32 - 1;

/// Monotonically increasing source of loader-generated library handles.
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(FIRST_GENERATED_HANDLE);

impl DlHandle {
    /// Sentinel handle returned by `dlopen(NULL)` representing the global
    /// symbol scope (main executable + pre-loaded libraries). It is drawn from
    /// the very top of the handle space, so it never collides with a
    /// loader-generated handle.
    pub const GLOBAL: Self = DlHandle(c_int::MAX);

    /// Allocates a fresh handle that is unique for the lifetime of the process.
    fn allocate() -> Result<Self, Error> {
        match NEXT_HANDLE.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            if next <= LAST_GENERATED_HANDLE {
                Some(next + 1)
            } else {
                None
            }
        }) {
            Ok(handle) => Ok(DlHandle(handle as c_int)),
            Err(_) => {
                let reason: &str = "dynamic library handle space exhausted";
                ::syslog::warn!("allocate(): {}", reason);
                Err(Error::new(ErrorCode::ValueOverflow, reason))
            },
        }
    }

    /// Casts the target handle to a pointer.
    pub fn as_mut_ptr(&self) -> *mut c_void {
        self.0 as *mut c_void
    }

    /// Casts a mutable pointer to the target handle.
    pub fn from_mut_ptr(ptr: *mut c_void) -> Self {
        DlHandle(ptr as c_int)
    }
}

//==================================================================================================
// InitState
//==================================================================================================

///
/// # Description
///
/// Shared, lock-free construction state for a dynamically loaded library.
///
/// `dlopen` releases [`DYNAMIC_LIBRARY_REGISTRY`](super::DYNAMIC_LIBRARY_REGISTRY)
/// before running a newly loaded library's `.init_array` constructors, so that a
/// constructor may legally call back into the loader (`dlsym`, and — for a
/// re-entrant open of the same library from its own constructor — `dlopen`)
/// without deadlocking. That released window let a *concurrent* `dlopen` of the
/// same filename observe the library as "loaded" on the dedup fast path and
/// return its handle before the constructors had run.
///
/// This state closes the window. The dedup fast path waits on
/// [`constructors_done`](Self::constructors_done) before handing back a handle,
/// while a re-entrant open from the *constructing* thread itself is detected via
/// [`constructor_tid`](Self::constructor_tid) and returns immediately — matching
/// the System V gABI rule that constructors run exactly once, on the loading
/// thread (and avoiding a self-deadlock).
///
/// The state lives behind an [`Arc`] so a waiting `dlopen` can hold on to it and
/// poll it after dropping every dlfcn lock.
///
pub struct InitState {
    /// Set to `true` once this library's `.init_array` constructors have
    /// finished running. Observed with `Acquire` ordering by a concurrent
    /// `dlopen` so the constructors' writes are visible once it returns.
    constructors_done: AtomicBool,
    /// Raw identifier of the thread that is running (or is about to run) this
    /// library's constructors. Valid only while `constructors_done` is `false`;
    /// initialized to [`Self::NO_THREAD`] when no owner is known.
    constructor_tid: AtomicI32,
}

impl InitState {
    /// Sentinel meaning "no constructing thread recorded yet". Distinct from
    /// every real (non-negative) thread identifier.
    const NO_THREAD: i32 = -1;

    /// Creates a fresh state for a library whose constructors have not run.
    fn new() -> Self {
        Self {
            constructors_done: AtomicBool::new(false),
            constructor_tid: AtomicI32::new(Self::NO_THREAD),
        }
    }

    /// Records the thread that will run this library's constructors.
    ///
    /// `dlopen` calls this while `DYNAMIC_LIBRARY_REGISTRY` is held — before the
    /// lock is released and constructors begin — so no concurrent observer can
    /// see the pre-set [`Self::NO_THREAD`] sentinel.
    pub fn set_constructor_thread(&self, tid: i32) {
        self.constructor_tid.store(tid, Ordering::Relaxed);
    }

    /// Marks this library's constructors as finished, releasing any concurrent
    /// `dlopen` blocked in [`wait_until_constructed`](Self::wait_until_constructed).
    pub fn mark_constructed(&self) {
        self.constructors_done.store(true, Ordering::Release);
    }

    /// Blocks until this library's constructors have finished.
    ///
    /// Returns immediately if the constructors are already done, or if the
    /// calling thread (`current_tid`) is the very thread running them — a
    /// re-entrant `dlopen` from a constructor must not wait on itself.
    /// Otherwise it spins, yielding the CPU so the constructing thread is
    /// scheduled, until the constructors complete.
    pub fn wait_until_constructed(&self, current_tid: i32) {
        if self.constructors_done.load(Ordering::Acquire) {
            return;
        }
        if self.constructor_tid.load(Ordering::Relaxed) == current_tid {
            return;
        }
        while !self.constructors_done.load(Ordering::Acquire) {
            // Yield so the constructing thread makes progress; fall back to a
            // spin hint if the scheduler kernel call is unavailable.
            let _ = ::sys::kcall::sched::__kcall_sched_yield();
            ::core::hint::spin_loop();
        }
    }
}

//==================================================================================================
// DlFile
//==================================================================================================

///
/// # Description
///
/// A structure representing a dynamic library file.
///
pub struct DynamicLibrary {
    /// Library name.
    filename: CString,
    /// Stable, loader-generated handle that uniquely identifies this library
    /// for its whole lifetime.
    handle: DlHandle,
    /// Underlying file descriptor.
    fd: RegularFile,
    /// Load address.
    load_address: VirtualAddress,
    /// Memory segments.
    _segments: Vec<MemorySegment>, // Keep this here to prevent memory drop.
    /// Dependencies, in `DT_NEEDED` order.
    ///
    /// Stored as an ordered list rather than a map so symbol resolution can
    /// search dependencies in the exact order the linker recorded them — the
    /// order glibc walks when it builds an object's `l_searchlist` — instead of
    /// an arbitrary alphabetical order.
    dependencies: Vec<(String, Option<Arc<Mutex<Self>>>)>,
    /// Dynamic symbols.
    dynsym: SymbolTable,
    /// Dynamic symbols names.
    dynstr: StringTable,
    /// Symbol-lookup accelerator (`DT_HASH` / `DT_GNU_HASH`), when the object
    /// carries a usable hash section. `None` falls back to a linear `.dynsym`
    /// scan in [`find`](DynamicLibrary::find).
    hash: Option<SymbolHashTable>,
    /// Relocation table for global functions.
    dynplt: Option<RelocationTable>,
    /// Relocation table for global variables.
    dynrel: Option<RelocationTable>,
    /// Absolute address of the `.init_array` section and the number of entries.
    init_array: Option<(usize, usize)>,
    /// Absolute address of the `.fini_array` section and the number of entries.
    fini_array: Option<(usize, usize)>,
    /// `DT_RUNPATH` directories of this library, already split on `:`.
    runpaths: Vec<String>,
    /// Shared construction state, used to serialize concurrent `dlopen` calls
    /// against this library's `.init_array` constructors.
    init_state: Arc<InitState>,
    /// Number of outstanding direct `dlopen()` handles that name this library.
    ///
    /// Mirrors glibc's `l_direct_opencount`: every `dlopen()` that resolves to
    /// this file — including a cache hit that returns an already-loaded handle —
    /// increments it, and every matching `dlclose()` decrements it. The library
    /// only becomes a candidate for unloading once this count reaches zero.
    /// Dependencies pulled in transitively are tracked by their `Arc` edges and
    /// are not reflected here.
    open_count: usize,
}

impl DynamicLibrary {
    /// Opens a dynamic library file.
    pub fn open(filename: &str) -> Result<Self, Error> {
        ::syslog::trace!("open(): filename={}", filename);
        // Attempt to open file.
        let fd: RegularFile = FileSystem::open_regular_file(
            &FileSystemPath::new(filename)?,
            &RegularFileOpenFlags::read_only(),
            None,
        )?;

        // Convert filename to a C string.
        let filename: CString = match CString::new(filename) {
            Ok(cstr) => cstr,
            Err(_) => {
                let reason: &str = "failed to convert filename to C string";
                ::syslog::warn!("open(): {}", reason);
                return Err(Error::new(ErrorCode::BadFile, reason));
            },
        };

        // Retrieve file information.
        let attr: FileSystemAttributes = fd.attributes()?;

        // Check if file is not a regular file.
        if attr.file_type() != FileType::RegularFile {
            let reason: &str = "file is not a regular file";
            ::syslog::warn!("open(): {}", reason);
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

        // Attempt to load file in one shot.
        let file_size: RegularFileOffset = attr.size();
        let mut bytes: Vec<u8> = vec![0; file_size.try_into()?];
        fd.read(&mut bytes)?;

        // Parse ELF file.
        match Elf::parse(&bytes) {
            Ok(elf) => {
                // Check if ELF file is not a dynamic library.
                if !elf.is_lib {
                    let reason: &str = "file is not a dynamic library";
                    ::syslog::warn!("load(): {}", reason);
                    return Err(Error::new(ErrorCode::BadFile, reason));
                }

                // First pass: compute the total size needed for all loadable segments.
                let total_size: usize = Self::compute_load_size(&elf)?;

                // Reserve virtual address space from the unified mmap region.
                let load_address: VirtualAddress = ::sysalloc::vaddr::reserve(total_size)?;
                let mut end_address: VirtualAddress = load_address;

                let mut segments: Vec<MemorySegment> = Vec::new();

                // Second pass: load segments at the reserved address.
                for phdr in elf.program_headers.iter() {
                    // Check if program header is loadable.
                    if phdr.p_type == goblin::elf::program_header::PT_LOAD {
                        // Skip empty loadable segments. They contribute nothing to the
                        // in-memory image, and mapping a zero-capacity region would fail.
                        // Some linkers (e.g. lld at higher optimization levels) emit an empty
                        // PT_LOAD as section-alignment padding between the text and data
                        // segments; a conforming loader ignores such entries. A valid PT_LOAD
                        // satisfies p_filesz <= p_memsz, so p_memsz == 0 implies p_filesz == 0.
                        // Reject the inconsistent p_memsz == 0 && p_filesz != 0 case instead of
                        // silently dropping the on-disk bytes it claims to carry.
                        if phdr.p_memsz == 0 {
                            if phdr.p_filesz != 0 {
                                let reason: &str = "PT_LOAD has p_memsz == 0 but non-zero p_filesz";
                                ::syslog::warn!(
                                    "load(): {reason} (vaddr={:#x}, filesz={})",
                                    phdr.p_vaddr,
                                    phdr.p_filesz
                                );
                                return Err(Error::new(ErrorCode::BadFile, reason));
                            }

                            ::syslog::debug!(
                                "load(): skipping empty PT_LOAD (vaddr={:#x})",
                                phdr.p_vaddr
                            );
                            continue;
                        }

                        // A valid PT_LOAD satisfies p_filesz <= p_memsz. Reject the
                        // inconsistent case instead of deferring it to MemorySegment::load().
                        if phdr.p_filesz > phdr.p_memsz {
                            let reason: &str = "PT_LOAD has p_filesz greater than p_memsz";
                            ::syslog::warn!(
                                "load(): {reason} (vaddr={:#x}, filesz={}, memsz={})",
                                phdr.p_vaddr,
                                phdr.p_filesz,
                                phdr.p_memsz
                            );
                            return Err(Error::new(ErrorCode::BadFile, reason));
                        }

                        // Validate that the on-disk range [p_offset, p_offset + p_filesz)
                        // lies within the file before using it to slice bytes[...]. This
                        // guards against overflow and out-of-range panics on malformed ELFs.
                        let file_end: usize = phdr
                            .p_offset
                            .checked_add(phdr.p_filesz)
                            .filter(|end| *end <= bytes.len() as u64)
                            .map(|end| end as usize)
                            .ok_or_else(|| {
                                let reason: &str = "PT_LOAD file range out of bounds";
                                ::syslog::warn!(
                                    "load(): {reason} (offset={}, filesz={}, file_size={})",
                                    phdr.p_offset,
                                    phdr.p_filesz,
                                    bytes.len()
                                );
                                Error::new(ErrorCode::BadFile, reason)
                            })?;

                        ::syslog::debug!(
                            "load(): loadable program header (vaddr={:#x}, paddr={:#x}, \
                             filesz={}, memsz={})",
                            phdr.p_vaddr,
                            phdr.p_paddr,
                            phdr.p_filesz,
                            phdr.p_memsz
                        );

                        let (base, offset, capacity): (VirtualAddress, usize, usize) = {
                            let unaligned_base: usize =
                                load_address.into_raw_value() + phdr.p_vaddr as usize;
                            let base: usize = mm::align_down(unaligned_base, PAGE_ALIGNMENT);

                            // Check if program headers overlap.
                            if base < end_address.into_raw_value() {
                                let reason: &str = "program headers overlap";
                                ::syslog::warn!("load(): {} (phdr={:#x?}", reason, phdr);
                                return Err(Error::new(ErrorCode::BadFile, reason));
                            }

                            let base: VirtualAddress = VirtualAddress::from_raw_value(base);
                            let offset: usize = unaligned_base - base.into_raw_value();
                            let capacity: usize =
                                mm::align_up(offset + phdr.p_memsz as usize, PAGE_ALIGNMENT)
                                    .ok_or_else(|| {
                                        let reason: &str = "align_up overflow";
                                        ::syslog::warn!(
                                            "load(): {reason} (p_memsz={}, vaddr={:#x}, \
                                             base={:#x})",
                                            phdr.p_memsz,
                                            phdr.p_vaddr,
                                            base.into_raw_value()
                                        );
                                        Error::new(ErrorCode::BadFile, reason)
                                    })?;
                            let end_raw: usize =
                                base.into_raw_value().checked_add(capacity).ok_or_else(|| {
                                    let reason: &str = "end_address overflow";
                                    ::syslog::warn!(
                                        "load(): {reason} (base={:#x}, capacity={capacity})",
                                        base.into_raw_value()
                                    );
                                    Error::new(ErrorCode::BadFile, reason)
                                })?;
                            end_address = VirtualAddress::from_raw_value(end_raw);
                            (base, offset, capacity)
                        };

                        // Create memory segment.
                        let mut segment: MemorySegment =
                            MemorySegment::new(base, capacity, AccessPermission::RDWR)?;
                        segment.load(offset, &bytes[phdr.p_offset as usize..file_end])?;

                        segments.push(segment);
                    }
                }

                // Collect dependencies, preserving the `DT_NEEDED` order the
                // linker recorded them in. Symbol resolution searches this list
                // in order, so the order must not be lost. Duplicate entries are
                // dropped: a repeated `DT_NEEDED` name refers to the same
                // dependency and must occupy a single slot.
                let mut dependencies: Vec<(String, Option<Arc<Mutex<Self>>>)> = Vec::new();
                for library in elf.libraries.iter() {
                    ::syslog::debug!("load(): depends on library '{}'", library);
                    let name: String = library.to_string();
                    if dependencies.iter().any(|(existing, _)| existing == &name) {
                        continue;
                    }
                    dependencies.push((name, None));
                }

                // Collect section headers.
                let mut section_headers: BTreeMap<String, SectionHeader> = BTreeMap::new();
                for section in elf.section_headers.iter() {
                    ::syslog::debug!("load(): {:?}", section);
                    let section_name = elf.shdr_strtab.get_at(section.sh_name).unwrap_or("");
                    if let Some(_section) =
                        section_headers.insert(section_name.to_string(), section.clone())
                    {
                        let reason: &str = "duplicate section header";
                        ::syslog::warn!("load(): {} (section.name={:?})", reason, section_name);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    }
                }

                // Collect sections.
                let dynsym: SymbolTable = match Self::get_dynsym(&section_headers, load_address) {
                    Some(dynsym) => dynsym,
                    None => {
                        let reason: &str = "missing dynamic symbol table";
                        ::syslog::warn!("load(): {}", reason);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    },
                };
                let dynstr: StringTable = match Self::get_dynstr(&section_headers, load_address) {
                    Some(dynstr) => dynstr,
                    None => {
                        let reason: &str = "missing dynamic string table";
                        ::syslog::warn!("load(): {}", reason);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    },
                };
                let dynplt: Option<RelocationTable> =
                    Self::get_dynplt(&section_headers, load_address);
                let dynrel: Option<RelocationTable> =
                    Self::get_dynrel(&section_headers, load_address);
                let init_array: Option<(usize, usize)> =
                    Self::get_init_array(&section_headers, load_address);
                let fini_array: Option<(usize, usize)> =
                    Self::get_fini_array(&section_headers, load_address);

                // Build the symbol-lookup accelerator from the object's ELF hash
                // table, if it carries a usable one. `dynsym.len()` bounds the
                // hash chains; a `None` result means `find()` scans linearly.
                let hash: Option<SymbolHashTable> =
                    Self::get_symbol_hash(&section_headers, load_address, dynsym.len());

                // Collect `DT_RUNPATH` entries (goblin exposes them already
                // resolved against `.dynstr`). Each entry may be a colon-
                // separated list of directories; split here so the search
                // path probe can iterate them directly.
                let mut runpaths: Vec<String> = Vec::new();
                for raw in elf.runpaths.iter() {
                    for component in raw.split(':') {
                        if !component.is_empty() {
                            runpaths.push(component.to_string());
                        }
                    }
                }

                Ok(DynamicLibrary {
                    filename,
                    handle: DlHandle::allocate()?,
                    fd,
                    load_address,
                    dependencies,
                    _segments: segments,
                    dynsym,
                    dynstr,
                    hash,
                    dynplt,
                    dynrel,
                    init_array,
                    fini_array,
                    runpaths,
                    init_state: Arc::new(InitState::new()),
                    open_count: 0,
                })
            },
            Err(error) => {
                let reason: &str = "failed to parse ELF file";
                ::syslog::warn!("load(): {} (error={:?})", reason, error);
                Err(Error::new(ErrorCode::IoErr, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Computes the total virtual address space needed for all loadable segments of the ELF.
    ///
    /// The size is calculated as the distance from the lowest segment base to the highest segment
    /// end, both page-aligned. This produces a tight reservation even for shared libraries whose
    /// link-time `p_vaddr` values do not start at zero.
    ///
    /// # Parameters
    ///
    /// - `elf`: A parsed ELF binary.
    ///
    /// # Returns
    ///
    /// On success, returns the total size in bytes needed for all loadable segments.
    /// On failure, returns an [`Error`].
    ///
    fn compute_load_size(elf: &Elf) -> Result<usize, Error> {
        // NOTE: The returned size spans from the lowest segment base to the highest segment end.
        // For position-independent shared libraries whose link-time `p_vaddr` starts near zero,
        // the reservation may be larger than the sum of individual segment sizes because
        // inter-segment gaps (e.g., between .text and .data) are included. This is intentional:
        // the library loader maps segments at offsets relative to a single contiguous base.
        let mut min_base: usize = usize::MAX;
        let mut max_end: usize = 0;
        for phdr in elf.program_headers.iter() {
            if phdr.p_type == goblin::elf::program_header::PT_LOAD {
                let seg_base: usize = mm::align_down(phdr.p_vaddr as usize, PAGE_ALIGNMENT);
                if seg_base < min_base {
                    min_base = seg_base;
                }
                let unaligned_end: usize = phdr.p_vaddr as usize + phdr.p_memsz as usize;
                let aligned_end: usize =
                    mm::align_up(unaligned_end, PAGE_ALIGNMENT).ok_or_else(|| {
                        let reason: &str = "align_up overflow in compute_load_size";
                        ::syslog::warn!("compute_load_size(): {reason}");
                        Error::new(ErrorCode::BadFile, reason)
                    })?;
                if aligned_end > max_end {
                    max_end = aligned_end;
                }
            }
        }
        if max_end == 0 {
            let reason: &str = "no loadable segments found";
            ::syslog::warn!("compute_load_size(): {}", reason);
            return Err(Error::new(ErrorCode::BadFile, reason));
        }
        Ok(max_end - min_base)
    }

    /// Returns the name of the dynamic library.
    pub fn name(&self) -> &str {
        // FIXME: this function should return a reference to a c-string.
        self.filename.to_str().unwrap_or("")
    }

    /// Returns the stable handle that uniquely identifies this dynamic library.
    pub fn handle(&self) -> DlHandle {
        self.handle
    }

    /// Gets the relocation table for global variables (`.rel.dyn).
    fn get_dynrel(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<RelocationTable> {
        if let Some(pltrel_header) = section_headers.get(".rel.dyn") {
            let dynrel_size: usize =
                pltrel_header.sh_size as usize / mem::size_of::<RelocationEntry>();
            let dynrel_table_ptr: *mut RelocationEntry = (load_address.into_raw_value()
                + pltrel_header.sh_addr as usize)
                as *mut RelocationEntry;

            // SAFETY: `ptr` is a valid pointer to a relocation table of `len`.
            Some(unsafe { RelocationTable::from_raw_parts(dynrel_table_ptr, dynrel_size) })
        } else {
            None
        }
    }

    /// Gets a mutable reference to the relocation table for global functions (`.rel.plt`).
    fn get_dynplt(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<RelocationTable> {
        if let Some(pltrel_header) = section_headers.get(".rel.plt") {
            let len: usize = pltrel_header.sh_size as usize / mem::size_of::<RelocationEntry>();
            let ptr: *mut RelocationEntry = (load_address.into_raw_value()
                + pltrel_header.sh_addr as usize)
                as *mut RelocationEntry;

            // SAFETY: `ptr` is a valid pointer to a relocation table of `len`.
            Some(unsafe { RelocationTable::from_raw_parts(ptr, len) })
        } else {
            None
        }
    }

    /// Gets a reference to the string table (`.dynstr`).
    fn get_dynstr(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<StringTable> {
        if let Some(dynstr_header) = section_headers.get(".dynstr") {
            let len: usize = dynstr_header.sh_size as usize;
            let ptr: *mut u8 =
                (load_address.into_raw_value() + dynstr_header.sh_addr as usize) as *mut u8;

            // SAFETY: `ptr` is a valid pointer to a string table of `len`.
            Some(unsafe { StringTable::from_raw_parts(ptr, len) })
        } else {
            None
        }
    }

    /// Gets a reference to the symbol table (`.dynsym`).
    fn get_dynsym(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<SymbolTable> {
        if let Some(dynsym_header) = section_headers.get(".dynsym") {
            let len: usize = dynsym_header.sh_size as usize / mem::size_of::<Symbol>();
            let ptr: *mut Symbol =
                (load_address.into_raw_value() + dynsym_header.sh_addr as usize) as *mut Symbol;
            // SAFETY: `ptr` is a valid pointer to a symbol table of `len`.
            Some(unsafe { SymbolTable::from_raw_parts(ptr, len) })
        } else {
            None
        }
    }

    /// Builds a symbol-lookup accelerator from the object's ELF hash table.
    ///
    /// Prefers `.gnu.hash` (`DT_GNU_HASH`) over `.hash` (`DT_HASH`) when both are
    /// present, since the GNU table's Bloom filter makes it the faster of the
    /// two. Returns `None` when neither section is present or usable (for
    /// example an older object linked without a hash table), in which case
    /// [`find`](Self::find) falls back to a linear `.dynsym` scan. Both hash
    /// tables index the same `.dynsym`, whose length bounds their chains.
    fn get_symbol_hash(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
        symtab_len: usize,
    ) -> Option<SymbolHashTable> {
        // Prefer the GNU hash table.
        if let Some(header) = section_headers.get(".gnu.hash") {
            if header.sh_addr != 0 {
                let base: *const u8 =
                    (load_address.into_raw_value() + header.sh_addr as usize) as *const u8;
                let len: usize = header.sh_size as usize;
                // SAFETY: `.gnu.hash` is `SHF_ALLOC`, so it is mapped at
                // `load_address + sh_addr` for `sh_size` bytes; `from_raw_parts`
                // validates the declared layout against `len` before use.
                if let Some(table) = unsafe { GnuHashTable::from_raw_parts(base, len, symtab_len) }
                {
                    return Some(SymbolHashTable::Gnu(table));
                }
            }
        }

        // Fall back to the SysV hash table.
        if let Some(header) = section_headers.get(".hash") {
            if header.sh_addr != 0 {
                let base: *const u8 =
                    (load_address.into_raw_value() + header.sh_addr as usize) as *const u8;
                let len: usize = header.sh_size as usize;
                // SAFETY: `.hash` is `SHF_ALLOC`, so it is mapped at
                // `load_address + sh_addr` for `sh_size` bytes; `from_raw_parts`
                // validates the declared layout against `len` before use.
                if let Some(table) = unsafe { SysvHashTable::from_raw_parts(base, len) } {
                    return Some(SymbolHashTable::Sysv(table));
                }
            }
        }

        None
    }

    /// Looks up a function-pointer array section (`.init_array` / `.fini_array`)
    /// by name, returning the absolute address of the first entry and the
    /// number of `usize`-sized entries it contains.
    ///
    /// Returns `None` when the section is missing or empty. Entries are
    /// always 4 bytes on i386; if `sh_size` is not a multiple of `usize`
    /// the trailing bytes are silently truncated (a well-formed ELF should
    /// never trigger this).
    fn get_function_pointer_array(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
        section_name: &str,
    ) -> Option<(usize, usize)> {
        let header: &SectionHeader = section_headers.get(section_name)?;
        let count: usize = (header.sh_size as usize) / mem::size_of::<usize>();
        if count == 0 {
            return None;
        }
        let base: usize = load_address.into_raw_value() + header.sh_addr as usize;
        Some((base, count))
    }

    /// Gets the `.init_array` section descriptor, if present.
    fn get_init_array(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<(usize, usize)> {
        Self::get_function_pointer_array(section_headers, load_address, ".init_array")
    }

    /// Gets the `.fini_array` section descriptor, if present.
    fn get_fini_array(
        section_headers: &BTreeMap<String, SectionHeader>,
        load_address: VirtualAddress,
    ) -> Option<(usize, usize)> {
        Self::get_function_pointer_array(section_headers, load_address, ".fini_array")
    }

    /// Finds a symbol in the dynamic library.
    ///
    /// Uses the `DT_HASH` / `DT_GNU_HASH` accelerator when the object provides
    /// one, walking a short hash-bucket chain instead of the whole `.dynsym`.
    /// The chain narrows the candidates by hash; the actual name is still
    /// compared to reject hash collisions, so the result is identical to the
    /// linear scan. Objects without a usable hash section - or whose table
    /// proves inconsistent during the walk - fall back to
    /// [`find_linear`](Self::find_linear).
    ///
    /// NOTE: a `DT_GNU_HASH` table does not index undefined symbols, so a name
    /// that is only *referenced* (not defined) by this object is reported as
    /// absent here. That matches the sole caller's needs: it treats an undefined
    /// match the same as no match, resolving the symbol from the dependency tree
    /// or the global scope regardless.
    fn find(&self, symbol_name: &str) -> Option<&Symbol> {
        ::syslog::trace!("find(): symbol={} in dlname={:?}", symbol_name, self.filename);

        // Fast path: hash-bucket walk over the accelerator.
        if let Some(hash) = self.hash.as_ref() {
            match hash.lookup(symbol_name.as_bytes(), self.dynsym.len(), |idx| {
                self.dynsym
                    .get(idx as usize)
                    .and_then(|sym| self.dynstr.get_name(sym.name_offset()).ok())
                    .is_some_and(|name| !name.is_empty() && name == symbol_name)
            }) {
                Lookup::Found(index) => return self.dynsym.get(index as usize),
                Lookup::NotFound => return None,
                // An inconsistent hash table falls through to the linear scan, so
                // a malformed `.hash`/`.gnu.hash` cannot hide a symbol that this
                // object really defines.
                Lookup::Inconsistent => {},
            }
        }

        // Slow path: linear scan for objects without a usable hash section, or
        // when the accelerator proved inconsistent above.
        self.find_linear(symbol_name)
    }

    /// Linear fallback for [`find`](Self::find): scans `.dynsym` for a symbol
    /// whose name matches `symbol_name`. Used when the object carries no usable
    /// ELF hash table.
    fn find_linear(&self, symbol_name: &str) -> Option<&Symbol> {
        for sym in self.dynsym.iter() {
            if let Ok(lookup_symbol_name) = self.dynstr.get_name(sym.name_offset()) {
                if !lookup_symbol_name.is_empty() && lookup_symbol_name == symbol_name {
                    return Some(sym);
                }
            }
        }

        None
    }

    /// Looks up a symbol for **relocation resolution**, with global fallback.
    ///
    /// Search order:
    /// 1. The library itself (defined symbols).
    /// 2. The library's DT_NEEDED dependency tree (recursive).
    /// 3. The global symbol table (main executable + `RTLD_GLOBAL` symbols).
    ///
    /// Step 3 is required when binding a library's relocations against symbols
    /// exported by the main executable (`--export-dynamic`) or by libraries
    /// promoted with `RTLD_GLOBAL`. This global fallback is correct for
    /// relocation resolution but MUST NOT be used to service
    /// `dlsym(handle, ...)`: per POSIX, a request against a specific handle
    /// only searches the object's own load group. `dlsym()` therefore calls
    /// [`lookup_load_group`](Self::lookup_load_group) instead.
    pub fn lookup(&self, symbol_name: &str) -> Result<Option<(usize, usize)>, Error> {
        ::syslog::trace!("lookup(): symbol={}, dlname={:?}", symbol_name, self.filename);

        // Search self and dependency tree (the load group) without global
        // fallback first.
        if let Some(result) = self.lookup_load_group(symbol_name)? {
            return Ok(Some(result));
        }

        // Fall back to the global symbol table (symbols from the main
        // executable, registered via --export-dynamic, plus any library
        // promoted with RTLD_GLOBAL). This fallback is performed only once at
        // the top level, not during recursive dependency traversal.
        if let Some(addr) = super::global_symbol_lookup(symbol_name) {
            // Global symbols are absolute addresses, so base is 0.
            return Ok(Some((0, addr)));
        }

        Ok(None)
    }

    /// Looks up a symbol in this library's **load group only**: the object
    /// itself plus its `DT_NEEDED` dependency tree — with NO global-scope
    /// fallback.
    ///
    /// This is the lookup path POSIX mandates for `dlsym(handle, name)`: a
    /// request against a specific handle must resolve only within the object
    /// and the objects it was loaded with, never the global scope. Using the
    /// global-fallback [`lookup`](Self::lookup) here would incorrectly resolve
    /// symbols exported by the main executable (`--export-dynamic`) or by other
    /// libraries promoted with `RTLD_GLOBAL`. Global-scope searches are reserved
    /// for the `RTLD_DEFAULT` / `dlopen(NULL)` handle, which `dlsym()` routes to
    /// `global_symbol_lookup()` separately.
    pub fn lookup_load_group(&self, symbol_name: &str) -> Result<Option<(usize, usize)>, Error> {
        ::syslog::trace!("lookup_load_group(): symbol={}, dlname={:?}", symbol_name, self.filename);

        // POSIX / System V symbol resolution searches a load group in
        // breadth-first `DT_NEEDED` order: the object itself first, then its
        // direct dependencies in the order the linker recorded them, then the
        // dependencies of those, and so on. This mirrors the order glibc walks
        // when it services a lookup against an object's `l_searchlist`, so a
        // symbol defined by more than one object in the group resolves to the
        // first definition in `DT_NEEDED` order rather than to an
        // alphabetically-first one.

        // The root object itself is the head of the search list.
        if let Some(symbol) = self.find(symbol_name) {
            if !symbol.is_undefined() {
                return Ok(Some((self.load_address.into_raw_value(), symbol.value() as usize)));
            }
        }

        // `visited` records the `Arc` allocation addresses of dependencies
        // already searched in this lookup, so a diamond-shaped graph searches
        // each object at most once. `queue` drives the breadth-first walk:
        // direct dependencies are enqueued in `DT_NEEDED` order and each searched
        // object appends its own dependencies to the back, so shallower and
        // earlier `DT_NEEDED` objects are always searched first. A `DT_NEEDED`
        // cycle is rejected at load time, so the bound graph is acyclic and the
        // queue is bounded by the number of dependency edges.
        let mut visited: BTreeSet<usize> = BTreeSet::new();
        let mut queue: VecDeque<Arc<Mutex<Self>>> = VecDeque::new();
        for dependency in self.dependencies.iter().filter_map(|(_, dep)| dep.as_ref()) {
            queue.push_back(dependency.clone());
        }

        while let Some(dlfile) = queue.pop_front() {
            // Skip a dependency already searched via a shorter or earlier path
            // (diamond-shaped graph).
            let id: usize = Arc::as_ptr(&dlfile) as usize;
            if visited.contains(&id) {
                continue;
            }

            // Guard against deadlock: if the mutex is held by an ancestor in our
            // call chain (a cycle back to a locked parent) or by a concurrent
            // lookup, skip rather than spinning forever. Leave it unmarked so
            // another queued edge may still search it once it is free.
            if dlfile.is_locked() {
                continue;
            }
            visited.insert(id);

            let dlfile: MutexGuard<'_, DynamicLibrary> = dlfile.lock();

            if let Some(symbol) = dlfile.find(symbol_name) {
                if !symbol.is_undefined() {
                    return Ok(Some((
                        dlfile.load_address.into_raw_value(),
                        symbol.value() as usize,
                    )));
                }
            }

            // Append this object's dependencies to the back of the queue in
            // `DT_NEEDED` order, preserving the breadth-first search order.
            for dependency in dlfile
                .dependencies
                .iter()
                .filter_map(|(_, dep)| dep.as_ref())
            {
                queue.push_back(dependency.clone());
            }
        }

        Ok(None)
    }

    fn get_symbol(&self, rel: &RelocationEntry) -> Result<&Symbol, Error> {
        if let Some(sym) = self.dynsym.get(rel.symbol_index() as usize) {
            Ok(sym)
        } else {
            let reason: &str = "invalid symbol index";
            ::syslog::warn!("get_symbol(): {} (rel={:?})", reason, rel);
            Err(Error::new(ErrorCode::BadFile, reason))
        }
    }

    fn get_symbol_value(&self, sym: &Symbol) -> Result<usize, Error> {
        let symbol_name: &str = self.dynstr.get_name(sym.name_offset())?;
        let symbol_value: usize = match self.lookup(symbol_name)? {
            Some((base, symbol_value)) => base + symbol_value,
            None => {
                // Per the System V ABI (gABI, chapter "Symbol Table"), an undefined
                // symbol whose binding is `STB_WEAK` and which cannot be resolved at
                // dynamic-link time is silently taken to have the value zero (or `NULL`
                // for function symbols). Every mainstream ELF dynamic loader (glibc
                // `elf/dl-lookup.c`, musl `ldso/dynlink.c`, FreeBSD `rtld-elf/rtld.c`,
                // Android Bionic `linker/linker_relocate.cpp`) implements this rule,
                // and we follow them here.
                //
                // Substituting zero is safe across the relocation types we currently
                // handle (R_386_32, R_386_PC32, R_386_JMP_SLOT, R_386_GLOB_DAT): the
                // resulting GOT/PLT entry or in-place 32-bit slot will be null, so any
                // code path that actually dereferences the symbol traps deterministically
                // — matching the contract the spec puts on the program (it must
                // null-check before use).
                if sym.is_undefined() && sym.is_weak() {
                    ::syslog::debug!(
                        "get_symbol_value(): resolving unresolved weak undefined symbol to zero \
                         per System V ABI (symbol_name={:?})",
                        symbol_name
                    );
                    return Ok(0);
                }

                let reason: &str = "symbol not found";
                ::syslog::warn!(
                    "get_symbol_value(): {} (symbol_name={:?}, symbol={:?})",
                    reason,
                    symbol_name,
                    sym
                );
                return Err(Error::new(ErrorCode::BadFile, reason));
            },
        };

        Ok(symbol_value)
    }

    /// Queries for the nearest symbol lower than the given address in the dynamic library.
    pub fn query(
        &self,
        symbol_addr: VirtualAddress,
    ) -> Option<(*const i8, VirtualAddress, *const i8, VirtualAddress)> {
        ::syslog::trace!("query(): symbol_addr={:#X?} in dlname={:?}", symbol_addr, self.filename);

        let mut nearest_symbol: Option<(*const i8, VirtualAddress, *const i8, VirtualAddress)> =
            None;

        for sym in self.dynsym.iter() {
            // Skip undefined symbols: with the STB_WEAK handling in
            // `get_symbol_value()`, an unresolved weak undefined symbol resolves
            // to 0 — that's the right behaviour for relocation but would cause
            // `dladdr()` to report a ghost symbol at address 0 for every weak
            // undefined entry in the dynsym.  Symbols that have an in-module
            // definition (or that resolved to a real address elsewhere) are
            // never `SHN_UNDEF` in this DSO's dynsym, so this filter only
            // excludes references the loader had to substitute zero for.
            if sym.is_undefined() {
                continue;
            }
            if let Ok(symbol_value) = self.get_symbol_value(sym) {
                let sym_addr: VirtualAddress = VirtualAddress::from_raw_value(symbol_value);
                if sym_addr <= symbol_addr {
                    if let Some(name) = self.dynstr.get_name_bytes(sym.name_offset()) {
                        if let Some((_, _, _, nearest_addr)) = &nearest_symbol {
                            if sym_addr > *nearest_addr {
                                nearest_symbol = Some((
                                    self.filename.as_ptr(),
                                    self.load_address,
                                    name.as_ptr() as *const i8,
                                    sym_addr,
                                ));
                            }
                        } else {
                            nearest_symbol = Some((
                                self.filename.as_ptr(),
                                self.load_address,
                                name.as_ptr() as *const i8,
                                sym_addr,
                            ));
                        }
                    }
                }
            }
        }

        nearest_symbol
    }

    /// Resolves a symbol in the dynamic library.
    pub fn resolve_all(&self) -> Result<(), Error> {
        ::syslog::trace!("resolve()");

        if let Some(rel) = self.dynplt.as_ref() {
            for rel in rel.iter() {
                self.resolve(rel)?;
            }
        }
        if let Some(rel) = self.dynrel.as_ref() {
            for rel in rel.iter() {
                self.resolve(rel)?;
            }
        }

        Ok(())
    }

    fn resolve(&self, rel: &RelocationEntry) -> Result<(), Error> {
        let storage_unit: UnalignedPointer<u32> = UnalignedPointer::new(
            (self.load_address.into_raw_value() as u32 + rel.offset()) as *mut u32,
        );

        match rel.typ()? {
            RelocationType::R_386_RELATIVE => {
                // R_386_RELATIVE relocation must have a zero symbol index.
                if rel.symbol_index() != 0 {
                    let reason: &str = "invalid R_386_RELATIVE relocation";
                    ::syslog::warn!("resolve(): {} (rel={:?})", reason, rel);
                    return Err(Error::new(ErrorCode::BadFile, reason));
                }

                unsafe {
                    Self::resolve_r_386_relative(
                        storage_unit,
                        self.load_address.into_raw_value() as u32,
                    );
                }
            },

            RelocationType::R_386_32 => {
                let sym: &Symbol = self.get_symbol(rel)?;
                let symbol_value: usize = self.get_symbol_value(sym)?;
                unsafe {
                    Self::resolve_r_386_32(storage_unit, symbol_value as u32);
                }
            },

            RelocationType::R_386_PC32 => {
                let sym: &Symbol = self.get_symbol(rel)?;
                let symbol_value: usize = self.get_symbol_value(sym)?;

                unsafe {
                    Self::resolve_r_386_pc32(storage_unit, symbol_value as u32);
                }
            },
            RelocationType::R_386_JMP_SLOT => {
                let sym: &Symbol = self.get_symbol(rel)?;
                let symbol_value: usize = self.get_symbol_value(sym)?;

                unsafe {
                    Self::resolve_r_386_jmp_slot(storage_unit, symbol_value as u32);
                }
            },
            RelocationType::R_386_GLOB_DAT => {
                let sym: &Symbol = self.get_symbol(rel)?;
                let symbol_value: usize = self.get_symbol_value(sym)?;

                unsafe {
                    Self::resolve_r_386_glob_dat(storage_unit, symbol_value as u32);
                }
            },

            relocation_entry_type => {
                let reason: &str = "unsupported relocation type";
                ::syslog::warn!(
                    "resolve(): {} (relocation_type={:?}, rel={:?})",
                    reason,
                    relocation_entry_type,
                    rel
                );
                return Err(Error::new(ErrorCode::BadFile, reason));
            },
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resolves a R_386_RELATIVE relocation.
    ///
    /// # Parameters
    ///
    /// - `storage_unit` - A pointer to the storage unit being relocated.
    /// - `base_address` - The base address at which the shared object has ben loaded into memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs pointer arithmetic and dereferences raw pointers.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    /// - The `storage_unit` points to the storage unit of a valid R_386_RELATIVE relocation entry.
    ///
    unsafe fn resolve_r_386_relative(mut storage_unit: UnalignedPointer<u32>, base_address: u32) {
        let symbol_addend: i32 = storage_unit.read_unaligned() as i32;
        let relocation_value: u32 = base_address.strict_add_signed(symbol_addend);
        storage_unit.write_unaligned(relocation_value);
    }

    ///
    /// # Description
    ///
    /// Resolves a R_386_32 relocation.
    ///
    /// # Parameters
    ///
    /// - `storage_unit` - A pointer to the storage unit being relocated.
    /// - `symbol_value` - The value of the symbol being relocated.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs pointer arithmetic and dereferences raw pointers.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    /// - The `storage_unit` points to the storage unit of a valid R_386_32 relocation entry.
    ///
    unsafe fn resolve_r_386_32(mut storage_unit: UnalignedPointer<u32>, symbol_value: u32) {
        let symbol_addend: i32 = storage_unit.read_unaligned() as i32;
        // ELF arithmetic (System V ABI): `S + A` is performed with wrapping
        // 32-bit semantics.  Use `wrapping_add_signed` instead of
        // `strict_add_signed` so that the loader does not panic when
        // resolving a weak undefined symbol (`S == 0`) against a negative
        // addend, which is legal per spec.
        let final_value: u32 = symbol_value.wrapping_add_signed(symbol_addend);
        storage_unit.write_unaligned(final_value);
    }

    ///
    /// # Description
    ///
    /// Resolves a R_386_JMP_SLOT relocation.
    ///
    /// # Parameters
    ///
    /// - `storage_unit` - A pointer to the storage unit being relocated.
    /// - `symbol_value` - The value of the symbol being relocated.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs pointer arithmetic and dereferences raw pointers.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    /// - The `storage_unit` points to the storage unit of a valid R_386_JMP_SLOT relocation entry.
    ///
    unsafe fn resolve_r_386_jmp_slot(mut storage_unit: UnalignedPointer<u32>, symbol_value: u32) {
        storage_unit.write_unaligned(symbol_value);
    }

    ///
    /// # Description
    ///
    /// Resolves a R_386_GLOB_DAT relocation.
    ///
    /// # Parameters
    ///
    /// - `storage_unit` - A pointer to the storage unit being relocated.
    /// - `symbol_value` - The value of the symbol being relocated.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs pointer arithmetic and dereferences raw pointers.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    /// - The `storage_unit` points to the storage unit of a valid R_386_GLOB_DAT relocation entry.
    ///
    unsafe fn resolve_r_386_glob_dat(mut storage_unit: UnalignedPointer<u32>, symbol_value: u32) {
        storage_unit.write_unaligned(symbol_value);
    }

    ///
    /// # Description
    ///
    /// Resolves a R_386_PC32 relocation.
    ///
    /// # Parameters
    ///
    /// - `storage_unit` - A pointer to the storage unit being relocated.
    /// - `symbol_value` - The value of the symbol being relocated.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs pointer arithmetic and dereferences raw pointers.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    /// - The `storage_unit` points to the storage unit of a valid R_386_PC32 relocation entry.
    ///
    unsafe fn resolve_r_386_pc32(mut storage_unit: UnalignedPointer<u32>, symbol_value: u32) {
        let symbol_addend: i32 = storage_unit.read_unaligned() as i32;
        let relocation_offset: u32 = storage_unit.as_ptr() as u32;
        // ELF arithmetic (System V ABI): `S + A - P` is performed with
        // wrapping 32-bit semantics.  Use `wrapping_add_signed` instead of
        // `strict_add_signed` so that the loader does not panic when
        // resolving a weak undefined symbol (`S == 0`) against a negative
        // addend, which is legal per spec.
        let tmp: u32 = symbol_value.wrapping_add_signed(symbol_addend);

        let final_value: i32 = if tmp > relocation_offset {
            (tmp - relocation_offset) as i32
        } else {
            -((relocation_offset - tmp) as i32)
        };

        storage_unit.write_unaligned(final_value as u32);
    }

    /// Returns a clone of this library's dependency list, in `DT_NEEDED` order.
    pub fn dependencies(&self) -> Vec<(String, Option<Arc<Mutex<Self>>>)> {
        self.dependencies.clone()
    }

    /// Returns the handles of all bound dependencies.
    pub fn dependency_handles(&self) -> Vec<DlHandle> {
        self.dependencies
            .iter()
            .filter_map(|(_, dep)| dep.as_ref().map(|d| d.lock().handle()))
            .collect()
    }

    /// Iterates over all defined (non-undefined) symbols exported by this
    /// library, yielding `(name, absolute_address)` pairs.
    pub fn exported_symbols(&self) -> Vec<(&str, usize)> {
        let mut result: Vec<(&str, usize)> = Vec::new();
        for sym in self.dynsym.iter() {
            if sym.is_undefined() {
                continue;
            }
            match self.dynstr.get_name(sym.name_offset()) {
                Ok(name) if !name.is_empty() => {
                    let addr: usize = self.load_address.into_raw_value() + sym.value() as usize;
                    result.push((name, addr));
                },
                Err(e) => {
                    ::syslog::warn!(
                        "exported_symbols(): skipping symbol at offset {} (error={:?})",
                        sym.name_offset(),
                        e
                    );
                },
                _ => {},
            }
        }
        result
    }

    /// Binds a dependency to the dynamic library.
    pub fn bind_dependency(
        &mut self,
        name: String,
        library: Arc<Mutex<Self>>,
    ) -> Result<(), Error> {
        match self.dependencies.iter_mut().find(|entry| entry.0 == name) {
            Some(entry) => {
                if entry.1.is_some() {
                    let reason: &str = "dependency already loaded";
                    ::syslog::warn!("bind_dependency(): {}", reason);
                    return Err(Error::new(ErrorCode::BadFile, reason));
                }
                entry.1 = Some(library);
                Ok(())
            },
            None => {
                let reason: &str = "dependency not listed";
                ::syslog::warn!("bind_dependency(): {}", reason);
                Err(Error::new(ErrorCode::BadFile, reason))
            },
        }
    }

    /// Detaches and returns all bound dependencies of the dynamic library.
    ///
    /// Each returned dependency edge is removed from this library, so the
    /// `Arc` references it held are released to the caller. `dlclose` relies
    /// on this to drop a dependent's hold on its dependencies before deciding
    /// whether those dependencies have become unreferenced and can be
    /// unloaded.
    pub fn take_dependencies(&mut self) -> Vec<(String, Arc<Mutex<Self>>)> {
        let mut dependencies: Vec<(String, Arc<Mutex<Self>>)> = Vec::new();
        for (name, library) in self.dependencies.iter_mut() {
            if let Some(library) = library.take() {
                dependencies.push((name.clone(), library));
            }
        }
        dependencies
    }

    /// Returns the `DT_RUNPATH` directories of the library, split on `:`.
    pub fn runpaths(&self) -> &[String] {
        &self.runpaths
    }

    /// Returns a cloneable handle to this library's construction state so a
    /// concurrent `dlopen` can wait for its constructors to finish without
    /// holding any dlfcn lock.
    pub fn init_state(&self) -> Arc<InitState> {
        self.init_state.clone()
    }

    /// Returns the number of outstanding direct `dlopen()` handles for this
    /// library (glibc's `l_direct_opencount`).
    pub fn open_count(&self) -> usize {
        self.open_count
    }

    /// Records an additional direct `dlopen()` of this library and returns the
    /// updated open count. Invoked on both the initial load and every
    /// subsequent cache hit so that staggered open/close sequences keep the
    /// library loaded while any handle is still live.
    pub fn increment_open_count(&mut self) -> usize {
        self.open_count = self.open_count.saturating_add(1);
        self.open_count
    }

    /// Balances a direct `dlopen()` with a `dlclose()` and returns the updated
    /// open count. Saturates at zero so an unbalanced `dlclose()` cannot
    /// underflow the counter.
    pub fn decrement_open_count(&mut self) -> usize {
        self.open_count = self.open_count.saturating_sub(1);
        self.open_count
    }

    /// Returns the loaded `.init_array` descriptor as `(base_address,
    /// entry_count)`, or `None` if the library has no constructors.
    ///
    /// Callers should snapshot this under the library's mutex, drop the
    /// mutex, and then invoke the entries via [`invoke_init_array`]. The
    /// descriptor remains valid for as long as the owning
    /// `Arc<Mutex<DynamicLibrary>>` is alive.
    pub fn init_array_descriptor(&self) -> Option<(usize, usize)> {
        self.init_array
    }

    /// Returns the loaded `.fini_array` descriptor as `(base_address,
    /// entry_count)`, or `None` if the library has no destructors.
    ///
    /// See [`init_array_descriptor`](Self::init_array_descriptor) for the
    /// expected lock-handling pattern.
    pub fn fini_array_descriptor(&self) -> Option<(usize, usize)> {
        self.fini_array
    }

    /// Invokes every function pointer in the supplied `.init_array`
    /// descriptor in order, as required by the System V ABI for shared-
    /// object constructor execution.
    ///
    /// `name` is purely for diagnostic logging.
    ///
    /// # Locking
    ///
    /// This function must be called with **no** dlfcn locks held — neither
    /// `DYNAMIC_LIBRARY_REGISTRY` nor the per-library mutex. Constructors
    /// may legally call `dlsym` (and, in a future relaxation, `dlopen`),
    /// both of which would otherwise re-enter the same locks and deadlock.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `descriptor` was produced by
    /// [`init_array_descriptor`](Self::init_array_descriptor) from a
    /// `DynamicLibrary` whose memory segments are still mapped, that
    /// `resolve_all` has already applied any `R_386_RELATIVE` patches to
    /// the section, and that the holding `Arc` lives until this call
    /// returns. Entry values equal to `0` or `usize::MAX` are treated as
    /// sentinels and skipped, matching the glibc loader behaviour.
    pub unsafe fn invoke_init_array(descriptor: Option<(usize, usize)>, name: &str) {
        let (base, count): (usize, usize) = match descriptor {
            Some(range) => range,
            None => return,
        };
        ::syslog::debug!("invoke_init_array(): library={:?} entries={}", name, count);
        for index in 0..count {
            // SAFETY: `base` points to the loaded `.init_array` section of
            // the originating library and `count` is the number of
            // `usize`-sized entries it contains.
            let entry_ptr: *const usize = (base + index * mem::size_of::<usize>()) as *const usize;
            let entry: usize = unsafe { entry_ptr.read_unaligned() };
            if entry == 0 || entry == usize::MAX {
                continue;
            }
            // SAFETY: the .init_array entry is a function pointer with C
            // calling convention and no arguments per the System V ABI.
            let func: extern "C" fn() = unsafe { mem::transmute::<usize, extern "C" fn()>(entry) };
            func();
        }
    }

    /// Invokes every function pointer in the supplied `.fini_array`
    /// descriptor in reverse order, as required by the System V ABI for
    /// shared-object destructor execution. Must be called before the
    /// library's memory segments are unmapped.
    ///
    /// `name` is purely for diagnostic logging.
    ///
    /// # Locking
    ///
    /// This function must be called with **no** dlfcn locks held -- neither
    /// `DYNAMIC_LIBRARY_REGISTRY` nor the per-library mutex of the
    /// originating library. The `dlclose()` caller releases
    /// `DYNAMIC_LIBRARY_REGISTRY` before invoking any destructor, so a
    /// destructor may legally call back into `dlopen`/`dlclose`/`dlsym`
    /// without deadlocking. The libraries being unloaded are left in the
    /// registry, with their dependency edges intact, until *every* destructor
    /// has run (see `dlclose`); therefore `dlsym(self_handle, ...)` and a
    /// re-entrant `dlopen` of the closing library (or of one of its
    /// still-loaded dependencies) resolve correctly during teardown instead
    /// of failing or mapping a second copy.
    ///
    /// # Safety
    ///
    /// See [`invoke_init_array`](Self::invoke_init_array).
    pub unsafe fn invoke_fini_array(descriptor: Option<(usize, usize)>, name: &str) {
        let (base, count): (usize, usize) = match descriptor {
            Some(range) => range,
            None => return,
        };
        ::syslog::debug!("invoke_fini_array(): library={:?} entries={}", name, count);
        for index in (0..count).rev() {
            // SAFETY: see `invoke_init_array`.
            let entry_ptr: *const usize = (base + index * mem::size_of::<usize>()) as *const usize;
            let entry: usize = unsafe { entry_ptr.read_unaligned() };
            if entry == 0 || entry == usize::MAX {
                continue;
            }
            let func: extern "C" fn() = unsafe { mem::transmute::<usize, extern "C" fn()>(entry) };
            func();
        }
    }
}

impl fmt::Debug for DynamicLibrary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DlFile {{ name={:?}, fd={:?} }}", self.filename, self.fd)
    }
}

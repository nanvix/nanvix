// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================
use crate::{
    dlfcn::syscall::segment::MemorySegment,
    fcntl::OpenFlags,
    ffi::{
        c_int,
        c_void,
    },
    safe::FileDescriptor,
    sys::stat::{
        self,
        file_mode,
    },
};
use ::alloc::{
    collections::btree_map::BTreeMap,
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
use ::core::mem;
use ::elf::{
    RelocationEntry,
    RelocationTable,
    StringTable,
    Symbol,
    SymbolTable,
    SymbolType,
};
use ::goblin::elf::{
    Elf,
    SectionHeader,
};
use ::nvx::{
    mm::{
        self,
        Address,
        VirtualAddress,
        PAGE_ALIGNMENT,
    },
    sys::{
        config::memory_layout::USER_LIBS_BASE,
        error::{
            Error,
            ErrorCode,
        },
    },
};
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};

//===================================================================================================
// DlHandle
//==================================================================================================

///
/// # Description
///
/// A structure that represents a handle to a dynamic library file.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct DlHandle(c_int);

impl DlHandle {
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
// Global Variables
//==================================================================================================

/// Base virtual address for dynamic libraries.
static LIBRARIES_BASE: Lazy<Mutex<VirtualAddress>> = Lazy::new(|| Mutex::new(USER_LIBS_BASE));

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
    /// Underlying file descriptor.
    fd: FileDescriptor,
    /// Load address.
    load_address: VirtualAddress,
    /// Memory segments.
    _segments: Vec<MemorySegment>, // Keep this here to prevent memory drop.
    /// Dependencies.
    dependencies: BTreeMap<String, Option<Arc<Mutex<Self>>>>,
    /// Dynamic symbols.
    dynsym: SymbolTable,
    /// Dynamic symbols names.
    dynstr: StringTable,
    /// Relocation table for global functions.
    dynplt: Option<RelocationTable>,
    /// Relocation table for global variables.
    dynrel: Option<RelocationTable>,
}

impl DynamicLibrary {
    /// Opens a dynamic library file.
    pub fn open(filename: &str) -> Result<Self, Error> {
        ::nvx::trace!("open(): filename={}", filename);
        // Attempt to open file.
        let fd: FileDescriptor = FileDescriptor::open(filename, OpenFlags::O_RDONLY.into(), 0)?;

        // Convert filename to a C string.
        let filename: CString = match CString::new(filename) {
            Ok(cstr) => cstr,
            Err(_) => {
                let reason: &str = "failed to convert filename to C string";
                ::nvx::error!("open(): {}", reason);
                return Err(Error::new(ErrorCode::BadFile, reason));
            },
        };

        // Retrieve file information.
        let mut buf: stat::stat = stat::stat::default();
        fd.stat(&mut buf)?;

        // Check if file is not a regular file.
        if !file_mode::S_ISREG(buf.st_mode) {
            let reason: &str = "file is not a regular file";
            ::nvx::error!("open(): {}", reason);
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

        // Attempt to load file in one shot.
        let file_size: usize = buf.st_size as usize;
        let mut bytes: Vec<u8> = vec![0; file_size];
        fd.read(&mut bytes)?;

        // Lock the base address for libraries to prevent any other thread to modify it while we
        // load this library.
        let mut libraries_base: MutexGuard<'_, VirtualAddress> = LIBRARIES_BASE.lock();

        // Parse ELF file.
        match Elf::parse(&bytes) {
            Ok(elf) => {
                // Check if ELF file is not a dynamic library.
                if !elf.is_lib {
                    let reason: &str = "file is not a dynamic library";
                    ::nvx::error!("load(): {}", reason);
                    return Err(Error::new(ErrorCode::BadFile, reason));
                }

                let load_address: VirtualAddress = *libraries_base;
                let mut end_address: VirtualAddress = load_address;

                let mut segments: Vec<MemorySegment> = Vec::new();

                // Traverse table of program headers, looking for the loadable ones.
                for phdr in elf.program_headers.iter() {
                    // Check if program header is loadable.
                    if phdr.p_type == goblin::elf::program_header::PT_LOAD {
                        ::nvx::debug!(
                            "load(): loadable program header (vaddr={:#x}, paddr={:#x}, \
                             filesz={}, memsz={})",
                            phdr.p_vaddr,
                            phdr.p_paddr,
                            phdr.p_filesz,
                            phdr.p_memsz
                        );

                        let (base, capacity): (VirtualAddress, usize) = {
                            let base: VirtualAddress = end_address;
                            let capacity: usize =
                                mm::align_up(phdr.p_memsz as usize, PAGE_ALIGNMENT);
                            end_address =
                                VirtualAddress::from_raw_value(base.into_raw_value() + capacity);
                            (base, capacity)
                        };

                        // Create memory segment.
                        let mut segment: MemorySegment = MemorySegment::new(base, capacity)?;
                        segment.load(
                            &bytes
                                [phdr.p_offset as usize..(phdr.p_offset + phdr.p_filesz) as usize],
                        )?;

                        segments.push(segment);
                    }
                }

                // Collect dependencies.
                let mut dependencies: BTreeMap<String, Option<Arc<Mutex<Self>>>> = BTreeMap::new();
                if !elf.libraries.is_empty() {
                    for library in elf.libraries.iter() {
                        ::nvx::debug!("load(): depends on library '{}'", library);
                        dependencies.insert(library.to_string(), None);
                    }
                }

                // Collect section headers.
                let mut section_headers: BTreeMap<String, SectionHeader> = BTreeMap::new();
                for section in elf.section_headers.iter() {
                    ::nvx::debug!("load(): {:?}", section);
                    let section_name = elf.shdr_strtab.get_at(section.sh_name).unwrap_or("");
                    if let Some(_section) =
                        section_headers.insert(section_name.to_string(), section.clone())
                    {
                        let reason: &str = "duplicate section header";
                        ::nvx::error!("load(): {} (section.name={:?})", reason, section_name);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    }
                }

                // Collect sections.
                let dynsym: SymbolTable = match Self::get_dynsym(&section_headers, load_address) {
                    Some(dynsym) => dynsym,
                    None => {
                        let reason: &str = "missing dynamic symbol table";
                        ::nvx::error!("load(): {}", reason);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    },
                };
                let dynstr: StringTable = match Self::get_dynstr(&section_headers, load_address) {
                    Some(dynstr) => dynstr,
                    None => {
                        let reason: &str = "missing dynamic string table";
                        ::nvx::error!("load(): {}", reason);
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    },
                };
                let dynplt: Option<RelocationTable> =
                    Self::get_dynplt(&section_headers, load_address);
                let dynrel: Option<RelocationTable> =
                    Self::get_dynrel(&section_headers, load_address);

                //==================================================================
                // Commit new base address for libraries.
                // No fail can happen after this point, else we leak address space.
                //==================================================================
                *libraries_base = end_address;

                Ok(DynamicLibrary {
                    filename,
                    fd,
                    load_address,
                    dependencies,
                    _segments: segments,
                    dynsym,
                    dynstr,
                    dynplt,
                    dynrel,
                })
            },
            Err(error) => {
                let reason: &str = "failed to parse ELF file";
                ::nvx::error!("load(): {} (error={:?})", reason, error);
                Err(Error::new(ErrorCode::IoErr, reason))
            },
        }
    }

    /// Returns the name of the dynamic library.
    pub fn name(&self) -> &str {
        // FIXME: this function should return a reference to a c-string.
        self.filename.to_str().unwrap_or("")
    }

    /// Returns a handle that uniquely identifies the dynamic library file.
    pub fn handle(&self) -> DlHandle {
        DlHandle(self.fd.get_raw_fd())
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
                + pltrel_header.sh_offset as usize)
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
                + pltrel_header.sh_offset as usize)
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
                (load_address.into_raw_value() + dynstr_header.sh_offset as usize) as *mut u8;

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
                (load_address.into_raw_value() + dynsym_header.sh_offset as usize) as *mut Symbol;
            // SAFETY: `ptr` is a valid pointer to a symbol table of `len`.
            Some(unsafe { SymbolTable::from_raw_parts(ptr, len) })
        } else {
            None
        }
    }

    /// Finds a symbol in the dynamic library.
    fn find(&self, symbol_name: &str) -> Option<&Symbol> {
        ::nvx::trace!("find(): symbol={} in dlname={:?}", symbol_name, self.filename);

        for sym in self.dynsym.iter() {
            if let Ok(lookup_symbol_name) = self.dynstr.get_name(sym.name_offset()) {
                if !lookup_symbol_name.is_empty() && lookup_symbol_name == symbol_name {
                    return Some(sym);
                }
            }
        }

        None
    }

    /// Looks up a symbol in the dynamic library.
    pub fn lookup(&self, symbol_name: &str, search_dependencies: bool) -> Option<VirtualAddress> {
        ::nvx::trace!("lookup(): symbol={} in dlname={:?}", symbol_name, self.filename);

        if let Some(symbol) = self.find(symbol_name) {
            if symbol.value() != 0 {
                return Some(VirtualAddress::from_raw_value(symbol.value() as usize));
            }
        }

        if search_dependencies {
            for (_dlname, dlfile) in self.dependencies.iter() {
                if let Some(dlfile) = dlfile {
                    let dlfile: MutexGuard<'_, DynamicLibrary> = dlfile.lock();

                    if let Some(symbol) = dlfile.find(symbol_name) {
                        if !symbol.is_undefined() {
                            return Some(VirtualAddress::from_raw_value(
                                dlfile.load_address.into_raw_value() + symbol.value() as usize,
                            ));
                        }
                    }
                }
            }
        }

        None
    }

    /// Queries for the nearest symbol lower than the given address in the dynamic library.
    pub fn query(
        &self,
        symbol_addr: VirtualAddress,
    ) -> Option<(*const i8, VirtualAddress, *const i8, VirtualAddress)> {
        ::nvx::trace!("query(): symbol_addr={:#X?} in dlname={:?}", symbol_addr, self.filename);

        let mut nearest_symbol: Option<(*const i8, VirtualAddress, *const i8, VirtualAddress)> =
            None;

        for sym in self.dynsym.iter() {
            if !sym.is_undefined() {
                let sym_addr = VirtualAddress::from_raw_value(sym.value() as usize);
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
    pub fn resolve(&mut self, symbol_name: &str, value: usize) -> Result<(), Error> {
        ::nvx::trace!("resolve(): symbol={}, value={:#X}", symbol_name, value);

        for (index, sym) in self.dynsym.iter_mut().enumerate() {
            if self
                .dynstr
                .get_name(sym.name_offset())
                .ok()
                .filter(|name| *name == symbol_name)
                .is_some()
                && sym.is_undefined()
            {
                let relocation_table: &mut RelocationTable = match sym.typ() {
                    SymbolType::Function => match self.dynplt.as_mut() {
                        Some(plt) => plt,
                        None => {
                            let reason = "missing relocation table for global functions";
                            ::nvx::error!("resolve(): {} (symbol_type={:?})", reason, sym.typ());
                            return Err(Error::new(ErrorCode::BadFile, reason));
                        },
                    },
                    SymbolType::Object => match self.dynrel.as_mut() {
                        Some(rel) => rel,
                        None => {
                            let reason = "missing relocation table for global variables";
                            ::nvx::error!("resolve(): {} (symbol_type={:?})", reason, sym.typ());
                            return Err(Error::new(ErrorCode::BadFile, reason));
                        },
                    },
                    _ => {
                        let reason = "unsupported symbol type";
                        ::nvx::error!("resolve(): {} (symbol_type={:?})", reason, sym.typ());
                        return Err(Error::new(ErrorCode::BadFile, reason));
                    },
                };

                if let Some(rel) = relocation_table
                    .iter_mut()
                    .find(|rel| rel.symbol_index() as usize == index)
                {
                    unsafe {
                        rel.bind(self.load_address.into_raw_value() as u32, sym.value())?;
                    }
                    sym.resolve(value as u32);
                    return Ok(());
                }
            }
        }

        let reason = "symbol not found";
        ::nvx::error!("resolve(): {}", reason);
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    pub fn unresolved(&self) -> Vec<String> {
        let mut unresolved: Vec<String> = Vec::new();
        for sym in self.dynsym.iter() {
            if let Ok(name) = self.dynstr.get_name(sym.name_offset()) {
                if !name.is_empty() && sym.is_undefined() {
                    unresolved.push(name.to_string());
                }
            }
        }
        unresolved
    }

    /// Returns the file descriptor of the dynamic library.
    pub fn dependencies(&self) -> BTreeMap<String, Option<Arc<Mutex<Self>>>> {
        self.dependencies.clone()
    }

    /// Binds a dependency to the dynamic library.
    pub fn bind_dependency(
        &mut self,
        name: String,
        library: Arc<Mutex<Self>>,
    ) -> Result<(), Error> {
        match self.dependencies.get(&name) {
            Some(None) => {
                self.dependencies.insert(name, Some(library));
                Ok(())
            },
            Some(Some(_)) => {
                let reason: &str = "dependency already loaded";
                ::nvx::error!("load_dependency(): {}", reason);
                Err(Error::new(ErrorCode::BadFile, reason))
            },
            None => {
                let reason: &str = "dependency not listed";
                ::nvx::error!("load_dependency(): {}", reason);
                Err(Error::new(ErrorCode::BadFile, reason))
            },
        }
    }

    /// Takes all dependencies of the dynamic library.
    pub fn take_dependencies(&mut self) -> Vec<(String, Arc<Mutex<Self>>)> {
        let mut dependencies: Vec<(String, Arc<Mutex<Self>>)> = Vec::new();
        for (name, library) in self.dependencies.iter() {
            if let Some(library) = library {
                dependencies.push((name.clone(), library.clone()));
            }
        }
        dependencies
    }
}

impl fmt::Debug for DynamicLibrary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DlFile {{ name={:?}, fd={:?} }}", self.filename, self.fd)
    }
}

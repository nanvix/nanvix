// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ffi::CStr,
    mem,
    ops::{
        Deref,
        DerefMut,
    },
};
// Architecture-independent ELF symbol/section constants (identical between ELF32 and ELF64).
use ::goblin::elf::{
    section_header::SHN_UNDEF,
    sym::{
        st_bind,
        st_type,
        STB_GLOBAL,
        STB_LOCAL,
        STB_WEAK,
        STT_FUNC,
        STT_OBJECT,
    },
};
// The guest dynamic linker only ever processes objects built for its own ABI, so the
// relocation-type constants, relocation-entry record, and symbol record are selected at compile
// time by the target architecture.
#[cfg(target_arch = "x86")]
use ::goblin::{
    elf::reloc::{
        R_386_16,
        R_386_32,
        R_386_32PLT,
        R_386_8,
        R_386_COPY,
        R_386_GLOB_DAT,
        R_386_GOT32,
        R_386_GOT32X,
        R_386_GOTOFF,
        R_386_GOTPC,
        R_386_IRELATIVE,
        R_386_JMP_SLOT,
        R_386_NONE,
        R_386_NUM,
        R_386_PC16,
        R_386_PC32,
        R_386_PC8,
        R_386_PLT32,
        R_386_RELATIVE,
        R_386_SIZE32,
        R_386_TLS_DESC,
        R_386_TLS_DESC_CALL,
        R_386_TLS_DTPMOD32,
        R_386_TLS_DTPOFF32,
        R_386_TLS_GD,
        R_386_TLS_GD_32,
        R_386_TLS_GD_CALL,
        R_386_TLS_GD_POP,
        R_386_TLS_GD_PUSH,
        R_386_TLS_GOTDESC,
        R_386_TLS_GOTIE,
        R_386_TLS_IE,
        R_386_TLS_IE_32,
        R_386_TLS_LDM,
        R_386_TLS_LDM_32,
        R_386_TLS_LDM_CALL,
        R_386_TLS_LDM_POP,
        R_386_TLS_LDM_PUSH,
        R_386_TLS_LDO_32,
        R_386_TLS_LE,
        R_386_TLS_LE_32,
        R_386_TLS_TPOFF,
        R_386_TLS_TPOFF32,
    },
    elf32::{
        reloc::{
            r_sym,
            r_type,
            Rel as RawReloc,
        },
        sym::Sym,
    },
};
#[cfg(target_arch = "aarch64")]
use ::goblin::{
    elf::reloc::{
        R_AARCH64_ABS64,
        R_AARCH64_GLOB_DAT,
        R_AARCH64_JUMP_SLOT,
        R_AARCH64_NONE,
        R_AARCH64_PREL64,
        R_AARCH64_RELATIVE,
    },
    elf64::{
        reloc::{
            r_sym,
            r_type,
            Rela as RawReloc,
        },
        sym::Sym,
    },
};
#[cfg(target_arch = "x86_64")]
use ::goblin::{
    elf::reloc::{
        R_X86_64_16,
        R_X86_64_32,
        R_X86_64_32S,
        R_X86_64_64,
        R_X86_64_8,
        R_X86_64_COPY,
        R_X86_64_GLOB_DAT,
        R_X86_64_GOT32,
        R_X86_64_GOTPCREL,
        R_X86_64_IRELATIVE,
        R_X86_64_JUMP_SLOT,
        R_X86_64_NONE,
        R_X86_64_PC16,
        R_X86_64_PC32,
        R_X86_64_PC64,
        R_X86_64_PC8,
        R_X86_64_PLT32,
        R_X86_64_RELATIVE,
        R_X86_64_SIZE32,
    },
    elf64::{
        reloc::{
            r_sym,
            r_type,
            Rela as RawReloc,
        },
        sym::Sym,
    },
};
use ::num_enum::{
    FromPrimitive,
    TryFromPrimitive,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

/// Width of an ELF address/offset field for the active guest ABI.
#[cfg(target_arch = "x86")]
pub type ElfAddr = u32;
/// Width of an ELF address/offset field for the active guest ABI.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub type ElfAddr = u64;

//==================================================================================================
// Symbol Table
//==================================================================================================

///
/// # Description
///
/// A structure that represents a symbol table in an ELF file.
///
pub struct SymbolTable {
    /// Pointer to the symbol table.
    ptr: *mut Symbol,
    /// Length of the symbol table.
    length: usize,
}

unsafe impl Send for SymbolTable {}
unsafe impl Sync for SymbolTable {}

impl SymbolTable {
    ///
    /// # Description
    ///
    /// Creates a new symbol table from a pointer and a length.
    ///
    /// # Parameters
    ///
    /// - `ptr`: A pointer to the symbol table.
    /// - `len`: The length of the symbol table.
    ///
    /// # Returns
    ///
    /// A new `SymbolTable` instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because does not perform any checks on whether the pointer is valid
    /// or not.
    ///
    /// This function is safe to use if all the following conditions are met:
    /// - `ptr` points to a valid symbol table of `len` symbols.
    ///
    pub unsafe fn from_raw_parts(ptr: *mut Symbol, len: usize) -> Self {
        SymbolTable { ptr, length: len }
    }
}

impl Deref for SymbolTable {
    type Target = [Symbol];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.length) }
    }
}

impl DerefMut for SymbolTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.length) }
    }
}

//==================================================================================================
// String Table
//==================================================================================================

///
/// # Description
///
/// A structure that represents a string table in an ELF file.
///
pub struct StringTable {
    /// Pointer to the string table.
    ptr: *const u8,
    /// Length of the string table.
    len: usize,
}

unsafe impl Send for StringTable {}
unsafe impl Sync for StringTable {}

impl StringTable {
    ///
    /// # Description
    ///
    /// Creates a new string table from a pointer and a length.
    ///
    /// # Parameters
    ///
    /// - `ptr`: A pointer to the string table.
    /// - `len`: The length of the string table.
    ///
    /// # Returns
    ///
    /// A new `StringTable` instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because does not perform any checks on whether the pointer is valid
    /// or not.
    ///
    /// This function is safe to use if all the following conditions are met:
    /// - `ptr` points to a valid string table of `len` bytes.
    ///
    pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self {
        StringTable { ptr, len }
    }

    ///
    /// # Description
    ///
    /// Retrieves the name of a symbol from the string table.
    ///
    /// # Parameters
    ///
    /// - `index`: The index of the symbol in the string table.
    ///
    /// # Returns
    ///
    /// A result containing the name of the symbol as a byte slice.
    ///
    pub fn get_name_bytes(&self, index: usize) -> Option<&[u8]> {
        let dynamic_symbols_names: &[u8] = &self[..];

        // Check if index is out of bounds.
        if index >= dynamic_symbols_names.len() {
            return None;
        }

        // Get the name of the symbol.
        Some(&dynamic_symbols_names[index..])
    }

    ///
    /// # Description
    ///
    /// Retrieves the name of a symbol from the string table.
    ///
    /// # Parameters
    ///
    /// - `index`: The index of the symbol in the string table.
    ///
    /// # Returns
    ///
    /// A result containing the name of the symbol as a string slice.
    ///
    pub fn get_name(&self, index: usize) -> Result<&str, Error> {
        let name: &[u8] = match self.get_name_bytes(index) {
            Some(name) => name,
            None => {
                let reason: &str = "index out of bounds";
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        match CStr::from_bytes_until_nul(name) {
            Ok(cstr) => match cstr.to_str() {
                Ok(str) => Ok(str),
                Err(_error) => {
                    let reason: &str = "invalid utf-8 sequence";
                    Err(Error::new(ErrorCode::ValueOutOfRange, reason))
                },
            },
            Err(_error) => {
                let reason: &str = "invalid c string";
                Err(Error::new(ErrorCode::ValueOutOfRange, reason))
            },
        }
    }
}

impl Deref for StringTable {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

//==================================================================================================
// Relocation Table
//==================================================================================================

///
/// # Description
///
/// A structure that represents a relocation table in an ELF file.
///
pub struct RelocationTable {
    ptr: *mut RelocationEntry,
    len: usize,
}

unsafe impl Send for RelocationTable {}
unsafe impl Sync for RelocationTable {}

impl RelocationTable {
    ///
    /// # Description
    ///
    /// Creates a new relocation table from a pointer and a length.
    ///
    /// # Parameters
    ///
    /// - `ptr`: A pointer to the relocation table.
    /// - `len`: The length of the relocation table.
    ///
    /// # Returns
    ///
    /// A new `RelocationTable` instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because does not perform any checks on whether the pointer is valid
    /// or not.
    ///
    /// This function is safe to use if all the following conditions are met:
    /// - `ptr` points to a valid relocation table of `len` relocations.
    ///
    pub unsafe fn from_raw_parts(ptr: *mut RelocationEntry, len: usize) -> Self {
        RelocationTable { ptr, len }
    }
}

impl Deref for RelocationTable {
    type Target = [RelocationEntry];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for RelocationTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

//==================================================================================================
// Symbol Type
//==================================================================================================

///
/// # Description
///
/// A structure that represents a symbol type in an ELF file.
///
#[repr(u8)]
#[derive(Debug, FromPrimitive)]
pub enum SymbolType {
    /// Unsupported.
    #[num_enum(default)]
    Undefined,
    /// Function symbol type.
    Function = STT_FUNC,
    /// Object symbol type.
    Object = STT_OBJECT,
}

//==================================================================================================
// Symbol Binding
//==================================================================================================

///
/// # Description
///
/// A high-level representation of the binding attribute encoded in the high 4 bits of an
/// ELF symbol's `st_info` field.
///
/// Bindings control how the link editor and the dynamic loader treat a symbol when multiple
/// definitions are visible and when no definition can be found.
///
/// Per the System V ABI (gABI, chapter "Symbol Table"):
/// - `STB_LOCAL` — definitions are not visible to other object files.
/// - `STB_GLOBAL` — definitions are visible to all combined object files; an undefined
///   global reference that cannot be resolved is an error.
/// - `STB_WEAK` — like global, but with lower precedence; **an undefined weak reference
///   that cannot be resolved at dynamic-link time is silently taken to be the value zero**
///   (or `NULL` for function symbols). The dynamic loader is required to honour this rule.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum SymbolBinding {
    /// Local symbol — invisible outside the defining object file.
    Local = STB_LOCAL,
    /// Global symbol — visible to all object files.
    Global = STB_GLOBAL,
    /// Weak symbol — global with lower precedence; an unresolved weak undefined reference
    /// is resolved to address zero per the System V ABI.
    Weak = STB_WEAK,
    /// Any other binding value defined by the platform or the processor supplement
    /// (for example reserved or processor-specific bindings) that we do not interpret
    /// further.
    #[num_enum(default)]
    Other,
}

//==================================================================================================
// Symbol
//==================================================================================================

///
/// # Description
///
/// A structure that represents a symbol in an ELF file.
///
#[repr(C)]
#[derive(Debug)]
pub struct Symbol(Sym);

::static_assert::assert_eq_size!(Symbol, mem::size_of::<Sym>());
::static_assert::assert_eq_align!(Symbol, mem::align_of::<Sym>());

impl Symbol {
    ///
    /// # Description
    ///
    /// Get the offset of the symbol's name within the associated string table.
    ///
    /// # Returns
    ///
    /// The offset of the symbol's name within the associated string table.
    ///
    pub fn name_offset(&self) -> usize {
        self.0.st_name as usize
    }

    ///
    /// # Description
    ///
    /// Get the type of the symbol.
    ///
    /// # Returns
    ///
    /// The type of the symbol.
    ///
    pub fn typ(&self) -> SymbolType {
        st_type(self.0.st_info).into()
    }

    ///
    /// # Description
    ///
    /// Returns the binding attribute of the symbol (high 4 bits of `st_info`).
    ///
    /// # Returns
    ///
    /// A [`SymbolBinding`] value. Bindings the loader does not interpret further are
    /// returned as [`SymbolBinding::Other`].
    ///
    pub fn binding(&self) -> SymbolBinding {
        st_bind(self.0.st_info).into()
    }

    ///
    /// # Description
    ///
    /// Get the value of the symbol.
    ///
    /// # Returns
    ///
    /// The value of the symbol.
    ///
    pub fn value(&self) -> ElfAddr {
        self.0.st_value
    }

    ///
    /// # Description
    ///
    /// Get the size of the symbol.
    ///
    /// # Returns
    ///
    /// The size of the symbol.
    ///
    pub fn size(&self) -> ElfAddr {
        self.0.st_size
    }

    ///
    /// # Description
    ///
    /// Tests if the symbol is undefined.
    ///
    /// # Returns
    ///
    /// True if the symbol is undefined, false otherwise.
    ///
    pub fn is_undefined(&self) -> bool {
        self.0.st_shndx as u32 == SHN_UNDEF
    }

    ///
    /// # Description
    ///
    /// Tests if the symbol has weak binding (`STB_WEAK`).
    ///
    /// Per the System V ABI, a weak undefined symbol that cannot be resolved at
    /// dynamic-link time is silently resolved to address zero. Loaders that consume this
    /// helper are expected to follow that rule rather than reporting a lookup failure.
    ///
    /// # Returns
    ///
    /// `true` if the symbol's binding is `STB_WEAK`, `false` otherwise.
    ///
    pub fn is_weak(&self) -> bool {
        self.binding() == SymbolBinding::Weak
    }

    ///
    /// # Description
    ///
    /// Sets the value of the symbol.
    ///
    /// # Parameters
    ///
    /// - `value`: The new value of the symbol.
    ///
    /// # Returns
    ///
    pub fn resolve(&mut self, value: ElfAddr) {
        self.0.st_value = value;
    }
}

//==================================================================================================
// Relocation Types
//==================================================================================================

#[cfg(target_arch = "x86")]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum RelocationType {
    /// 8-bit relocation.
    R_386_8 = R_386_8 as u8,
    /// 16-bit relocation.
    R_386_16 = R_386_16 as u8,
    /// 32-bit relocation.
    R_386_32 = R_386_32 as u8,
    /// Direct 32-bit for PLT.
    R_386_32PLT = R_386_32PLT as u8,
    /// Copy symbol at runtime.
    R_386_COPY = R_386_COPY as u8,
    /// Create GOT entry.
    R_386_GLOB_DAT = R_386_GLOB_DAT as u8,
    /// 32-bit GOT entry.
    R_386_GOT32 = R_386_GOT32 as u8,
    /// Load from 32-bit GOT entry, relaxable.
    R_386_GOT32X = R_386_GOT32X as u8,
    /// 32-bit offset to GOT.
    R_386_GOTOFF = R_386_GOTOFF as u8,
    /// 32-bit PC relative offset to GOT.
    R_386_GOTPC = R_386_GOTPC as u8,
    /// Adjust indirectly by program base.
    R_386_IRELATIVE = R_386_IRELATIVE as u8,
    /// Create PLT entry.
    R_386_JMP_SLOT = R_386_JMP_SLOT as u8,
    /// No relocation.
    R_386_NONE = R_386_NONE as u8,
    /// Keep this the last entry.
    R_386_NUM = R_386_NUM as u8,
    /// PC relative 8-bit.
    R_386_PC8 = R_386_PC8 as u8,
    /// PC relative 16-bit.
    R_386_PC16 = R_386_PC16 as u8,
    /// PC relative 32-bit.
    R_386_PC32 = R_386_PC32 as u8,
    /// 32-bit PLT address.
    R_386_PLT32 = R_386_PLT32 as u8,
    /// Adjust by program base.
    R_386_RELATIVE = R_386_RELATIVE as u8,
    /// 32-bit symbol size.
    R_386_SIZE32 = R_386_SIZE32 as u8,
    /// TLS descriptor containing pointer to code and argument.
    R_386_TLS_DESC = R_386_TLS_DESC as u8,
    /// Marker of call through TLS descriptor for relaxation.
    R_386_TLS_DESC_CALL = R_386_TLS_DESC_CALL as u8,
    /// ID of module containing symbol.
    R_386_TLS_DTPMOD32 = R_386_TLS_DTPMOD32 as u8,
    /// Offset in TLS block.
    R_386_TLS_DTPOFF32 = R_386_TLS_DTPOFF32 as u8,
    /// Direct 32-bit for GNU version of general dynamic thread local data.
    R_386_TLS_GD = R_386_TLS_GD as u8,
    /// Direct 32-bit for general dynamic thread local data.
    R_386_TLS_GD_32 = R_386_TLS_GD_32 as u8,
    /// Relocation for call to __tls_get_addr().
    R_386_TLS_GD_CALL = R_386_TLS_GD_CALL as u8,
    /// Tag for popl in GD TLS code.
    R_386_TLS_GD_POP = R_386_TLS_GD_POP as u8,
    /// Tag for pushl in GD TLS code.
    R_386_TLS_GD_PUSH = R_386_TLS_GD_PUSH as u8,
    /// GOT offset for TLS descriptor.
    R_386_TLS_GOTDESC = R_386_TLS_GOTDESC as u8,
    /// GOT entry for static TLS block offset.
    R_386_TLS_GOTIE = R_386_TLS_GOTIE as u8,
    /// Address of GOT entry for static TLS block offset.
    R_386_TLS_IE = R_386_TLS_IE as u8,
    /// GOT entry for negated static TLS block offset.
    R_386_TLS_IE_32 = R_386_TLS_IE_32 as u8,
    /// Direct 32-bit for GNU version of local dynamic thread local data in LE code.
    R_386_TLS_LDM = R_386_TLS_LDM as u8,
    /// Direct 32-bit for local dynamic thread local data in LE code.
    R_386_TLS_LDM_32 = R_386_TLS_LDM_32 as u8,
    /// Relocation for call to __tls_get_addr() in LDM code.
    R_386_TLS_LDM_CALL = R_386_TLS_LDM_CALL as u8,
    /// Tag for popl in LDM TLS code.
    R_386_TLS_LDM_POP = R_386_TLS_LDM_POP as u8,
    /// Tag for pushl in LDM TLS code.
    R_386_TLS_LDM_PUSH = R_386_TLS_LDM_PUSH as u8,
    /// Offset relative to TLS block.
    R_386_TLS_LDO_32 = R_386_TLS_LDO_32 as u8,
    /// Offset relative to static TLS block.
    R_386_TLS_LE = R_386_TLS_LE as u8,
    /// Negated offset relative to static TLS block.
    R_386_TLS_LE_32 = R_386_TLS_LE_32 as u8,
    /// Offset in static TLS block.
    R_386_TLS_TPOFF = R_386_TLS_TPOFF as u8,
    /// Negated offset in static TLS block.
    R_386_TLS_TPOFF32 = R_386_TLS_TPOFF32 as u8,
}

/// x86-64 (RELA) relocation types recognized by the guest dynamic linker. Only the subset that the
/// freestanding fixtures and the bundled libc can emit is enumerated; any other type is reported as
/// an unsupported relocation by the resolver.
#[cfg(target_arch = "x86_64")]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum RelocationType {
    /// Direct 8-bit.
    R_X86_64_8 = R_X86_64_8 as u8,
    /// Direct 16-bit zero extended.
    R_X86_64_16 = R_X86_64_16 as u8,
    /// Direct 32-bit zero extended.
    R_X86_64_32 = R_X86_64_32 as u8,
    /// Direct 32-bit sign extended.
    R_X86_64_32S = R_X86_64_32S as u8,
    /// Direct 64-bit.
    R_X86_64_64 = R_X86_64_64 as u8,
    /// Copy symbol at runtime.
    R_X86_64_COPY = R_X86_64_COPY as u8,
    /// Create GOT entry.
    R_X86_64_GLOB_DAT = R_X86_64_GLOB_DAT as u8,
    /// 32-bit GOT entry.
    R_X86_64_GOT32 = R_X86_64_GOT32 as u8,
    /// 32-bit signed PC relative offset to GOT.
    R_X86_64_GOTPCREL = R_X86_64_GOTPCREL as u8,
    /// Adjust indirectly by program base.
    R_X86_64_IRELATIVE = R_X86_64_IRELATIVE as u8,
    /// Create PLT entry.
    R_X86_64_JUMP_SLOT = R_X86_64_JUMP_SLOT as u8,
    /// No relocation.
    R_X86_64_NONE = R_X86_64_NONE as u8,
    /// PC relative 8-bit signed.
    R_X86_64_PC8 = R_X86_64_PC8 as u8,
    /// PC relative 16-bit signed.
    R_X86_64_PC16 = R_X86_64_PC16 as u8,
    /// PC relative 32-bit signed.
    R_X86_64_PC32 = R_X86_64_PC32 as u8,
    /// PC relative 64-bit.
    R_X86_64_PC64 = R_X86_64_PC64 as u8,
    /// 32-bit PLT address.
    R_X86_64_PLT32 = R_X86_64_PLT32 as u8,
    /// Adjust by program base.
    R_X86_64_RELATIVE = R_X86_64_RELATIVE as u8,
    /// Size of symbol plus 32-bit addend.
    R_X86_64_SIZE32 = R_X86_64_SIZE32 as u8,
}

/// AArch64 ELF64/RELA relocation types used by the guest dynamic linker.
#[cfg(target_arch = "aarch64")]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum RelocationType {
    /// No relocation.
    R_AARCH64_NONE = R_AARCH64_NONE,
    /// Direct 64-bit relocation.
    R_AARCH64_ABS64 = R_AARCH64_ABS64,
    /// PC-relative 64-bit relocation.
    R_AARCH64_PREL64 = R_AARCH64_PREL64,
    /// Create GOT entry.
    R_AARCH64_GLOB_DAT = R_AARCH64_GLOB_DAT,
    /// Create PLT entry.
    R_AARCH64_JUMP_SLOT = R_AARCH64_JUMP_SLOT,
    /// Adjust by program base.
    R_AARCH64_RELATIVE = R_AARCH64_RELATIVE,
}

//==================================================================================================
// Relocation Entry
//==================================================================================================

///
/// # Description
///
/// A structure that represents a relocation entry in an ELF file.
///
#[repr(C)]
#[derive(Debug)]
pub struct RelocationEntry(RawReloc);

::static_assert::assert_eq_size!(RelocationEntry, mem::size_of::<RawReloc>());
::static_assert::assert_eq_align!(RelocationEntry, mem::align_of::<RawReloc>());

impl RelocationEntry {
    ///
    /// # Description
    ///
    /// Get the type of the relocation entry.
    ///
    /// # Returns
    ///
    /// The type of the relocation entry.
    ///
    ///
    pub fn typ(&self) -> Result<RelocationType, Error> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let typ: u8 = r_type(self.0.r_info) as u8;
        #[cfg(target_arch = "aarch64")]
        let typ: u32 = r_type(self.0.r_info);
        typ.try_into().map_err(|_error| {
            let reason: &str = "invalid relocation type";
            Error::new(ErrorCode::ValueOutOfRange, reason)
        })
    }

    ///
    /// # Description
    ///
    /// Get the symbol index of the relocation entry.
    ///
    /// # Returns
    ///
    /// The symbol index of the relocation entry.
    ///
    pub fn symbol_index(&self) -> u32 {
        r_sym(self.0.r_info)
    }

    ///
    /// # Description
    ///
    /// Get the offset of the relocation entry.
    ///
    /// # Returns
    ///
    /// The offset of the relocation entry.
    ///
    pub fn offset(&self) -> ElfAddr {
        self.0.r_offset
    }

    ///
    /// # Description
    ///
    /// Gets the explicit addend of an ELF64 RELA relocation entry.
    ///
    /// # Returns
    ///
    /// The signed addend stored in the relocation entry.
    ///
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn addend(&self) -> i64 {
        self.0.r_addend
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf32::STT_NOTYPE;
    #[cfg(target_arch = "x86")]
    use ::goblin::elf32::sym::Sym;
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    use ::goblin::elf64::sym::Sym;

    /// Builds a Symbol with the given binding (high 4 bits) and type (low 4 bits),
    /// and the given section index.
    fn make_symbol(binding: u8, sym_type: u8, st_shndx: u16) -> Symbol {
        Symbol(Sym {
            st_name: 0,
            st_value: 0,
            st_size: 0,
            st_info: (binding << 4) | (sym_type & 0xf),
            st_other: 0,
            st_shndx,
        })
    }

    #[test]
    fn binding_decodes_local_global_weak() {
        let local = make_symbol(STB_LOCAL, STT_FUNC, 1);
        let global = make_symbol(STB_GLOBAL, STT_FUNC, 1);
        let weak = make_symbol(STB_WEAK, STT_FUNC, 1);

        assert_eq!(local.binding(), SymbolBinding::Local);
        assert_eq!(global.binding(), SymbolBinding::Global);
        assert_eq!(weak.binding(), SymbolBinding::Weak);
    }

    #[test]
    fn binding_falls_back_to_other_for_unknown_values() {
        // Reserved/processor-specific binding values must not be classified as a known
        // binding; the loader is expected to treat them conservatively (i.e., not weak).
        let exotic = make_symbol(10, STT_FUNC, 1);
        assert_eq!(exotic.binding(), SymbolBinding::Other);
        assert!(!exotic.is_weak());
    }

    #[test]
    fn is_weak_matches_stb_weak_only() {
        assert!(!make_symbol(STB_LOCAL, STT_FUNC, 0).is_weak());
        assert!(!make_symbol(STB_GLOBAL, STT_FUNC, 0).is_weak());
        assert!(make_symbol(STB_WEAK, STT_FUNC, 0).is_weak());
        assert!(make_symbol(STB_WEAK, STT_OBJECT, 0).is_weak());
    }

    #[test]
    fn is_undefined_and_is_weak_are_independent() {
        // Weak + undefined is the case the dynamic loader must resolve to 0.
        let weak_undef = make_symbol(STB_WEAK, STT_NOTYPE, SHN_UNDEF as u16);
        assert!(weak_undef.is_undefined());
        assert!(weak_undef.is_weak());

        // Weak + defined: a definition that may be overridden by a strong one.
        let weak_def = make_symbol(STB_WEAK, STT_FUNC, 1);
        assert!(!weak_def.is_undefined());
        assert!(weak_def.is_weak());

        // Strong + undefined: must remain an error in the loader.
        let strong_undef = make_symbol(STB_GLOBAL, STT_NOTYPE, SHN_UNDEF as u16);
        assert!(strong_undef.is_undefined());
        assert!(!strong_undef.is_weak());
    }

    #[test]
    fn binding_does_not_depend_on_type_nibble() {
        // The low 4 bits encode the symbol type; the high 4 bits encode the binding.
        // Changing the type must not affect the binding decoding.
        for sym_type in [STT_NOTYPE, STT_OBJECT, STT_FUNC] {
            assert_eq!(make_symbol(STB_WEAK, sym_type, 1).binding(), SymbolBinding::Weak);
        }
    }
}

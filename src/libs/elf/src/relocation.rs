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
use ::goblin::{
    elf::{
        reloc::{
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
        section_header::SHN_UNDEF,
        sym::{
            st_type,
            STT_FUNC,
            STT_OBJECT,
        },
    },
    elf32::{
        reloc::{
            r_sym,
            r_type,
            Rel,
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
    /// Get the value of the symbol.
    ///
    /// # Returns
    ///
    /// The value of the symbol.
    ///
    pub fn value(&self) -> u32 {
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
    pub fn size(&self) -> u32 {
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
    /// Sets the value of the symbol.
    ///
    /// # Parameters
    ///
    /// - `value`: The new value of the symbol.
    ///
    /// # Returns
    ///
    pub fn resolve(&mut self, value: u32) {
        self.0.st_value = value;
    }
}

//==================================================================================================
// Relocation Types
//==================================================================================================

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
pub struct RelocationEntry(Rel);

::static_assert::assert_eq_size!(RelocationEntry, mem::size_of::<Rel>());
::static_assert::assert_eq_align!(RelocationEntry, mem::align_of::<Rel>());

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
        // NOTE: Per the ELF spec, r_type() is guaranteed to be a u8.
        let typ: u8 = r_type(self.0.r_info) as u8;
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
    pub fn offset(&self) -> u32 {
        self.0.r_offset
    }
}

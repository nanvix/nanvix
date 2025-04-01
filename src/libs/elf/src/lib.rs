// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

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
            R_386_GLOB_DAT,
            R_386_JMP_SLOT,
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
use ::num_enum::FromPrimitive;
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
    /// This function is unsafe because does not perform any checks on wether the pointer is valid
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
    /// This function is unsafe because does not perform any checks on wether the pointer is valid
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
    /// This function is unsafe because does not perform any checks on wether the pointer is valid
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

::sys::static_assert_size!(Symbol, mem::size_of::<Sym>());
::sys::static_assert_alignment!(Symbol, mem::align_of::<Sym>());

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
        self.0.st_shndx as u32 == SHN_UNDEF && self.0.st_value == 0
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

::sys::static_assert_size!(RelocationEntry, mem::size_of::<Rel>());
::sys::static_assert_alignment!(RelocationEntry, mem::align_of::<Rel>());

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
    pub fn typ(&self) -> u32 {
        r_type(self.0.r_info)
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

    ///
    /// # Description
    ///
    /// Binds the relocation entry to a value.
    ///
    /// # Parameters
    ///
    /// - `base`: The base address of the shared object.
    /// - `value`: The value to bind to the relocation entry.
    ///
    /// # Returns
    ///
    /// If successful, returns `Ok(())`. Otherwise, returns an error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    ///
    /// It is safe to use this function if all the following conditions are met:
    /// - The `base` address points to a valid location.
    ///
    pub unsafe fn bind(&mut self, base: u32, value: u32) -> Result<(), Error> {
        match self.typ() {
            R_386_JMP_SLOT | R_386_GLOB_DAT => {
                let ptr: *mut u32 = (base + self.offset()) as *mut u32;
                *ptr = value;
                Ok(())
            },
            _ => {
                let reason: &str = "unsupported relocation type";
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
        }
    }
}

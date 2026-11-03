// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF32 Symbol Table Parser
//!
//! Extracts function symbols from an ELF32 binary stored in a byte buffer.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::elf32::{
    Elf32Fhdr,
    Elf32Shdr,
    Elf32Sym,
    EI_CLASS,
    EI_DATA,
    ELFCLASS32,
    ELFDATA2LSB,
    ELFMAG0,
    ELFMAG1,
    ELFMAG2,
    ELFMAG3,
    SHT_DYNSYM,
    SHT_SYMTAB,
    STT_FUNC,
    STT_NOTYPE,
    ST_TYPE_MASK,
};
use ::alloc::{
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use ::core::mem::size_of;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Elf32FuncSymbol
//==================================================================================================

/// A function symbol extracted from an ELF32 binary.
#[derive(Debug, Clone)]
pub struct Elf32FuncSymbol {
    /// Symbol virtual address.
    pub addr: u32,
    /// Symbol size in bytes (0 if unknown).
    pub size: u32,
    /// Symbol name.
    pub name: String,
}

//==================================================================================================
// parse_elf32_func_symbols()
//==================================================================================================

/// Parses function symbols from an ELF32 binary in a byte buffer.
///
/// # Description
///
/// Scans section headers for `.symtab` (`SHT_SYMTAB`) or `.dynsym` (`SHT_DYNSYM`),
/// preferring `.symtab` when present. Extracts symbols whose type is `STT_FUNC` or
/// `STT_NOTYPE` (assembly entry points) with a non-zero address.
///
/// # Parameters
///
/// - `data`: Raw bytes of a complete ELF32 file.
///
/// # Returns
///
/// A vector of [`Elf32FuncSymbol`] entries on success.
///
/// # Errors
///
/// - `InvalidExecutableFormat` — if the ELF magic, class, or ELF header invariants are invalid.
///
/// # Notes
///
/// Parsing of section headers and symbol tables is best-effort. If these structures are
/// malformed, truncated, or point outside `data`, this function may stop scanning and return
/// the symbols collected so far (including an empty vector) instead of returning an error.
///
pub fn parse_elf32_func_symbols(data: &[u8]) -> Result<Vec<Elf32FuncSymbol>, Error> {
    let mut out: Vec<Elf32FuncSymbol> = Vec::new();

    // Minimum ELF32 header size.
    if data.len() < Elf32Fhdr::SIZE {
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "ELF data too short"));
    }

    // Check ELF magic.
    if data[0] != ELFMAG0 || data[1] != ELFMAG1 || data[2] != ELFMAG2 || data[3] != ELFMAG3 {
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "invalid ELF magic"));
    }

    // ELF32 class check.
    if data[EI_CLASS] != ELFCLASS32 {
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "not ELF32"));
    }

    // Little-endian check (read_u32/read_u16 assume LE).
    if data[EI_DATA] != ELFDATA2LSB {
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "not little-endian"));
    }

    let e_shoff: usize = read_u32(data, Elf32Fhdr::OFFSET_E_SHOFF)? as usize;
    let e_shentsize: usize = read_u16(data, Elf32Fhdr::OFFSET_E_SHENTSIZE)? as usize;
    let e_shnum: usize = read_u16(data, Elf32Fhdr::OFFSET_E_SHNUM)? as usize;

    if e_shoff == 0 || e_shnum == 0 || e_shentsize < Elf32Shdr::SIZE {
        return Err(Error::new(
            ErrorCode::InvalidExecutableFormat,
            "invalid section header table parameters",
        ));
    }

    // Locate .symtab or .dynsym section.
    let mut symtab_offset: usize = 0;
    let mut symtab_size: usize = 0;
    let mut symtab_entsize: usize = 0;
    let mut symtab_link: usize = 0;

    for i in 0..e_shnum {
        let sh: usize = match e_shentsize
            .checked_mul(i)
            .and_then(|v| e_shoff.checked_add(v))
        {
            Some(v) => v,
            None => break,
        };
        let sh_end: usize = match sh.checked_add(Elf32Shdr::SIZE) {
            Some(v) => v,
            None => break,
        };
        if sh_end > data.len() {
            break;
        }

        let sh_type: u32 = read_u32(data, sh + Elf32Shdr::OFFSET_SH_TYPE)?;

        if sh_type == SHT_SYMTAB {
            symtab_offset = read_u32(data, sh + Elf32Shdr::OFFSET_SH_OFFSET)? as usize;
            symtab_size = read_u32(data, sh + Elf32Shdr::OFFSET_SH_SIZE)? as usize;
            symtab_entsize = read_u32(data, sh + Elf32Shdr::OFFSET_SH_ENTSIZE)? as usize;
            symtab_link = read_u32(data, sh + Elf32Shdr::OFFSET_SH_LINK)? as usize;
            break;
        } else if sh_type == SHT_DYNSYM && symtab_offset == 0 {
            symtab_offset = read_u32(data, sh + Elf32Shdr::OFFSET_SH_OFFSET)? as usize;
            symtab_size = read_u32(data, sh + Elf32Shdr::OFFSET_SH_SIZE)? as usize;
            symtab_entsize = read_u32(data, sh + Elf32Shdr::OFFSET_SH_ENTSIZE)? as usize;
            symtab_link = read_u32(data, sh + Elf32Shdr::OFFSET_SH_LINK)? as usize;
        }
    }

    if symtab_offset == 0 || symtab_entsize < Elf32Sym::SIZE {
        // No symbol table found — return empty (not an error).
        return Ok(out);
    }

    // Read .strtab referenced by the symbol table section.
    let strtab_sh: usize = match e_shentsize
        .checked_mul(symtab_link)
        .and_then(|v| e_shoff.checked_add(v))
    {
        Some(v) => v,
        None => {
            return Err(Error::new(ErrorCode::InvalidArgument, "strtab section index overflow"));
        },
    };
    let strtab_sh_end: usize = match strtab_sh
        .checked_add(Elf32Shdr::OFFSET_SH_SIZE)
        .and_then(|v| v.checked_add(size_of::<u32>()))
    {
        Some(v) => v,
        None => {
            return Err(Error::new(ErrorCode::InvalidArgument, "strtab section header overflow"));
        },
    };
    if strtab_sh_end > data.len() {
        return Err(Error::new(ErrorCode::InvalidArgument, "strtab section header out of bounds"));
    }
    let strtab_offset: usize = read_u32(data, strtab_sh + Elf32Shdr::OFFSET_SH_OFFSET)? as usize;
    let strtab_size: usize = read_u32(data, strtab_sh + Elf32Shdr::OFFSET_SH_SIZE)? as usize;

    let strtab_end: usize = match strtab_offset.checked_add(strtab_size) {
        Some(v) => v,
        None => {
            return Err(Error::new(ErrorCode::InvalidArgument, "strtab offset overflow"));
        },
    };
    if strtab_end > data.len() {
        return Err(Error::new(ErrorCode::InvalidArgument, "strtab extends past end of data"));
    }
    let strtab: &[u8] = &data[strtab_offset..strtab_end];

    // Parse symbol entries.
    let num_syms: usize = symtab_size / symtab_entsize;
    for i in 0..num_syms {
        let sym: usize = match symtab_entsize
            .checked_mul(i)
            .and_then(|v| symtab_offset.checked_add(v))
        {
            Some(v) => v,
            None => break,
        };
        let sym_end: usize = match sym.checked_add(Elf32Sym::SIZE) {
            Some(v) => v,
            None => break,
        };
        if sym_end > data.len() {
            break;
        }

        let st_name: usize = read_u32(data, sym + Elf32Sym::OFFSET_ST_NAME)? as usize;
        let st_value: u32 = read_u32(data, sym + Elf32Sym::OFFSET_ST_VALUE)?;
        let st_size: u32 = read_u32(data, sym + Elf32Sym::OFFSET_ST_SIZE)?;
        let st_info: u8 = data[sym + Elf32Sym::OFFSET_ST_INFO];

        // Keep STT_FUNC and STT_NOTYPE (assembly entry points).
        let st_type: u8 = st_info & ST_TYPE_MASK;
        if st_type != STT_FUNC && st_type != STT_NOTYPE {
            continue;
        }

        // Skip zero-address symbols.
        if st_value == 0 {
            continue;
        }

        // Read name from strtab.
        if st_name >= strtab.len() {
            continue;
        }
        let name_end: usize = strtab[st_name..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(strtab.len() - st_name);
        let name: String =
            String::from_utf8_lossy(&strtab[st_name..st_name + name_end]).to_string();

        if name.is_empty() {
            continue;
        }

        out.push(Elf32FuncSymbol {
            addr: st_value,
            size: st_size,
            name,
        });
    }

    Ok(out)
}

/// Reads a little-endian `u32` from `data` at byte position `offset`.
fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let end: usize = offset + size_of::<u32>();
    let bytes: &[u8] = data
        .get(offset..end)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "u32 read out of bounds"))?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "u32 slice conversion failed"))?,
    ))
}

/// Reads a little-endian `u16` from `data` at byte position `offset`.
fn read_u16(data: &[u8], offset: usize) -> Result<u16, Error> {
    let end: usize = offset + size_of::<u16>();
    let bytes: &[u8] = data
        .get(offset..end)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "u16 read out of bounds"))?;
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "u16 slice conversion failed"))?,
    ))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;

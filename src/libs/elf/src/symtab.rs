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
mod tests {
    use super::*;
    use crate::elf32::{
        SHT_STRTAB,
        STT_OBJECT,
    };

    /// Builds a minimal ELF32 binary with a `.symtab` and `.strtab`.
    fn build_test_elf(symbols: &[(u32, u32, u8, &str)]) -> Vec<u8> {
        // ELF header: 52 bytes.
        // Section headers start right after (offset 52).
        // We have 3 sections: null (index 0), .symtab (index 1), .strtab (index 2).
        let e_shoff: u32 = 52;
        let e_shentsize: u16 = 40;
        let e_shnum: u16 = 3;

        // Build strtab: NUL byte + concatenated NUL-terminated names.
        let mut strtab: Vec<u8> = vec![0u8];
        let mut name_offsets: Vec<usize> = Vec::new();
        for &(_, _, _, name) in symbols {
            name_offsets.push(strtab.len());
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }

        // Build symtab entries (16 bytes each).
        // Entry 0 is the null symbol.
        let sym_entsize: usize = 16;
        let mut symtab: Vec<u8> = vec![0u8; sym_entsize];
        for (i, &(addr, size, info, _)) in symbols.iter().enumerate() {
            let mut entry: Vec<u8> = vec![0u8; sym_entsize];
            entry[0..4].copy_from_slice(&(name_offsets[i] as u32).to_le_bytes());
            entry[4..8].copy_from_slice(&addr.to_le_bytes());
            entry[8..12].copy_from_slice(&size.to_le_bytes());
            entry[12] = info;
            symtab.extend_from_slice(&entry);
        }

        // Section data starts after section headers.
        let data_start: usize = e_shoff as usize + e_shnum as usize * e_shentsize as usize;
        let symtab_offset: usize = data_start;
        let strtab_offset: usize = symtab_offset + symtab.len();

        // Build section headers.
        let mut shdrs: Vec<u8> = vec![0u8; e_shentsize as usize]; // null section header

        // .symtab (SHT_SYMTAB = 2), sh_link = 2 (points to .strtab).
        let mut sh_symtab: Vec<u8> = vec![0u8; e_shentsize as usize];
        sh_symtab[4..8].copy_from_slice(&SHT_SYMTAB.to_le_bytes());
        sh_symtab[16..20].copy_from_slice(&(symtab_offset as u32).to_le_bytes());
        sh_symtab[20..24].copy_from_slice(&(symtab.len() as u32).to_le_bytes());
        sh_symtab[24..28].copy_from_slice(&2u32.to_le_bytes()); // sh_link -> strtab
        sh_symtab[36..40].copy_from_slice(&(sym_entsize as u32).to_le_bytes());
        shdrs.extend_from_slice(&sh_symtab);

        // .strtab section header.
        let mut sh_strtab: Vec<u8> = vec![0u8; e_shentsize as usize];
        sh_strtab[4..8].copy_from_slice(&SHT_STRTAB.to_le_bytes());
        sh_strtab[16..20].copy_from_slice(&(strtab_offset as u32).to_le_bytes());
        sh_strtab[20..24].copy_from_slice(&(strtab.len() as u32).to_le_bytes());
        shdrs.extend_from_slice(&sh_strtab);

        // Assemble the ELF.
        let total_size: usize = strtab_offset + strtab.len();
        let mut elf: Vec<u8> = vec![0u8; total_size];

        // ELF magic + class + endianness.
        elf[0..4].copy_from_slice(&[ELFMAG0, ELFMAG1, ELFMAG2, ELFMAG3]);
        elf[4] = ELFCLASS32;
        elf[5] = ELFDATA2LSB;
        elf[Elf32Fhdr::OFFSET_E_SHOFF..Elf32Fhdr::OFFSET_E_SHOFF + 4]
            .copy_from_slice(&e_shoff.to_le_bytes());
        elf[Elf32Fhdr::OFFSET_E_SHENTSIZE..Elf32Fhdr::OFFSET_E_SHENTSIZE + 2]
            .copy_from_slice(&e_shentsize.to_le_bytes());
        elf[Elf32Fhdr::OFFSET_E_SHNUM..Elf32Fhdr::OFFSET_E_SHNUM + 2]
            .copy_from_slice(&e_shnum.to_le_bytes());

        elf[e_shoff as usize..e_shoff as usize + shdrs.len()].copy_from_slice(&shdrs);
        elf[symtab_offset..symtab_offset + symtab.len()].copy_from_slice(&symtab);
        elf[strtab_offset..strtab_offset + strtab.len()].copy_from_slice(&strtab);

        elf
    }

    #[test]
    fn parse_basic_symbols() {
        let elf: Vec<u8> = build_test_elf(&[
            (0x1000, 0x100, STT_FUNC, "foo"),
            (0x2000, 0x200, STT_FUNC, "bar"),
        ]);

        let symbols: Vec<Elf32FuncSymbol> =
            parse_elf32_func_symbols(&elf).expect("should parse valid ELF");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].addr, 0x1000);
        assert_eq!(symbols[0].size, 0x100);
        assert_eq!(symbols[1].name, "bar");
    }

    #[test]
    fn parse_skips_non_func_symbols() {
        let elf: Vec<u8> = build_test_elf(&[
            (0x1000, 0x100, STT_FUNC, "func"),
            (0x2000, 0x50, STT_OBJECT, "object"),
            (0x3000, 0x0, STT_NOTYPE, "asm_entry"),
        ]);

        let symbols: Vec<Elf32FuncSymbol> =
            parse_elf32_func_symbols(&elf).expect("should parse valid ELF");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "func");
        assert_eq!(symbols[1].name, "asm_entry");
    }

    #[test]
    fn parse_rejects_truncated_elf() {
        let result: Result<Vec<Elf32FuncSymbol>, _> = parse_elf32_func_symbols(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_bad_magic() {
        let mut elf: Vec<u8> = build_test_elf(&[(0x1000, 0x100, STT_FUNC, "foo")]);
        elf[0] = 0x00;
        let result: Result<Vec<Elf32FuncSymbol>, _> = parse_elf32_func_symbols(&elf);
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_elf64() {
        let mut elf: Vec<u8> = build_test_elf(&[(0x1000, 0x100, STT_FUNC, "foo")]);
        elf[4] = 2; // ELFCLASS64
        let result: Result<Vec<Elf32FuncSymbol>, _> = parse_elf32_func_symbols(&elf);
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_big_endian() {
        let mut elf: Vec<u8> = build_test_elf(&[(0x1000, 0x100, STT_FUNC, "foo")]);
        elf[5] = 2; // ELFDATA2MSB
        let result: Result<Vec<Elf32FuncSymbol>, _> = parse_elf32_func_symbols(&elf);
        assert!(result.is_err());
    }

    #[test]
    fn parse_handles_overflow_shoff() {
        let mut elf: Vec<u8> = build_test_elf(&[(0x1000, 0x100, STT_FUNC, "foo")]);
        elf[Elf32Fhdr::OFFSET_E_SHOFF..Elf32Fhdr::OFFSET_E_SHOFF + 4]
            .copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        // Overflow makes all section headers unreachable — returns Ok with no symbols.
        let symbols: Vec<Elf32FuncSymbol> =
            parse_elf32_func_symbols(&elf).expect("should not error on unreachable sections");
        assert!(symbols.is_empty());
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

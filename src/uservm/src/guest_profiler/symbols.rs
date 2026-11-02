// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! ELF symbol table parser for address-to-function-name resolution.

//==================================================================================================
// Imports
//==================================================================================================

use ::elf::symtab::{
    Elf32FuncSymbol,
    parse_elf32_func_symbols,
};
use ::std::path::Path;

//==================================================================================================
// Symbol Resolver
//==================================================================================================

/// Resolves guest addresses to function names using ELF symbol tables.
pub struct SymbolResolver {
    symbols: Vec<Elf32FuncSymbol>,
}

impl SymbolResolver {
    /// Creates a resolver from one or more ELF files.
    ///
    /// Parses the `.symtab` section of each ELF to extract function symbols.
    /// Files that can't be read or parsed are skipped, and per-file diagnostics are
    /// emitted to stderr describing load results and read failures.
    pub fn from_elf_files(paths: &[&Path]) -> Self {
        let mut symbols: Vec<Elf32FuncSymbol> = Vec::new();

        for path in paths {
            match std::fs::read(path) {
                Ok(data) => match parse_elf32_func_symbols(&data) {
                    Ok(mut file_symbols) => {
                        let count: usize = file_symbols.len();
                        symbols.append(&mut file_symbols);
                        eprintln!(
                            "GUEST_PROFILE_SYMBOLS: loaded {} symbols from {} ({} bytes)",
                            count,
                            path.display(),
                            data.len()
                        );
                    },
                    Err(e) => {
                        eprintln!(
                            "GUEST_PROFILE_SYMBOLS: failed to parse {}: {}",
                            path.display(),
                            e.reason
                        );
                    },
                },
                Err(e) => {
                    eprintln!("GUEST_PROFILE_SYMBOLS: failed to read {}: {}", path.display(), e);
                },
            }
        }

        // Sort by address for binary search.
        symbols.sort_by_key(|s| s.addr);

        Self { symbols }
    }

    /// Resolves an address to a function name.
    ///
    /// Returns `"function_name+0xOFFSET"` if found, or `"0xADDRESS"` if not.
    pub fn resolve(&self, addr: u32) -> String {
        // Binary search for the largest symbol address <= addr.
        match self.symbols.binary_search_by_key(&addr, |s| s.addr) {
            Ok(idx) => self.symbols[idx].name.clone(),
            Err(0) => format!("{:#010x}", addr),
            Err(idx) => {
                let sym: &Elf32FuncSymbol = &self.symbols[idx - 1];
                if sym.size > 0 && addr < sym.addr + sym.size {
                    let offset: u32 = addr - sym.addr;
                    if offset == 0 {
                        sym.name.clone()
                    } else {
                        format!("{}+{:#x}", sym.name, offset)
                    }
                } else if sym.size == 0 {
                    // Unknown size — assume it's the right symbol if close.
                    let offset: u32 = addr - sym.addr;
                    if offset < 0x10000 {
                        format!("{}+{:#x}", sym.name, offset)
                    } else {
                        format!("{:#010x}", addr)
                    }
                } else {
                    format!("{:#010x}", addr)
                }
            },
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;

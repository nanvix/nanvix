// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Helper to build a resolver from a list of (addr, size, name) tuples.
fn make_resolver(syms: &[(u32, u32, &str)]) -> SymbolResolver {
    let mut symbols: Vec<Elf32FuncSymbol> = syms
        .iter()
        .map(|&(addr, size, name)| Elf32FuncSymbol {
            addr,
            size,
            name: name.to_string(),
        })
        .collect();
    symbols.sort_by_key(|s| s.addr);
    SymbolResolver { symbols }
}

#[test]
fn resolve_exact_match() {
    let resolver: SymbolResolver = make_resolver(&[(0x1000, 0x100, "foo")]);
    assert_eq!(resolver.resolve(0x1000), "foo");
}

#[test]
fn resolve_offset_within_function() {
    let resolver: SymbolResolver = make_resolver(&[(0x1000, 0x100, "foo")]);
    assert_eq!(resolver.resolve(0x1050), "foo+0x50");
}

#[test]
fn resolve_unknown_address() {
    let resolver: SymbolResolver = make_resolver(&[(0x1000, 0x100, "foo")]);
    assert_eq!(resolver.resolve(0x9999_0000), "0x99990000");
}

#[test]
fn resolve_address_before_all_symbols() {
    let resolver: SymbolResolver = make_resolver(&[(0x1000, 0x100, "foo")]);
    assert_eq!(resolver.resolve(0x0500), "0x00000500");
}

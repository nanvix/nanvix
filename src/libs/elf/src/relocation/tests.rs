// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::elf32::STT_NOTYPE;
#[cfg(not(target_arch = "x86_64"))]
use ::goblin::elf32::sym::Sym;
#[cfg(target_arch = "x86_64")]
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

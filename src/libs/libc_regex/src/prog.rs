// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ast::Ast;
use alloc::vec::Vec;

//==================================================================================================
// Structures
//==================================================================================================

/// A single instruction of the compiled NFA program.
pub(crate) enum Inst {
    /// Match a single literal byte.
    Char(u8),
    /// Match any byte (except a newline under `REG_NEWLINE`).
    Any,
    /// Match any byte unconditionally, including a newline under `REG_NEWLINE`.
    ///
    /// Used only by the unanchored search prelude, whose byte-skipping must not be constrained by
    /// newline sensitivity (which POSIX applies only to `.`, anchors, and complemented classes).
    AnyByte,
    /// Match a byte against a 256-bit class bitmap.
    Set([u8; 32]),
    /// Record the current position into the given submatch slot.
    Save(usize),
    /// Fork execution: try the first target, then the second.
    ///
    /// When the boolean is set, taking the second target adds a lazy-repetition penalty.
    Split(usize, usize, bool),
    /// Jump to the given target.
    Jmp(usize),
    /// Assert the beginning of the line/string.
    Bol,
    /// Assert the end of the line/string.
    Eol,
    /// Accept the match.
    Match,
}

/// A compiled regular expression program.
pub(crate) struct Prog {
    /// Instruction stream.
    pub(crate) insts: Vec<Inst>,
    /// Number of capturing groups.
    pub(crate) nsub: i32,
    /// Whether matching is case-insensitive.
    pub(crate) icase: bool,
    /// Whether matching is newline-sensitive.
    pub(crate) newline: bool,
}

//==================================================================================================
// Compiler
//==================================================================================================

/// Appends an instruction and returns its index.
fn emit(insts: &mut Vec<Inst>, inst: Inst) -> usize {
    insts.push(inst);
    insts.len() - 1
}

/// Sets the first target of a `Split` instruction.
fn patch_split_x(insts: &mut [Inst], idx: usize, x: usize) {
    if let Some(Inst::Split(xx, _, _)) = insts.get_mut(idx) {
        *xx = x;
    }
}

/// Sets the second target of a `Split` instruction.
fn patch_split_y(insts: &mut [Inst], idx: usize, y: usize) {
    if let Some(Inst::Split(_, yy, _)) = insts.get_mut(idx) {
        *yy = y;
    }
}

/// Sets the target of a `Jmp` instruction.
fn patch_jmp(insts: &mut [Inst], idx: usize, x: usize) {
    if let Some(Inst::Jmp(xx)) = insts.get_mut(idx) {
        *xx = x;
    }
}

/// Sets the membership bit for byte `c` in a 32-byte bitmap.
fn bit_set(set: &mut [u8; 32], c: u8) {
    set[usize::from(c >> 3)] |= 1u8 << (c & 7);
}

/// Folds a class bitmap so that, for every member, both ASCII cases are present.
fn fold_set_icase(set: &mut [u8; 32]) {
    let orig: [u8; 32] = *set;
    for b in 0u8..=255 {
        if orig[usize::from(b >> 3)] & (1u8 << (b & 7)) != 0 {
            bit_set(set, b.to_ascii_lowercase());
            bit_set(set, b.to_ascii_uppercase());
        }
    }
}

/// Emits the instructions for a single AST node.
fn compile_node(insts: &mut Vec<Inst>, node: &Ast) {
    match node {
        Ast::Empty => {},
        Ast::Char(c) => {
            emit(insts, Inst::Char(*c));
        },
        Ast::Any => {
            emit(insts, Inst::Any);
        },
        Ast::Set(s) => {
            emit(insts, Inst::Set(*s));
        },
        Ast::Bol => {
            emit(insts, Inst::Bol);
        },
        Ast::Eol => {
            emit(insts, Inst::Eol);
        },
        Ast::Cat(l, r) => {
            compile_node(insts, l);
            compile_node(insts, r);
        },
        Ast::Alt(l, r) => {
            // split A, B ; A: l ; jmp End ; B: r ; End:
            let split: usize = emit(insts, Inst::Split(0, 0, false));
            let here: usize = insts.len();
            patch_split_x(insts, split, here);
            compile_node(insts, l);
            let jmp: usize = emit(insts, Inst::Jmp(0));
            let here: usize = insts.len();
            patch_split_y(insts, split, here);
            compile_node(insts, r);
            let here: usize = insts.len();
            patch_jmp(insts, jmp, here);
        },
        Ast::Rep(sub, min, max, minimal) => compile_rep(insts, sub, *min, *max, *minimal),
        Ast::Group(g, sub) => {
            let base: usize = 2 * usize::try_from(*g).unwrap_or(0);
            emit(insts, Inst::Save(base));
            compile_node(insts, sub);
            emit(insts, Inst::Save(base + 1));
        },
    }
}

/// Emits the instructions for a repetition `[min, max]` of `sub`.
fn compile_rep(insts: &mut Vec<Inst>, sub: &Ast, min: i32, max: i32, minimal: bool) {
    // Emit `min` mandatory copies.
    let mn: i32 = min.max(0);
    for _ in 0..mn {
        compile_node(insts, sub);
    }
    if max == -1 {
        // zero-or-more tail: L: split A, B ; A: sub ; jmp L ; B:
        let l: usize = insts.len();
        let split: usize = emit(insts, Inst::Split(0, 0, minimal));
        let here: usize = insts.len();
        if minimal {
            patch_split_y(insts, split, here);
        } else {
            patch_split_x(insts, split, here);
        }
        compile_node(insts, sub);
        emit(insts, Inst::Jmp(l));
        let here: usize = insts.len();
        if minimal {
            patch_split_x(insts, split, here);
        } else {
            patch_split_y(insts, split, here);
        }
    } else {
        // up to (max - min) optional copies: split Ai, End ; sub
        let opt: i32 = (max - mn).max(0);
        let mut splits: Vec<usize> = Vec::new();
        for _ in 0..opt {
            let split: usize = emit(insts, Inst::Split(0, 0, minimal));
            splits.push(split);
            let here: usize = insts.len();
            if minimal {
                patch_split_y(insts, split, here);
            } else {
                patch_split_x(insts, split, here);
            }
            compile_node(insts, sub);
        }
        let here: usize = insts.len();
        for s in &splits {
            if minimal {
                patch_split_x(insts, *s, here);
            } else {
                patch_split_y(insts, *s, here);
            }
        }
    }
}

/// Compiles an AST into a runnable [`Prog`].
///
/// The program is prefixed with a non-greedy ".*?" skip loop so the pattern can start at any
/// position (leftmost match), and the whole match is captured into slots `0`/`1`.
pub(crate) fn build_prog(tree: &Ast, nsub: i32, icase: bool, newline: bool) -> Prog {
    let mut insts: Vec<Inst> = Vec::new();

    // Unanchored leftmost search prelude:
    //   0: SPLIT 3, 1   (try the pattern first; else skip a byte)
    //   1: ANYBYTE
    //   2: JMP 0
    //   3: SAVE 0       (pattern start)
    //
    // The skip step uses `AnyByte` (not `Any`) so the search can advance past newlines even under
    // `REG_NEWLINE`; newline sensitivity must constrain only `.`, anchors, and complemented
    // classes, not the leftmost-search machinery.
    emit(&mut insts, Inst::Split(3, 1, false));
    emit(&mut insts, Inst::AnyByte);
    emit(&mut insts, Inst::Jmp(0));
    emit(&mut insts, Inst::Save(0));

    compile_node(&mut insts, tree);

    emit(&mut insts, Inst::Save(1));
    emit(&mut insts, Inst::Match);

    // Fold literals and classes to be case-insensitive, if requested.
    if icase {
        for inst in insts.iter_mut() {
            match inst {
                Inst::Char(c) => *c = c.to_ascii_lowercase(),
                Inst::Set(s) => fold_set_icase(s),
                _ => {},
            }
        }
    }

    Prog {
        insts,
        nsub,
        icase,
        newline,
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ast::Ast,
    types::{
        REG_BADBR,
        REG_EBRACE,
        REG_EBRACK,
        REG_ECTYPE,
        REG_EESCAPE,
        REG_EPAREN,
        REG_ERANGE,
    },
};
use ::sysapi::ffi::c_int;
use alloc::boxed::Box;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Returns `true` if `c` is whitespace under the C locale (matches `isspace`).
fn is_space_c(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Sets the membership bit for byte `c` in a 32-byte (256-bit) bitmap.
fn set_add(set: &mut [u8; 32], c: u8) {
    set[usize::from(c >> 3)] |= 1u8 << (c & 7);
}

/// Clears the membership bit for byte `c` in a 32-byte (256-bit) bitmap.
fn set_remove(set: &mut [u8; 32], c: u8) {
    set[usize::from(c >> 3)] &= !(1u8 << (c & 7));
}

//==================================================================================================
// Parser
//==================================================================================================

/// Recursive-descent parser that turns a pattern into an [`Ast`].
pub(crate) struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    ere: bool,
    /// Whether matching is newline-sensitive (`REG_NEWLINE`).
    newline: bool,
    /// Whether ERE duplication symbols prefer shortest matches by default.
    minimal: bool,
    /// First error encountered (`0` means none).
    pub(crate) err: c_int,
    /// Number of capturing groups seen so far.
    pub(crate) ngroup: i32,
}

impl<'a> Parser<'a> {
    /// Creates a new parser over `bytes`, selecting ERE or BRE syntax.
    pub(crate) fn new(bytes: &'a [u8], ere: bool, newline: bool, minimal: bool) -> Self {
        Self {
            bytes,
            pos: 0,
            ere,
            newline,
            minimal,
            err: 0,
            ngroup: 0,
        }
    }

    /// Returns `true` if the whole pattern has been consumed.
    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Returns the current byte, if any.
    fn cur(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Returns the byte `off` positions ahead of the cursor, if any.
    fn at(&self, off: usize) -> Option<u8> {
        self.bytes.get(self.pos + off).copied()
    }

    /// Parses a top-level alternation: `cat ('|' cat)*`.
    pub(crate) fn parse_alt(&mut self) -> Option<Box<Ast>> {
        let mut left: Box<Ast> = self.parse_cat()?;
        loop {
            let is_alt: bool = if self.ere && self.cur() == Some(b'|') {
                self.pos += 1;
                true
            } else if !self.ere && self.cur() == Some(b'\\') && self.at(1) == Some(b'|') {
                self.pos += 2;
                true
            } else {
                false
            };
            if !is_alt {
                break;
            }
            let right: Box<Ast> = self.parse_cat()?;
            left = Box::new(Ast::Alt(left, right));
        }
        Some(left)
    }

    /// Parses a concatenation of quantified atoms.
    fn parse_cat(&mut self) -> Option<Box<Ast>> {
        let mut head: Option<Box<Ast>> = None;
        while !self.at_cat_end() && self.err == 0 {
            let mut atom: Box<Ast> = self.parse_atom()?;
            loop {
                let quant: Option<(i32, i32, bool)> = self.parse_quant();
                if self.err != 0 {
                    return None;
                }
                match quant {
                    Some((mn, mx, minimal)) => atom = Box::new(Ast::Rep(atom, mn, mx, minimal)),
                    None => break,
                }
            }
            head = Some(match head {
                None => atom,
                Some(h) => Box::new(Ast::Cat(h, atom)),
            });
        }
        Some(head.unwrap_or_else(|| Box::new(Ast::Empty)))
    }

    /// Returns `true` when the parser is at the end of a concatenation.
    fn at_cat_end(&self) -> bool {
        match self.cur() {
            None => true,
            Some(c) => {
                if self.ere {
                    c == b'|' || c == b')'
                } else if c == b'\\' {
                    matches!(self.at(1), Some(b'|') | Some(b')'))
                } else {
                    false
                }
            },
        }
    }

    /// Parses a single atom (no trailing quantifier).
    fn parse_atom(&mut self) -> Option<Box<Ast>> {
        let c: u8 = match self.cur() {
            Some(c) => c,
            None => return Some(Box::new(Ast::Empty)),
        };

        // Grouping.
        if self.ere && c == b'(' {
            self.pos += 1;
            self.ngroup += 1;
            let g: i32 = self.ngroup;
            let inner: Box<Ast> = self.parse_alt()?;
            if self.cur() != Some(b')') {
                self.err = REG_EPAREN;
                return None;
            }
            self.pos += 1;
            return Some(Box::new(Ast::Group(g, inner)));
        }
        if !self.ere && c == b'\\' && self.at(1) == Some(b'(') {
            self.pos += 2;
            self.ngroup += 1;
            let g: i32 = self.ngroup;
            let inner: Box<Ast> = self.parse_alt()?;
            if self.cur() != Some(b'\\') || self.at(1) != Some(b')') {
                self.err = REG_EPAREN;
                return None;
            }
            self.pos += 2;
            return Some(Box::new(Ast::Group(g, inner)));
        }

        if c == b'.' {
            self.pos += 1;
            return Some(Box::new(Ast::Any));
        }
        if c == b'[' {
            self.pos += 1;
            return self.parse_set();
        }
        if c == b'^' {
            self.pos += 1;
            return Some(Box::new(Ast::Bol));
        }
        if c == b'$' {
            self.pos += 1;
            return Some(Box::new(Ast::Eol));
        }

        // Escapes.
        if c == b'\\' {
            if let Some(e) = self.at(1) {
                self.pos += 2;
                return Some(match e {
                    b'n' => Box::new(Ast::Char(b'\n')),
                    b't' => Box::new(Ast::Char(b'\t')),
                    b'r' => Box::new(Ast::Char(b'\r')),
                    b'w' | b'W' | b's' | b'S' | b'd' | b'D' => {
                        Box::new(Ast::Set(shorthand_class(e, self.newline)))
                    },
                    // Escaped literal (covers \\, \., \*, digits, etc.). Backreferences are not
                    // supported by the NFA, so a \1 is treated as the literal digit.
                    _ => Box::new(Ast::Char(e)),
                });
            }
            self.err = REG_EESCAPE;
            return None;
        }

        // Ordinary literal byte.
        self.pos += 1;
        Some(Box::new(Ast::Char(c)))
    }

    /// Parses a bracket expression `[...]`, with the leading `[` already consumed.
    fn parse_set(&mut self) -> Option<Box<Ast>> {
        let mut set: [u8; 32] = [0u8; 32];
        let mut negate: bool = false;
        if self.cur() == Some(b'^') {
            negate = true;
            self.pos += 1;
        }
        // A ']' immediately after '[' or '[^' is a literal ']'.
        let mut first: bool = true;
        while let Some(c) = self.cur() {
            if c == b']' && !first {
                self.pos += 1;
                if negate {
                    for b in set.iter_mut() {
                        *b = !*b;
                    }
                    // POSIX: under `REG_NEWLINE`, a non-matching list never matches a newline.
                    if self.newline {
                        set_remove(&mut set, b'\n');
                    }
                }
                return Some(Box::new(Ast::Set(set)));
            }
            first = false;

            if c == b'[' && self.at(1) == Some(b':') {
                self.pos += 2;
                if !self.set_posix_class(&mut set) {
                    return None;
                }
                continue;
            }

            self.pos += 1;
            // Range: a-z (but '-' at the end or before ']' is a literal).
            if self.cur() == Some(b'-') {
                if let Some(hi) = self.at(1) {
                    if hi != b']' {
                        self.pos += 2;
                        if hi < c {
                            self.err = REG_ERANGE;
                            return None;
                        }
                        let mut ci: u8 = c;
                        loop {
                            set_add(&mut set, ci);
                            if ci == hi {
                                break;
                            }
                            ci += 1;
                        }
                        continue;
                    }
                }
            }
            set_add(&mut set, c);
        }
        self.err = REG_EBRACK;
        None
    }

    /// Adds a POSIX named class (e.g. `[:alpha:]`) to `set`, with `[:` already consumed.
    fn set_posix_class(&mut self, set: &mut [u8; 32]) -> bool {
        // Find the terminating ":]".
        let start: usize = self.pos;
        let mut q: usize = self.pos;
        while let Some(b) = self.bytes.get(q) {
            if *b == b':' {
                break;
            }
            q += 1;
        }
        if self.bytes.get(q) != Some(&b':') || self.bytes.get(q + 1) != Some(&b']') {
            self.err = REG_ECTYPE;
            return false;
        }
        let name: &[u8] = match self.bytes.get(start..q) {
            Some(n) => n,
            None => {
                self.err = REG_ECTYPE;
                return false;
            },
        };
        self.pos = q + 2;

        let pred: fn(u8) -> bool = match name {
            b"alpha" => |c| c.is_ascii_alphabetic(),
            b"digit" => |c| c.is_ascii_digit(),
            b"alnum" => |c| c.is_ascii_alphanumeric(),
            b"space" => is_space_c,
            b"upper" => |c| c.is_ascii_uppercase(),
            b"lower" => |c| c.is_ascii_lowercase(),
            b"blank" => |c| c == b' ' || c == b'\t',
            b"punct" => |c| c.is_ascii_punctuation(),
            b"cntrl" => |c| c.is_ascii_control(),
            b"graph" => |c| c.is_ascii_graphic(),
            b"print" => |c| c.is_ascii_graphic() || c == b' ',
            b"xdigit" => |c| c.is_ascii_hexdigit(),
            _ => {
                self.err = REG_ECTYPE;
                return false;
            },
        };
        for b in 0u8..=255 {
            if pred(b) {
                set_add(set, b);
            }
        }
        true
    }

    /// Parses a `{n,m}` / `\{n,m\}` interval, returning `(min, max)` (`max == -1` is unbounded).
    fn parse_interval(&mut self) -> Option<(i32, i32)> {
        let mut min: i32 = 0;
        let mut max: i32 = 0;
        let mut have_min: bool = false;
        let mut have_max: bool = false;
        let mut comma: bool = false;
        while let Some(c) = self.cur() {
            if c.is_ascii_digit() {
                min = match accumulate_digit(min, c) {
                    Some(v) => v,
                    None => {
                        self.err = REG_BADBR;
                        return None;
                    },
                };
                have_min = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.cur() == Some(b',') {
            comma = true;
            self.pos += 1;
            while let Some(c) = self.cur() {
                if c.is_ascii_digit() {
                    max = match accumulate_digit(max, c) {
                        Some(v) => v,
                        None => {
                            self.err = REG_BADBR;
                            return None;
                        },
                    };
                    have_max = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        // Consume the closing brace: '}' (ERE) or '\}' (BRE).
        if self.ere {
            if self.cur() != Some(b'}') {
                self.err = REG_EBRACE;
                return None;
            }
            self.pos += 1;
        } else {
            if self.cur() != Some(b'\\') || self.at(1) != Some(b'}') {
                self.err = REG_EBRACE;
                return None;
            }
            self.pos += 2;
        }
        if !have_min {
            self.err = REG_BADBR;
            return None;
        }
        let pmax: i32 = if !comma {
            min
        } else if !have_max {
            -1
        } else {
            max
        };
        if pmax != -1 && pmax < min {
            self.err = REG_BADBR;
            return None;
        }
        Some((min, pmax))
    }

    /// Consumes a quantifier if present, returning `(min, max)`; `None` means no quantifier (or an
    /// error, in which case `self.err` is set).
    fn parse_quant(&mut self) -> Option<(i32, i32, bool)> {
        let c: u8 = self.cur()?;
        if c == b'*' {
            self.pos += 1;
            return Some((0, -1, self.parse_minimal_suffix()));
        }
        if self.ere {
            if c == b'+' {
                self.pos += 1;
                return Some((1, -1, self.parse_minimal_suffix()));
            }
            if c == b'?' {
                self.pos += 1;
                return Some((0, 1, self.parse_minimal_suffix()));
            }
            if c == b'{' {
                self.pos += 1;
                return self.parse_interval().map(|(min, max)| {
                    let minimal: bool = self.parse_minimal_suffix();
                    (min, max, minimal)
                });
            }
        } else if c == b'\\' {
            if let Some(e) = self.at(1) {
                if e == b'+' {
                    self.pos += 2;
                    return Some((1, -1, false));
                }
                if e == b'?' {
                    self.pos += 2;
                    return Some((0, 1, false));
                }
                if e == b'{' {
                    self.pos += 2;
                    return self.parse_interval().map(|(min, max)| (min, max, false));
                }
            }
        }
        None
    }

    /// Parses an ERE repetition modifier (`?`) and returns whether the repetition is minimal.
    fn parse_minimal_suffix(&mut self) -> bool {
        let mut minimal: bool = self.ere && self.minimal;
        if self.ere && self.cur() == Some(b'?') {
            self.pos += 1;
            minimal = !minimal;
        }
        minimal
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Accumulates decimal digit `c` into `acc`, returning `None` on `i32` overflow.
fn accumulate_digit(acc: c_int, c: u8) -> Option<c_int> {
    acc.checked_mul(10)?.checked_add(c_int::from(c - b'0'))
}

/// Builds the membership bitmap for a GNU shorthand class (`\w \W \s \S \d \D`).
fn shorthand_class(e: u8, newline: bool) -> [u8; 32] {
    let mut set: [u8; 32] = [0u8; 32];
    let lower: u8 = e.to_ascii_lowercase();
    for b in 0u8..=255 {
        let add: bool = match lower {
            b'w' => b.is_ascii_alphanumeric() || b == b'_',
            b's' => is_space_c(b),
            b'd' => b.is_ascii_digit(),
            _ => false,
        };
        if add {
            set_add(&mut set, b);
        }
    }
    if e.is_ascii_uppercase() {
        for x in set.iter_mut() {
            *x = !*x;
        }
        // POSIX: under `REG_NEWLINE`, a complemented class never matches a newline.
        if newline {
            set_remove(&mut set, b'\n');
        }
    }
    set
}

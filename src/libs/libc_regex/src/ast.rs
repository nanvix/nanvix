// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::boxed::Box;

//==================================================================================================
// Structures
//==================================================================================================

/// Abstract syntax tree node for a parsed regular expression.
pub(crate) enum Ast {
    /// Matches the empty string.
    Empty,
    /// Matches a single literal byte.
    Char(u8),
    /// Matches any byte (except a newline under `REG_NEWLINE`).
    Any,
    /// Matches a byte against a 256-bit membership bitmap.
    Set([u8; 32]),
    /// Asserts the beginning of the line/string.
    Bol,
    /// Asserts the end of the line/string.
    Eol,
    /// Concatenation of two subexpressions.
    Cat(Box<Ast>, Box<Ast>),
    /// Alternation of two subexpressions.
    Alt(Box<Ast>, Box<Ast>),
    /// Repetition of a subexpression `[min, max]` (`max == -1` means unbounded).
    Rep(Box<Ast>, i32, i32, bool),
    /// Capturing group with a 1-based index.
    Group(i32, Box<Ast>),
}

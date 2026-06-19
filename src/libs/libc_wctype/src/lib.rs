// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

mod cstring;

pub mod iswalnum;
pub mod iswalpha;
pub mod iswblank;
pub mod iswcntrl;
pub mod iswctype;
pub mod iswdigit;
pub mod iswgraph;
pub mod iswlower;
pub mod iswprint;
pub mod iswpunct;
pub mod iswspace;
pub mod iswupper;
pub mod iswxdigit;
pub mod locale;
pub mod towctrans;
pub mod towlower;
pub mod towupper;
pub mod wctrans;
pub mod wctrans_t;
pub mod wctype;
pub mod wctype_t;
pub mod wint_t;

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Types
//==================================================================================================

/// Wide character type (matches Linux/Nanvix convention).
#[allow(non_camel_case_types)]
pub type wchar_t = i32;

/// Wide integer type for wide character I/O.
#[allow(non_camel_case_types)]
pub type wint_t = i32;

/// End-of-file indicator for wide character I/O.
pub const WEOF: wint_t = -1;

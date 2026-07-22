// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Types
//==================================================================================================

/// Wide integer type for wide character classification.
#[allow(non_camel_case_types)]
pub type wint_t = i32;

/// End-of-file indicator for wide character operations.
pub const WEOF: wint_t = -1;

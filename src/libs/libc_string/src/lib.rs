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
// Nanvix uses a fixed-width C ABI where c_size_t and usize are both 32-bit.
#![allow(suspicious_runtime_symbol_definitions)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

pub mod bcmp;
pub mod bcopy;
pub mod bzero;
pub mod ffs;
pub mod locale;
pub mod memccpy;
pub mod memchr;
pub mod memcmp;
pub mod memcpy;
pub mod memmove;
pub mod mempcpy;
pub mod memrchr;
pub mod memset;
pub mod stpcpy;
pub mod stpncpy;
pub mod strcasecmp;
pub mod strcasestr;
pub mod strcat;
pub mod strchr;
pub mod strchrnul;
pub mod strcmp;
pub mod strcoll;
pub mod strcpy;
pub mod strcspn;
pub mod strdup;
pub mod strerror;
pub mod strerror_r;
pub mod strlcat;
pub mod strlcpy;
pub mod strlen;
pub mod strncasecmp;
pub mod strncat;
pub mod strncmp;
pub mod strncpy;
pub mod strndup;
pub mod strnlen;
pub mod strpbrk;
pub mod strrchr;
pub mod strsep;
pub mod strsignal;
pub mod strspn;
pub mod strstr;
pub mod strtok;
pub mod strtok_r;
pub mod strverscmp;
pub mod strxfrm;

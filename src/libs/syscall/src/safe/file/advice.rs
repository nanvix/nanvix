// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::file_advice::{
        POSIX_FADV_DONTNEED,
        POSIX_FADV_NOREUSE,
        POSIX_FADV_NORMAL,
        POSIX_FADV_RANDOM,
        POSIX_FADV_SEQUENTIAL,
        POSIX_FADV_WILLNEED,
    },
    ffi::c_int,
};

//===================================================================================================
// RegularFileAdvice
//===================================================================================================

///
/// # Description
///
/// A Structure that represents advisory information about a regular file.
///
#[derive(Debug)]
pub struct RegularFileAdvice {
    advice: c_int,
}

impl RegularFileAdvice {
    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that hints sequential access to a file.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that hints sequential access.
    ///
    pub fn sequential_access() -> Self {
        Self {
            advice: POSIX_FADV_SEQUENTIAL,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that hints random access to a file.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that hints random access.
    ///
    pub fn random_access() -> Self {
        Self {
            advice: POSIX_FADV_RANDOM,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that indicates no specific access pattern.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that indicates no specific access pattern.
    ///
    pub fn normal_access() -> Self {
        Self {
            advice: POSIX_FADV_NORMAL,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that hints that the file will not be accessed in the near future.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that indicates the file will not be accessed soon.
    ///
    pub fn will_not_access() -> Self {
        Self {
            advice: POSIX_FADV_DONTNEED,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that indicates the file will be accessed in the near future.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that indicates the file will be accessed soon.
    ///
    pub fn will_access() -> Self {
        Self {
            advice: POSIX_FADV_WILLNEED,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileAdvice` that hints that once the file is accessed, data will not be reused.
    ///
    /// # Returns
    ///
    /// A `RegularFileAdvice` structure that indicates data will not be reused.
    ///
    pub fn dont_reuse() -> Self {
        Self {
            advice: POSIX_FADV_NOREUSE,
        }
    }
}

impl From<RegularFileAdvice> for c_int {
    fn from(advice: RegularFileAdvice) -> Self {
        advice.advice
    }
}

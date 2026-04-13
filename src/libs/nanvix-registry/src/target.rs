// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a target architecture for Nanvix releases.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// x86 (32/64-bit) architecture.
    X86,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Target {
    /// String representation of x86 target.
    pub const X86_STR: &'static str = "x86";

    ///
    /// # Description
    ///
    /// Converts the target architecture to its string representation.
    ///
    /// # Returns
    ///
    /// A string representation of the target architecture.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            Target::X86 => Target::X86_STR,
        }
    }
}

impl ::std::fmt::Display for Target {
    ///
    /// # Description
    ///
    /// Converts the target architecture to its string representation.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for Target {
    type Error = anyhow::Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a string slice to a `Target` enum variant.
    ///
    /// # Parameters
    ///
    /// - `value`: The string representation of the target architecture (case-insensitive).
    ///
    /// # Returns
    ///
    /// On success, returns the corresponding `Target` variant. On failure, it returns an object
    /// that describes the error.
    ///
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value_lower: String = value.to_lowercase();
        match value_lower.as_str() {
            Self::X86_STR => Ok(Target::X86),
            _ => {
                let reason: String = format!("Unknown target architecture: {value}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests that `X86` target converts to correct string representation.
    ///
    #[test]
    fn test_x86_as_str() {
        let target: Target = Target::X86;
        assert_eq!(target.as_str(), "x86");
    }

    ///
    /// # Description
    ///
    /// Tests that `X86` display trait works correctly.
    ///
    #[test]
    fn test_x86_display() {
        let target: Target = Target::X86;
        assert_eq!(format!("{}", target), "x86");
    }

    ///
    /// # Description
    ///
    /// Tests conversion from valid string.
    ///
    #[test]
    fn test_try_from_valid() {
        let target: Target = Target::try_from("x86").expect("should succeed");
        assert!(matches!(target, Target::X86));
    }

    ///
    /// # Description
    ///
    /// Tests case-insensitive conversion from string.
    ///
    #[test]
    fn test_try_from_case_insensitive() {
        let target: Target = Target::try_from("X86").expect("should succeed");
        assert!(matches!(target, Target::X86));
    }

    ///
    /// # Description
    ///
    /// Tests conversion from invalid string.
    ///
    #[test]
    fn test_try_from_invalid() {
        let result: Result<Target> = Target::try_from("arm64");
        assert!(result.is_err());
    }
}

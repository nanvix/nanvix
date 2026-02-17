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
/// Represents a target machine type for Nanvix releases.
///
#[derive(Debug, Clone, Copy)]
pub enum Machine {
    /// Hyperlight machine type.
    Hyperlight,
    /// MicroVM machine type.
    Microvm,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Machine {
    /// String representation of Hyperlight machine.
    pub const HYPERLIGHT_STR: &'static str = "hyperlight";
    /// String representation of Microvm machine.
    pub const MICROVM_STR: &'static str = "microvm";

    ///
    /// # Description
    ///
    /// Converts the machine type to its string representation.
    ///
    /// # Returns
    ///
    /// A string representation of the machine type.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            Machine::Hyperlight => Machine::HYPERLIGHT_STR,
            Machine::Microvm => Machine::MICROVM_STR,
        }
    }
}

impl ::std::fmt::Display for Machine {
    ///
    /// # Description
    ///
    /// Converts the machine type to its string representation.
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

impl TryFrom<&str> for Machine {
    type Error = anyhow::Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a string slice to a `Machine` enum variant.
    ///
    /// # Parameters
    ///
    /// - `value`: The string representation of the machine type (case-insensitive).
    ///
    /// # Returns
    ///
    /// On success, returns the corresponding `Machine` variant. On failure, it returns an object
    /// that describes the error.
    ///
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value_lower: String = value.to_lowercase();
        match value_lower.as_str() {
            Self::HYPERLIGHT_STR => Ok(Machine::Hyperlight),
            Self::MICROVM_STR => Ok(Machine::Microvm),
            _ => {
                let reason: String = format!("Unknown machine type: {value}");
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
    /// Tests that `Hyperlight` machine converts to correct string representation.
    ///
    #[test]
    fn test_hyperlight_as_str() {
        let machine: Machine = Machine::Hyperlight;
        assert_eq!(machine.as_str(), "hyperlight");
    }

    ///
    /// # Description
    ///
    /// Tests that `Microvm` machine converts to correct string representation.
    ///
    #[test]
    fn test_microvm_as_str() {
        let machine: Machine = Machine::Microvm;
        assert_eq!(machine.as_str(), "microvm");
    }

    ///
    /// # Description
    ///
    /// Tests that `Hyperlight` display trait works correctly.
    ///
    #[test]
    fn test_hyperlight_display() {
        let machine: Machine = Machine::Hyperlight;
        assert_eq!(format!("{}", machine), "hyperlight");
    }

    ///
    /// # Description
    ///
    /// Tests that `Microvm` display trait works correctly.
    ///
    #[test]
    fn test_microvm_display() {
        let machine: Machine = Machine::Microvm;
        assert_eq!(format!("{}", machine), "microvm");
    }

    ///
    /// # Description
    ///
    /// Tests that valid hyperlight string is converted successfully.
    ///
    #[test]
    fn test_try_from_valid_hyperlight() {
        let result: Result<Machine> = Machine::try_from("hyperlight");
        assert!(result.is_ok());
        assert!(matches!(result.expect("failed"), Machine::Hyperlight));
    }

    ///
    /// # Description
    ///
    /// Tests that valid microvm string is converted successfully.
    ///
    #[test]
    fn test_try_from_valid_microvm() {
        let result: Result<Machine> = Machine::try_from("microvm");
        assert!(result.is_ok());
        assert!(matches!(result.expect("failed"), Machine::Microvm));
    }

    ///
    /// # Description
    ///
    /// Tests that conversion is case-insensitive.
    ///
    #[test]
    fn test_try_from_case_insensitive() {
        let test_cases: [&str; 4] = ["HYPERLIGHT", "Hyperlight", "MICROVM", "MicroVM"];

        for case in &test_cases {
            let result: Result<Machine> = Machine::try_from(*case);
            assert!(result.is_ok());
        }
    }

    ///
    /// # Description
    ///
    /// Tests that invalid machine string returns an error.
    ///
    #[test]
    fn test_try_from_invalid() {
        let result: Result<Machine> = Machine::try_from("invalid-machine");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("Unknown machine type"));
    }

    ///
    /// # Description
    ///
    /// Tests that empty string returns an error.
    ///
    #[test]
    fn test_try_from_empty() {
        let result: Result<Machine> = Machine::try_from("");
        assert!(result.is_err());
    }
}

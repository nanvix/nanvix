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
/// Represents a deployment type for Nanvix releases.
///
#[derive(Debug, Clone, Copy)]
pub enum Deployment {
    /// Single-process deployment mode.
    SingleProcess,
    /// Multi-process deployment mode.
    MultiProcess,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Deployment {
    /// String representation of SingleProcess deployment.
    pub const SINGLE_PROCESS_STR: &'static str = "single-process";
    /// String representation of MultiProcess deployment.
    pub const MULTI_PROCESS_STR: &'static str = "multi-process";

    ///
    /// # Description
    ///
    /// Converts the deployment type to its string representation.
    ///
    /// # Returns
    ///
    /// A string representation of the deployment type.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            Deployment::SingleProcess => Deployment::SINGLE_PROCESS_STR,
            Deployment::MultiProcess => Deployment::MULTI_PROCESS_STR,
        }
    }
}

impl ::std::fmt::Display for Deployment {
    ///
    /// # Description
    ///
    /// Converts the deployment type to its string representation.
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

impl TryFrom<&str> for Deployment {
    type Error = anyhow::Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a string slice to a `Deployment` enum variant.
    ///
    /// # Parameters
    ///
    /// - `value`: The string representation of the deployment type (case-insensitive).
    ///
    /// # Returns
    ///
    /// On success, returns the corresponding `Deployment` variant. On failure, it returns an object
    /// that describes the error.
    ///
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value_lower: String = value.to_lowercase();
        match value_lower.as_str() {
            Self::SINGLE_PROCESS_STR => Ok(Deployment::SingleProcess),
            Self::MULTI_PROCESS_STR => Ok(Deployment::MultiProcess),
            _ => {
                let reason: String = format!("Unknown deployment type: {value}");
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
    /// Tests that `SingleProcess` deployment converts to correct string representation.
    ///
    #[test]
    fn test_single_process_as_str() {
        let deployment: Deployment = Deployment::SingleProcess;
        assert_eq!(deployment.as_str(), "single-process");
    }

    ///
    /// # Description
    ///
    /// Tests that `MultiProcess` deployment converts to correct string representation.
    ///
    #[test]
    fn test_multi_process_as_str() {
        let deployment: Deployment = Deployment::MultiProcess;
        assert_eq!(deployment.as_str(), "multi-process");
    }

    ///
    /// # Description
    ///
    /// Tests that `SingleProcess` display trait works correctly.
    ///
    #[test]
    fn test_single_process_display() {
        let deployment: Deployment = Deployment::SingleProcess;
        assert_eq!(format!("{}", deployment), "single-process");
    }

    ///
    /// # Description
    ///
    /// Tests that `MultiProcess` display trait works correctly.
    ///
    #[test]
    fn test_multi_process_display() {
        let deployment: Deployment = Deployment::MultiProcess;
        assert_eq!(format!("{}", deployment), "multi-process");
    }

    ///
    /// # Description
    ///
    /// Tests that valid single-process string is converted successfully.
    ///
    #[test]
    fn test_try_from_valid_single_process() {
        let result: Result<Deployment> = Deployment::try_from("single-process");
        assert!(result.is_ok());
        assert!(matches!(result.expect("failed"), Deployment::SingleProcess));
    }

    ///
    /// # Description
    ///
    /// Tests that valid multi-process string is converted successfully.
    ///
    #[test]
    fn test_try_from_valid_multi_process() {
        let result: Result<Deployment> = Deployment::try_from("multi-process");
        assert!(result.is_ok());
        assert!(matches!(result.expect("failed"), Deployment::MultiProcess));
    }

    ///
    /// # Description
    ///
    /// Tests that conversion is case-insensitive.
    ///
    #[test]
    fn test_try_from_case_insensitive() {
        let test_cases: [&str; 4] = [
            "SINGLE-PROCESS",
            "Single-Process",
            "MULTI-PROCESS",
            "Multi-Process",
        ];

        for case in &test_cases {
            let result: Result<Deployment> = Deployment::try_from(*case);
            assert!(result.is_ok());
        }
    }

    ///
    /// # Description
    ///
    /// Tests that invalid deployment string returns an error.
    ///
    #[test]
    fn test_try_from_invalid() {
        let result: Result<Deployment> = Deployment::try_from("invalid-deployment");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("Unknown deployment type"));
    }

    ///
    /// # Description
    ///
    /// Tests that empty string returns an error.
    ///
    #[test]
    fn test_try_from_empty() {
        let result: Result<Deployment> = Deployment::try_from("");
        assert!(result.is_err());
    }
}

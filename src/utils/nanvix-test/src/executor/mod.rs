// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod empty;
pub mod http;
pub mod terminal;

use ::anyhow::Result;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Executor variants supported by the Nanvix test runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorName {
    /// Empty executor.
    Empty,
    /// HTTP executor.
    Http,
    /// Terminal executor.
    Terminal,
}

impl ExecutorName {
    ///
    /// # Description
    ///
    /// Parses a textual identifier into the corresponding executor.
    ///
    /// # Parameters
    ///
    /// - `identifier`: Executor label read from the configuration file.
    ///
    /// # Return Value
    ///
    /// Returns the matching executor variant when the identifier is supported; returns an error
    /// when the identifier is invalid.
    pub fn from_str(identifier: &str) -> Result<Self> {
        match identifier {
            "empty" => Ok(Self::Empty),
            "http" => Ok(Self::Http),
            "terminal" => Ok(Self::Terminal),
            _ => Err(::anyhow::anyhow!(format!("invalid executor name '{identifier}'"))),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the canonical directory label associated with the executor variant.
    ///
    /// # Return Value
    ///
    /// Returns one of `empty`, `http`, or `terminal` for use when organizing logs.
    ///
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Http => "http",
            Self::Terminal => "terminal",
        }
    }
}

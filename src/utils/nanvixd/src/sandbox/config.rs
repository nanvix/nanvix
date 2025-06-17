// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// Packs configuration for a sandbox.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Socket address for the Linux daemon.
    linuxd_sockaddr: String,
    /// Socket address for the Sandbox.
    sandbox_sockaddr: String,
    /// File for console output.
    console_file: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxConfig {
    ///
    /// # Description
    ///
    /// Creates a new sandbox configuration.
    ///
    /// # Parameters
    ///
    /// - `linuxd_sockaddr`: Socket address for the Linux daemon.
    /// - `sandbox_sockaddr`: Socket address for the Sandbox.
    /// - `console_file`: File for console output.
    ///
    /// # Returns
    ///
    /// A new sandbox configuration.
    ///
    pub fn new(
        linuxd_sockaddr: &str,
        sandbox_sockaddr: &str,
        console_file: &str,
    ) -> Self {
        Self {
            linuxd_sockaddr: linuxd_sockaddr.to_string(),
            sandbox_sockaddr: sandbox_sockaddr.to_string(),
            console_file: console_file.to_string(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the Linux daemon.
    ///
    /// # Returns
    ///
    /// The socket address for the Linux daemon.
    ///
    pub fn linuxd_sockaddr(&self) -> &str {
        &self.linuxd_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the Sandbox.
    ///
    /// # Returns
    ///
    /// The socket address for the Sandbox.
    ///
    pub fn sandbox_sockaddr(&self) -> &str {
        &self.sandbox_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the file for console output.
    ///
    /// # Returns
    ///
    /// The file for console output.
    ///
    pub fn console_file(&self) -> &str {
        &self.console_file
    }
}

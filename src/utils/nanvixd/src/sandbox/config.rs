// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// Packs configuration for a sandbox.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Socket address for the control plane nanvixd <-> linuxd.
    control_plane_sockaddr: String,
    /// Socket address to interact with the user VM stdin/stdout client <-> linuxd.
    gateway_sockaddr: String,
    /// Socket address for the linuxd <-> user VM communication.
    user_vm_sockaddr: String,
    /// Path to the program to run.
    program: String,
    /// Argv for the program to run.
    program_args: Option<String>,
    /// File for console output.
    console_file: Option<String>,
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
    /// - `control_plane_sockaddr`: Socket address for the control plane.
    /// - `linuxd_sockaddr`: Socket address for the Linux daemon.
    /// - `sandbox_sockaddr`: Socket address for the Sandbox.
    /// - `program`: Path to the binary to run in the User VM.
    /// - `program_args`: Argv for the program to run in the user VM.
    /// - `console_file`: File for console output.
    ///
    /// # Returns
    ///
    /// A new sandbox configuration.
    ///
    pub fn new(
        control_plane_sockaddr: &str,
        gateway_sockaddr: &str,
        user_vm_sockaddr: &str,
        program: &str,
        program_args: Option<String>,
        console_file: Option<String>,
    ) -> Self {
        Self {
            control_plane_sockaddr: control_plane_sockaddr.to_string(),
            gateway_sockaddr: gateway_sockaddr.to_string(),
            user_vm_sockaddr: user_vm_sockaddr.to_string(),
            program: program.to_string(),
            program_args,
            console_file,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the control plane.
    ///
    /// # Returns
    ///
    /// The socket address for the control plane.
    ///
    pub fn control_plane_sockaddr(&self) -> &str {
        &self.control_plane_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for linuxd's gateway socket.
    ///
    /// # Returns
    ///
    /// The socket address for linuxd's gateway.
    ///
    pub fn gateway_sockaddr(&self) -> &str {
        &self.gateway_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the file path of the sandbox's program.
    ///
    /// # Returns
    ///
    /// The file path of the main program.
    ///
    pub fn program(&self) -> &str {
        &self.program
    }

    ///
    /// # Description
    ///
    /// Returns the argv of the sandbox's program.
    ///
    /// # Returns
    ///
    /// The argv of the main program.
    ///
    pub fn program_args(&self) -> Option<&str> {
        self.program_args.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the user VM to linuxd communication.
    ///
    /// # Returns
    ///
    /// The socket address for the user VM.
    ///
    pub fn user_vm_sockaddr(&self) -> &str {
        &self.user_vm_sockaddr
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
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }
}

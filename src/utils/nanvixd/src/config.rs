// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants and utilities for Nanvix Daemon.
//!
//! This module provides default values used by the Nanvix Daemon, and loads the configuration
//! files that set its options.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix::sandbox_config::Ipv4Cidr;
use ::serde::Deserialize;
use ::std::path::Path;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Fallback binary directory path for Nanvix binaries, used when the directory that contains the
/// running executable cannot be determined.
///
pub const FALLBACK_BIN_DIRECTORY: &str = "./bin";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Options of the Nanvix Daemon that a configuration file sets.
///
/// A configuration file is a TOML file whose keys set the options of the Nanvix Daemon. Each key
/// is named after the command-line option that it sets, without the leading dash and with the
/// remaining dashes replaced by underscores (e.g., `bin_dir` sets `-bin-dir`). The exceptions are
/// the repeatable `-allow-host` and `-block-host` options, which the `allow_hosts` and
/// `block_hosts` arrays set. Every key is optional, and unknown keys are rejected. Relative paths
/// are resolved against the directory that contains the configuration file.
///
/// On Windows, `gateway_sockaddr` must be a named pipe path (`\\.\pipe\<name>`), which is used as
/// is, because it is not a filesystem path.
///
/// For example:
///
/// ```toml
/// http_addr = "127.0.0.1:8080"
/// bin_dir = "/opt/nanvix/bin"
/// log_dir = "/var/log/nanvix"
/// allow_host_networking = true
/// allow_hosts = ["10.0.0.0/8"]
/// block_hosts = ["10.0.0.1"]
/// ```
///
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigFile {
    /// HTTP server socket address (`-http-addr`).
    pub(crate) http_addr: Option<String>,
    /// Directory that contains Nanvix binaries (`-bin-dir`).
    pub(crate) bin_dir: Option<String>,
    /// File to which the console output is redirected (`-console-file`).
    pub(crate) console_file: Option<String>,
    /// RAM filesystem image attached to User VMs (`-ramfs`).
    pub(crate) ramfs: Option<String>,
    /// Directory for log files (`-log-dir`).
    pub(crate) log_dir: Option<String>,
    /// Route logs to stdout instead of a log file (`-log-to-stdout`).
    #[serde(default)]
    pub(crate) log_to_stdout: bool,
    /// Snapshot from which User VMs are restored instead of cold-booted (`-snapshot`).
    pub(crate) snapshot: Option<String>,
    /// Host directory mounted on the guest (`-mount`).
    pub(crate) mount: Option<String>,
    /// Kernel arguments written to guest control registers (`-kernel-args`).
    pub(crate) kernel_args: Option<String>,
    /// GDB server port (`-gdb-port`).
    #[cfg(feature = "gdb")]
    pub(crate) gdb_port: Option<u16>,
    /// Enable host networking for the guest (`-allow-host-networking`).
    #[serde(default)]
    pub(crate) allow_host_networking: bool,
    /// IPv4 addresses or CIDRs to which egress is permitted (`-allow-host`).
    #[serde(default)]
    pub(crate) allow_hosts: Vec<String>,
    /// IPv4 addresses or CIDRs to which egress is denied (`-block-host`).
    #[serde(default)]
    pub(crate) block_hosts: Vec<String>,
    /// Path of the gateway endpoint (`-gateway-sockaddr`).
    pub(crate) gateway_sockaddr: Option<String>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ConfigFile {
    ///
    /// # Description
    ///
    /// Loads a configuration file.
    ///
    /// Relative paths are resolved against the directory that contains the configuration file.
    /// Options are validated on their own, like their command-line counterparts.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the configuration file.
    ///
    /// # Returns
    ///
    /// On success, returns the options that the configuration file sets. On failure, returns an
    /// error that names the configuration file: it cannot be read, it is not valid TOML, it has an
    /// unknown key or a value of the wrong type, or it sets an invalid option.
    ///
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let contents: String = ::std::fs::read_to_string(path).map_err(|error| {
            anyhow::anyhow!("failed to read configuration file {}: {error}", path.display())
        })?;
        let mut config: Self = ::toml::from_str(&contents).map_err(|error| {
            anyhow::anyhow!("failed to parse configuration file {}: {error}", path.display())
        })?;
        config.resolve_paths(path.parent().unwrap_or(Path::new("")));
        config.validate().map_err(|error| {
            anyhow::anyhow!("invalid configuration file {}: {error}", path.display())
        })?;
        Ok(config)
    }

    ///
    /// # Description
    ///
    /// Resolves relative paths against a base directory.
    ///
    /// # Parameters
    ///
    /// - `base_directory`: Directory against which relative paths are resolved.
    ///
    fn resolve_paths(&mut self, base_directory: &Path) {
        let paths = [
            &mut self.bin_dir,
            &mut self.console_file,
            &mut self.ramfs,
            &mut self.log_dir,
            &mut self.snapshot,
            &mut self.mount,
            // On Windows, the gateway endpoint is a named pipe, whose path is not a filesystem
            // path, so it is used as is.
            #[cfg(unix)]
            &mut self.gateway_sockaddr,
        ];
        for path in paths.into_iter().flatten() {
            // Joining an absolute path yields that path unchanged. Both components are UTF-8, so
            // converting the joined path back to a string is lossless.
            *path = base_directory
                .join(path.as_str())
                .to_string_lossy()
                .into_owned();
        }
    }

    ///
    /// # Description
    ///
    /// Validates the options that the configuration file sets.
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error that describes the first invalid
    /// option.
    ///
    fn validate(&self) -> Result<()> {
        if self.log_to_stdout && self.log_dir.is_some() {
            anyhow::bail!("log_to_stdout and log_dir are mutually exclusive");
        }
        if let Some(snapshot_path) = &self.snapshot {
            validate_snapshot_path(snapshot_path)?;
        }
        if let Some(mount_directory) = &self.mount {
            validate_mount_directory(mount_directory)?;
        }
        #[cfg(windows)]
        if let Some(gateway_sockaddr) = &self.gateway_sockaddr {
            validate_named_pipe_path(gateway_sockaddr)?;
        }
        for (key, hosts) in [
            ("allow_hosts", &self.allow_hosts),
            ("block_hosts", &self.block_hosts),
        ] {
            if let Some(host) = hosts.iter().find(|host| Ipv4Cidr::parse(host).is_none()) {
                anyhow::bail!("invalid {key} entry (expected IPv4 or CIDR): {host}");
            }
        }
        Ok(())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the default directory in which the Nanvix Daemon looks for Nanvix binaries.
///
/// The default is the directory that contains the running executable. This way, a Nanvix Daemon
/// that is shipped alongside the binaries it depends on (e.g., in the `bin` directory of a release
/// archive) uses the co-located binaries, regardless of the current working directory. If that
/// directory cannot be determined, [`FALLBACK_BIN_DIRECTORY`] is returned instead.
///
/// Whether a symbolic link to the executable is followed depends on the platform:
/// [`std::env::current_exe()`] resolves it on Linux, but reports the path of the link itself on
/// Windows.
///
/// # Returns
///
/// The default binary directory path.
///
pub fn default_binary_directory() -> String {
    ::std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(executable_directory)
        .unwrap_or_else(|| FALLBACK_BIN_DIRECTORY.to_string())
}

///
/// # Description
///
/// Returns the directory that contains an executable.
///
/// # Parameters
///
/// - `executable_path`: Path to the executable.
///
/// # Returns
///
/// The directory that contains the executable, or `None` if `executable_path` has no parent
/// directory or if that directory is not valid UTF-8.
///
fn executable_directory(executable_path: &Path) -> Option<String> {
    executable_path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .and_then(Path::to_str)
        .map(str::to_string)
}

///
/// # Description
///
/// Checks that a snapshot path refers to an existing file.
///
/// # Parameters
///
/// - `path`: Snapshot path.
///
/// # Returns
///
/// On success, returns empty. On failure, returns an error: the path does not exist, or it is not
/// a file.
///
pub(crate) fn validate_snapshot_path(path: &str) -> Result<()> {
    let metadata: ::std::fs::Metadata = ::std::fs::metadata(path)
        .map_err(|_| anyhow::anyhow!("snapshot path does not exist: {path}"))?;
    if !metadata.is_file() {
        anyhow::bail!("snapshot path is not a file: {path}");
    }
    Ok(())
}

///
/// # Description
///
/// Checks that a mount path refers to an existing directory.
///
/// # Parameters
///
/// - `path`: Host directory to mount on the guest.
///
/// # Returns
///
/// On success, returns empty. On failure, returns an error: the path does not exist, or it is not
/// a directory.
///
pub(crate) fn validate_mount_directory(path: &str) -> Result<()> {
    let metadata: ::std::fs::Metadata = ::std::fs::metadata(path)
        .map_err(|_| anyhow::anyhow!("mount directory does not exist: {path}"))?;
    if !metadata.is_dir() {
        anyhow::bail!("mount path is not a directory: {path}");
    }
    Ok(())
}

///
/// # Description
///
/// Checks that a gateway endpoint path is a named pipe path (`\\.\pipe\<name>`), which the
/// gateway endpoint of the Nanvix Daemon is on Windows.
///
/// # Parameters
///
/// - `path`: Gateway endpoint path.
///
/// # Returns
///
/// On success, returns empty. On failure, returns an error: the path is not a named pipe path.
///
#[cfg(windows)]
fn validate_named_pipe_path(path: &str) -> Result<()> {
    const NAMED_PIPE_PREFIX: &str = r"\\.\pipe\";
    let has_named_pipe_prefix: bool = path
        .get(..NAMED_PIPE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(NAMED_PIPE_PREFIX));
    if !has_named_pipe_prefix || path.len() == NAMED_PIPE_PREFIX.len() {
        anyhow::bail!(
            "gateway_sockaddr must be a named pipe path ({NAMED_PIPE_PREFIX}<name>) on Windows: \
             {path}"
        );
    }
    Ok(())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;

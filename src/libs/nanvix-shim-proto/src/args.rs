// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

//! CLI argument parsing for the containerd shimv2 binary protocol.
//!
//! Containerd invokes a shim binary with specific flags and subcommands as defined in the
//! [Runtime v2 specification](https://github.com/containerd/containerd/blob/main/core/runtime/v2/README.md#commands).
//!
//! ## Spec-defined flags (MUST be accepted)
//!
//! ### `start` subcommand
//! - `-namespace`      — the namespace for the container
//! - `-address`        — the address of containerd's main gRPC socket
//! - `-publish-binary` — the binary path to publish events back to containerd
//! - `-id`             — the id of the container
//!
//! The bundle for the container is set as the process's working directory (`cwd`).
//!
//! ### `delete` subcommand
//! - `-namespace`      — the namespace for the container
//! - `-address`        — the address of containerd's main socket
//! - `-publish-binary` — the binary path to publish events back to containerd
//! - `-id`             — the id of the container
//! - `-bundle`         — the path to the bundle to delete (on Windows/FreeBSD; matches `cwd` on other platforms)
//!
//! ### Command-like flags (SHOULD be implemented)
//! - `-v` / `-version` — print the shim version and exit
//! - `-info`           — read option protobuf from stdin, print RuntimeInfo to stdout, and exit
//!
//! ## Implementation-defined flags (not in the spec)
//! - `-debug`  — enable debug-level logging (used by containerd/rust-extensions and Kata)
//! - `-socket` — the ttrpc socket address for the child process to listen on (set by `start`,
//!   consumed by the `run` action internally)
//!
//! ## Environment variables (set by containerd)
//! - `TTRPC_ADDRESS` — address of containerd's ttrpc API socket
//! - `GRPC_ADDRESS`  — address of containerd's gRPC API socket (containerd 1.7+)
//!
//! ## References
//! - [containerd Runtime v2 README](https://github.com/containerd/containerd/blob/main/core/runtime/v2/README.md)
//! - [containerd/rust-extensions shim args](https://github.com/containerd/rust-extensions/blob/main/crates/shim/src/args.rs)

//==================================================================================================
// Types
//==================================================================================================

/// Flags passed from containerd to the shim binary.
///
/// See the [containerd Runtime v2 specification](https://github.com/containerd/containerd/blob/main/core/runtime/v2/README.md#commands)
/// for the authoritative definition.
#[derive(Debug, Default)]
pub struct ShimArgs {
    /// Enable debug-level logging.
    ///
    /// Implementation-defined flag (not in the containerd spec). Used by
    /// containerd/rust-extensions, Kata, and hcsshim.
    pub debug: bool,

    /// Namespace that owns the container.
    ///
    /// Spec-defined: MUST be accepted by `start` and `delete`.
    pub namespace: String,

    /// Identifier of the container/task.
    ///
    /// Spec-defined: MUST be accepted by `start` and `delete`.
    pub id: String,

    /// ttrpc socket address for the shim to listen on.
    ///
    /// Implementation-defined flag. Set by the `start` command and passed to
    /// the child process so it knows where to bind the ttrpc server.
    pub socket: String,

    /// Path to the OCI bundle directory.
    ///
    /// Spec-defined for `delete`: the path to the bundle to delete. On
    /// non-Windows/non-FreeBSD platforms this matches the process `cwd`.
    pub bundle: String,

    /// Address of containerd's main gRPC socket.
    ///
    /// Spec-defined: MUST be accepted by `start` and `delete`.
    pub address: String,

    /// Path to the binary used to publish events back to containerd.
    ///
    /// Spec-defined: MUST be accepted by `start` and `delete`.
    pub publish_binary: String,
}

/// The action the shim was invoked with.
///
/// containerd invokes the shim binary with a positional subcommand:
/// - `start`  — launch a new shim process and return its ttrpc address
/// - `delete` — clean up after a container has exited
/// - (none)   — run the ttrpc server (internal; invoked by the `start` child process)
#[derive(Debug)]
pub enum Action {
    /// Launch a new shim process and return its ttrpc address to stdout.
    Start(ShimArgs),
    /// Clean up resources and write a DeleteResponse protobuf to stdout.
    Delete(ShimArgs),
    /// Run the ttrpc server (internal action, not invoked by containerd directly).
    Run(ShimArgs),
    /// Print version information and exit (`-v` / `-version`).
    Version,
    /// Print usage information and exit.
    Help,
}

//==================================================================================================
// Implementations
//==================================================================================================

/// Parse command-line arguments into an action and flags.
///
/// Follows Go-flag conventions (single dash, space-separated values) to match
/// containerd's invocation style.
pub fn parse_args(args: &[String]) -> anyhow::Result<Action> {
    let mut shim_args: ShimArgs = ShimArgs::default();
    let mut i: usize = 1; // skip argv[0]
    let mut positional: Vec<String> = Vec::new();
    let mut version: bool = false;

    while i < args.len() {
        match args[i].as_str() {
            "-debug" => shim_args.debug = true,
            "-namespace" => {
                i += 1;
                shim_args.namespace = args.get(i).cloned().unwrap_or_default();
            },
            "-id" => {
                i += 1;
                shim_args.id = args.get(i).cloned().unwrap_or_default();
            },
            "-socket" => {
                i += 1;
                shim_args.socket = args.get(i).cloned().unwrap_or_default();
            },
            "-bundle" => {
                i += 1;
                shim_args.bundle = args.get(i).cloned().unwrap_or_default();
            },
            "-address" => {
                i += 1;
                shim_args.address = args.get(i).cloned().unwrap_or_default();
            },
            "-publish-binary" => {
                i += 1;
                shim_args.publish_binary = args.get(i).cloned().unwrap_or_default();
            },
            "-v" | "-version" | "--version" => version = true,
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    if version {
        return Ok(Action::Version);
    }

    match positional.first().map(|s| s.as_str()) {
        Some("start") => Ok(Action::Start(shim_args)),
        Some("delete") => Ok(Action::Delete(shim_args)),
        None => Ok(Action::Run(shim_args)),
        Some(_) => Ok(Action::Run(shim_args)),
    }
}

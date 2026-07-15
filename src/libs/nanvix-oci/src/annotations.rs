// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Nanvix OCI annotation constants.
//!
//! These correspond to the LABEL directives in Nanvix Dockerfiles and are read
//! by the shim to determine how to launch the workload.

/// Target OS — always `"nanvix"`.
pub const OS: &str = "com.nanvix.os";

/// Target architecture (e.g., `"x86"`).
pub const ARCH: &str = "com.nanvix.arch";

/// Path to the application binary (initrd) within the image.
/// Maps to the `-- <app.elf>` argument of `nanvixd`.
pub const INITRD_PATH: &str = "com.nanvix.initrd.path";

/// Optional arguments passed to the application (space-separated).
pub const INITRD_ARGS: &str = "com.nanvix.initrd.args";

/// Optional environment variables (`"KEY1=val1 KEY2=val2"`).
pub const INITRD_ENV: &str = "com.nanvix.initrd.env";

/// Path to the ramfs directory within the image.
/// If absent, no ramfs is attached to the VM.
/// Maps to the `-ramfs <generated.img>` argument of `nanvixd`.
pub const RAMFS_ROOT: &str = "com.nanvix.ramfs.root";

/// Optional Nanvix version compatibility hint.
pub const VERSION: &str = "com.nanvix.version";

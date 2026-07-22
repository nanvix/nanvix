// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Default executable search path.
pub const _PATH_DEFPATH: &str = "/usr/bin:/bin";
/// Standard executable search path, including administrative directories.
pub const _PATH_STDPATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";
/// Path to the Bourne-compatible shell.
pub const _PATH_BSHELL: &str = "/bin/sh";
/// Path to the C shell.
pub const _PATH_CSHELL: &str = "/bin/csh";
/// Path to the system console.
pub const _PATH_CONSOLE: &str = "/dev/console";
/// Path to the null device.
pub const _PATH_DEVNULL: &str = "/dev/null";
/// Path to the controlling terminal.
pub const _PATH_TTY: &str = "/dev/tty";
/// Device directory prefix.
pub const _PATH_DEV: &str = "/dev/";
/// Temporary directory prefix.
pub const _PATH_TMP: &str = "/tmp/";
/// Path to the mounted file-system table.
pub const _PATH_MOUNTED: &str = "/etc/mtab";
/// Path to the file-system table.
pub const _PATH_MNTTAB: &str = "/etc/fstab";
/// Path to the file that disables non-root logins.
pub const _PATH_NOLOGIN: &str = "/etc/nologin";
/// Path to the valid login shells file.
pub const _PATH_SHELLS: &str = "/etc/shells";
/// Path to the login history file.
pub const _PATH_WTMP: &str = "/var/log/wtmp";
/// Path to the active login records file.
pub const _PATH_UTMP: &str = "/var/run/utmp";
/// Path to the last-login records file.
pub const _PATH_LASTLOG: &str = "/var/log/lastlog";

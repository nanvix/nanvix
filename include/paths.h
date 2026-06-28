/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_PATHS_H
#define _NANVIX_PATHS_H

/**
 * @file paths.h
 * @brief Default path prefixes for system files and directories.
 *
 * Defines the `_PATH_*` string constants that name the conventional locations of
 * system files and directories and the default executable search paths.
 */

#define _PATH_DEFPATH "/usr/bin:/bin"
#define _PATH_STDPATH "/usr/bin:/bin:/usr/sbin:/sbin"
#define _PATH_BSHELL "/bin/sh"
#define _PATH_CSHELL "/bin/csh"
#define _PATH_CONSOLE "/dev/console"
#define _PATH_DEVNULL "/dev/null"
#define _PATH_TTY "/dev/tty"
#define _PATH_DEV "/dev/"
#define _PATH_TMP "/tmp/"
#define _PATH_MOUNTED "/etc/mtab"
#define _PATH_MNTTAB "/etc/fstab"
#define _PATH_NOLOGIN "/etc/nologin"
#define _PATH_SHELLS "/etc/shells"
#define _PATH_WTMP "/var/log/wtmp"
#define _PATH_UTMP "/var/run/utmp"
#define _PATH_LASTLOG "/var/log/lastlog"

#endif /* _NANVIX_PATHS_H */

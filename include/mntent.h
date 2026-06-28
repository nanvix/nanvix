/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_MNTENT_H
#define _NANVIX_MNTENT_H

/**
 * @file mntent.h
 * @brief Filesystem table (fstab/mtab) access.
 *
 * Declares the structure and functions used to read and write the filesystem
 * description files (/etc/fstab and /etc/mtab), implemented by the libc_mntent
 * Rust crate as thin parsers layered on top of <stdio.h>.
 */

#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Paths and Options
 *==================================================================================================*/

#define MNTTAB  "/etc/fstab"
#define MOUNTED "/etc/mtab"

#define MNTTYPE_IGNORE "ignore"
#define MNTTYPE_NFS    "nfs"
#define MNTTYPE_SWAP   "swap"

#define MNTOPT_DEFAULTS "defaults"
#define MNTOPT_RO       "ro"
#define MNTOPT_RW       "rw"
#define MNTOPT_SUID     "suid"
#define MNTOPT_NOSUID   "nosuid"
#define MNTOPT_NOAUTO   "noauto"

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Filesystem table entry. */
struct mntent {
    char *mnt_fsname; /**< Device or remote filesystem.  */
    char *mnt_dir;    /**< Mount point.                  */
    char *mnt_type;   /**< Filesystem type.              */
    char *mnt_opts;   /**< Comma-separated mount options.*/
    int mnt_freq;     /**< Dump frequency in days.       */
    int mnt_passno;   /**< Pass number on parallel fsck. */
};

/*==================================================================================================
 * Filesystem Table Access
 *==================================================================================================*/

extern FILE *setmntent(const char *filename, const char *type);
extern struct mntent *getmntent(FILE *stream);
extern struct mntent *getmntent_r(FILE *stream, struct mntent *result, char *buffer, int bufsize);
extern int addmntent(FILE *stream, const struct mntent *mnt);
extern int endmntent(FILE *stream);
extern char *hasmntopt(const struct mntent *mnt, const char *opt);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_MNTENT_H */

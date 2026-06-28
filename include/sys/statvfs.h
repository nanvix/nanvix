/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_STATVFS_H
#define _NANVIX_SYS_STATVFS_H

/**
 * @file sys/statvfs.h
 * @brief File-system information.
 *
 * Declares the statvfs()/fstatvfs() interfaces and the struct statvfs they
 * populate. Nanvix does not yet expose per-file-system statistics, so the backing
 * implementations are stubs that fail with ENOSYS; the declarations exist so that
 * portable software which queries file-system geometry compiles and links.
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * File-System Information
 *==================================================================================================*/

/** @brief File-system information. */
struct statvfs {
    unsigned long f_bsize;   /**< File-system block size.                 */
    unsigned long f_frsize;  /**< Fundamental file-system block size.     */
    fsblkcnt_t    f_blocks;  /**< Total blocks (in f_frsize units).       */
    fsblkcnt_t    f_bfree;   /**< Total free blocks.                      */
    fsblkcnt_t    f_bavail;  /**< Free blocks available to non-superuser.  */
    fsfilcnt_t    f_files;   /**< Total file nodes (inodes).              */
    fsfilcnt_t    f_ffree;   /**< Total free file nodes (inodes).         */
    fsfilcnt_t    f_favail;  /**< Free file nodes for non-superuser.      */
    unsigned long f_fsid;    /**< File-system identifier.                 */
    unsigned long f_flag;    /**< Bitwise-or of ST_* mount flags.         */
    unsigned long f_namemax; /**< Maximum filename length.                */
};

/* f_flag bits. */
#define ST_RDONLY 1 /**< Read-only file system. */
#define ST_NOSUID 2 /**< Set-user/group-ID bits ignored on exec. */

extern int statvfs(const char *path, struct statvfs *buf);
extern int fstatvfs(int fd, struct statvfs *buf);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_STATVFS_H */

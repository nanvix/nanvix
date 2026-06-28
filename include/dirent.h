/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_DIRENT_H
#define _NANVIX_DIRENT_H

/**
 * @file dirent.h
 * @brief Directory entries.
 *
 * Declares the directory-stream type, the directory-entry structures, and the
 * directory-traversal interfaces. The layouts mirror the Rust definitions in the
 * sysapi crate (dirent.rs).
 */

#include <sys/types.h>
#include <limits.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define DT_UNKNOWN 0 /**< Unknown file type. */
#define DT_FIFO 1 /**< FIFO. */
#define DT_CHR 2 /**< Character device. */
#define DT_DIR 4 /**< Directory. */
#define DT_BLK 6 /**< Block device. */
#define DT_REG 8 /**< Regular file. */
#define DT_LNK 10 /**< Symbolic link. */
#define DT_SOCK 12 /**< Socket. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Directory stream type. */
typedef struct __dirstream DIR;

/** @brief Directory entry. */
struct dirent {
    ino_t d_ino;                /**< File serial number.                  */
    char d_name[NAME_MAX + 1];  /**< Null-terminated file name.           */
};

/** @brief Extended (Nanvix) directory entry. */
struct posix_dent {
    ino_t d_ino;                /**< File serial number.                  */
    reclen_t d_reclen;          /**< Length of this entry.                */
    unsigned char d_type;       /**< File type.                           */
    char d_name[NAME_MAX + 1];  /**< Null-terminated file name.           */
    char d_pad[1];              /**< Padding.                             */
};

/*==================================================================================================
 * Directory Operations
 *==================================================================================================*/

extern DIR *opendir(const char *dirname);
extern DIR *fdopendir(int fd);
extern struct dirent *readdir(DIR *dirp);
extern int closedir(DIR *dirp);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_DIRENT_H */

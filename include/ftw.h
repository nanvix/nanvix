/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_FTW_H
#define _NANVIX_FTW_H

/**
 * @file ftw.h
 * @brief File-tree-walk interface.
 */

#include <sys/stat.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Type-flag values passed to the callback
 *==================================================================================================*/

#define FTW_F 0   /**< Regular file.                 */
#define FTW_D 1   /**< Directory.                    */
#define FTW_DNR 2 /**< Directory that cannot be read. */
#define FTW_NS 3  /**< Stat failed.                  */
#define FTW_SL 4  /**< Symbolic link.                */

/*==================================================================================================
 * Functions
 *==================================================================================================*/

/**
 * @brief Walks the file tree rooted at @p path, invoking @p fn for each entry.
 *
 * @param path    Root of the tree to walk.
 * @param fn      Callback invoked per entry with its path, stat data, and a type flag.
 * @param nopenfd Maximum number of directory streams kept open simultaneously.
 *
 * @returns Zero on success, the callback's non-zero return, or -1 on error.
 */
extern int ftw(
    const char *path, int (*fn)(const char *fpath, const struct stat *sb, int typeflag),
    int nopenfd);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_FTW_H */

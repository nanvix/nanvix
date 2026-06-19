/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_UTIME_H
#define _NANVIX_UTIME_H

/**
 * @file utime.h
 * @brief File access and modification times.
 *
 * Declares the legacy `utime()` interface and its `utimbuf` structure. The
 * layout mirrors the Rust definition in the sysapi crate (utime.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Structures
 *==================================================================================================*/

/**
 * @brief File access and modification times.
 */
struct utimbuf {
    time_t actime;  /**< Access time.       */
    time_t modtime; /**< Modification time. */
};

/*==================================================================================================
 * Functions
 *==================================================================================================*/

/**
 * @brief Sets the access and modification times of a file.
 *
 * @param filename Path of the target file.
 * @param times    Desired access and modification times, or NULL for the
 *                 current time.
 *
 * @returns Zero on success, or -1 on failure with `errno` set.
 */
extern int utime(const char *filename, const struct utimbuf *times);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_UTIME_H */

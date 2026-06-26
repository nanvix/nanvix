/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_LIBGEN_H
#define _NANVIX_LIBGEN_H

/**
 * @file libgen.h
 * @brief Definitions for pattern matching functions.
 *
 * Declares basename() and dirname(), the POSIX pathname component functions
 * implemented by the libc_libgen Rust crate.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Pathname Manipulation
 *==================================================================================================*/

extern char *basename(char *path);
extern char *dirname(char *path);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_LIBGEN_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_FNMATCH_H
#define _NANVIX_FNMATCH_H

/**
 * @file fnmatch.h
 * @brief Filename matching.
 *
 * Declares the filename-matching routine fnmatch() and its flags, implemented by
 * the libc_fnmatch Rust crate.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Flags
 *==================================================================================================*/

#define FNM_NOMATCH 1  /* Value returned on no match. */

#define FNM_PATHNAME (1 << 0) /* No wildcard can ever match '/'. */
#define FNM_PERIOD   (1 << 2) /* Leading '.' is matched only explicitly. */
#define FNM_NOESCAPE (1 << 1) /* Backslashes do not quote special chars. */
#define FNM_CASEFOLD (1 << 4) /* Compare case-insensitively. */

/* Equivalent to FNM_CASEFOLD. */
#define FNM_IGNORECASE FNM_CASEFOLD

/*==================================================================================================
 * Filename Matching
 *==================================================================================================*/

extern int fnmatch(const char *pattern, const char *string, int flags);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_FNMATCH_H */

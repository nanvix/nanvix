/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_LIMITS_H
#define _NANVIX_LIMITS_H

/**
 * @file limits.h
 * @brief Implementation-defined constants.
 *
 * Declares the numerical limits of the standard integer types (via compiler
 * builtins) and the POSIX path/name/resource limits. The POSIX limits mirror the
 * Rust definitions in the sysapi crate (limits.rs).
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

/* Numerical limits of the standard integer types (compiler-defined). */
#define CHAR_BIT 8
#define MB_LEN_MAX 4

#define SCHAR_MIN (-__SCHAR_MAX__ - 1)
#define SCHAR_MAX __SCHAR_MAX__
#define UCHAR_MAX (__SCHAR_MAX__ * 2 + 1)

/* Plain char follows the target ABI: unsigned on AArch64, signed on x86. */
#ifdef __CHAR_UNSIGNED__
#define CHAR_MIN 0
#define CHAR_MAX UCHAR_MAX
#else
#define CHAR_MIN SCHAR_MIN
#define CHAR_MAX SCHAR_MAX
#endif

#define SHRT_MIN (-__SHRT_MAX__ - 1)
#define SHRT_MAX __SHRT_MAX__
#define USHRT_MAX (__SHRT_MAX__ * 2 + 1)

#define INT_MIN (-__INT_MAX__ - 1)
#define INT_MAX __INT_MAX__
#define UINT_MAX (__INT_MAX__ * 2U + 1U)

#define LONG_MIN (-__LONG_MAX__ - 1L)
#define LONG_MAX __LONG_MAX__
#define ULONG_MAX (__LONG_MAX__ * 2UL + 1UL)

#define LLONG_MIN (-__LONG_LONG_MAX__ - 1LL)
#define LLONG_MAX __LONG_LONG_MAX__
#define ULLONG_MAX (__LONG_LONG_MAX__ * 2ULL + 1ULL)

/* POSIX path, name, and resource limits (sysapi/limits.rs). */
#define HOST_NAME_MAX 255
#define IOV_MAX 16
#define NAME_MAX 255
#define OPEN_MAX 64
#define PATH_MAX 1024
#define PTHREAD_KEYS_MAX 128
#define SSIZE_MAX INT_MAX

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_LIMITS_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_PARAM_H
#define _NANVIX_SYS_PARAM_H

/**
 * @file sys/param.h
 * @brief Legacy system parameters and helper macros.
 */

#include <limits.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Limits
 *==================================================================================================*/

#ifndef MAXPATHLEN
#ifdef PATH_MAX
#define MAXPATHLEN PATH_MAX
#else
#define MAXPATHLEN 4096
#endif
#endif

#ifndef NBBY
#define NBBY 8 /**< Number of bits in a byte. */
#endif

/*==================================================================================================
 * Helper macros
 *==================================================================================================*/

#ifndef MIN
#define MIN(a, b) (((a) < (b)) ? (a) : (b))
#endif
#ifndef MAX
#define MAX(a, b) (((a) > (b)) ? (a) : (b))
#endif

#define howmany(x, y) (((x) + ((y) - 1)) / (y))
#define roundup(x, y) ((((x) + ((y) - 1)) / (y)) * (y))
#define powerof2(x) ((((x) - 1) & (x)) == 0)

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_PARAM_H */

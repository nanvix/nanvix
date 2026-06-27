/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_SYSMACROS_H
#define _NANVIX_SYS_SYSMACROS_H

/**
 * @file sys/sysmacros.h
 * @brief Device number macros.
 *
 * `major()` and `minor()` extract the major and minor components of a `dev_t`
 * device number, and `makedev()` composes one from a major/minor pair.
 */

#include <sys/types.h>

#ifndef major
#define major(dev) ((unsigned int)(((dev_t)(dev) >> 8) & 0xffu))
#endif
#ifndef minor
#define minor(dev) ((unsigned int)((dev_t)(dev) & 0xffu))
#endif
#ifndef makedev
#define makedev(maj, min) ((dev_t)(((((dev_t)(maj)) & 0xffu) << 8) | (((dev_t)(min)) & 0xffu)))
#endif

#endif /* _NANVIX_SYS_SYSMACROS_H */

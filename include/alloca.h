/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_ALLOCA_H
#define _NANVIX_ALLOCA_H

/**
 * @file alloca.h
 * @brief Stack allocation.
 *
 * `alloca()` allocates memory in the caller's stack frame; the storage is freed
 * automatically when the calling function returns. It maps directly to the
 * compiler builtin.
 */

#include <stddef.h>

#ifndef alloca
#define alloca(size) __builtin_alloca(size)
#endif

#endif /* _NANVIX_ALLOCA_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDBOOL_H
#define _NANVIX_STDBOOL_H

/**
 * @file stdbool.h
 * @brief Boolean type and values.
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers.
 */

#ifndef __cplusplus

/** @brief Boolean type. */
#define bool _Bool

/** @brief Boolean true. */
#define true 1

/** @brief Boolean false. */
#define false 0

#endif /* !__cplusplus */

/** @brief Indicates that `bool`, `true`, and `false` are defined. */
#define __bool_true_false_are_defined 1

#endif /* _NANVIX_STDBOOL_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDALIGN_H
#define _NANVIX_STDALIGN_H

/**
 * @file stdalign.h
 * @brief Alignment convenience macros.
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers. The macros expand to the
 * `_Alignas` and `_Alignof` operators that both clang and gcc provide for the
 * active target.
 */

#ifndef __cplusplus

/** @brief Convenience spelling of the `_Alignas` specifier. */
#ifndef alignas
#define alignas _Alignas
#endif

/** @brief Convenience spelling of the `_Alignof` operator. */
#ifndef alignof
#define alignof _Alignof
#endif

#endif /* !__cplusplus */

/** @brief Indicates that `alignas` is defined. */
#define __alignas_is_defined 1

/** @brief Indicates that `alignof` is defined. */
#define __alignof_is_defined 1

#endif /* _NANVIX_STDALIGN_H */

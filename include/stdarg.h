/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDARG_H
#define _NANVIX_STDARG_H

/**
 * @file stdarg.h
 * @brief Variable argument lists.
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers. The machinery maps onto
 * the compiler's `__builtin_va_*` primitives, which both clang and gcc provide
 * for the active target ABI.
 */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/*
 * Guarded with `_VA_LIST_DEFINED` so other headers that need `va_list` in scope
 * (for example <stdio.h> and <wchar.h>) share this single definition instead of
 * emitting a competing typedef.
 */
#ifndef _VA_LIST_DEFINED
#define _VA_LIST_DEFINED

/** @brief Type for iterating over a function's variable arguments. */
typedef __builtin_va_list va_list;

/** @brief Legacy GNU spelling of `va_list`, kept for ported sources. */
typedef __builtin_va_list __gnuc_va_list;

#endif

/*==================================================================================================
 * Macros
 *==================================================================================================*/

/** @brief Initializes @p ap to retrieve the arguments following @p param. */
#define va_start(ap, param) __builtin_va_start(ap, param)

/** @brief Releases @p ap once argument retrieval is complete. */
#define va_end(ap) __builtin_va_end(ap)

/** @brief Returns the next argument of the given @p type. */
#define va_arg(ap, type) __builtin_va_arg(ap, type)

/** @brief Copies the state of @p src into @p dst. */
#define va_copy(dst, src) __builtin_va_copy(dst, src)

/** @brief Legacy GNU spelling of `va_copy`, kept for ported sources. */
#define __va_copy(dst, src) __builtin_va_copy(dst, src)

#endif /* _NANVIX_STDARG_H */

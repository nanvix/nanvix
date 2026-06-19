/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_ASSERT_H
#define _NANVIX_ASSERT_H

/**
 * @file assert.h
 * @brief Diagnostics.
 *
 * Provides the assert() macro for runtime assertions. The underlying
 * __assert_func() is implemented by the libc_assert Rust crate and matches
 * the signature expected by NewLib's <assert.h>.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Internal Functions (do not call directly)
 *==================================================================================================*/

extern void __assert_func(const char *file, int line, const char *function, const char *expression);
extern void __assert(const char *file, int line, const char *expression);

#ifdef __cplusplus
}
#endif

/*==================================================================================================
 * assert Macro
 *==================================================================================================*/

/* The assert macro must be redefinable, so it lives outside the include guard. */
#undef assert

#ifdef NDEBUG
#define assert(expr) ((void)0)
#else
#define assert(expr) ((expr) ? (void)0 : __assert_func(__FILE__, __LINE__, __func__, #expr))
#endif

#endif /* _NANVIX_ASSERT_H */

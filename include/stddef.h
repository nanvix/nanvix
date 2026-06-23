/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDDEF_H
#define _NANVIX_STDDEF_H

/**
 * @file stddef.h
 * @brief Common definitions.
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers. The underlying types
 * are obtained from compiler-predefined macros (`__SIZE_TYPE__`, ...), which
 * both clang and gcc supply for the active target, so the layouts always match
 * the selected ABI.
 */

/*==================================================================================================
 * Constants
 *==================================================================================================*/

/** @brief Null pointer constant. */
#ifndef NULL
#ifdef __cplusplus
#define NULL 0
#else
#define NULL ((void *)0)
#endif
#endif

/*==================================================================================================
 * Macros
 *==================================================================================================*/

/** @brief Offset (in bytes) of a member within a structure. */
#define offsetof(type, member) __builtin_offsetof(type, member)

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Signed result of subtracting two pointers. */
#ifndef _PTRDIFF_T_DEFINED
#define _PTRDIFF_T_DEFINED
typedef __PTRDIFF_TYPE__ ptrdiff_t;
#endif

/** @brief Unsigned result of the `sizeof` operator. */
#ifndef _SIZE_T_DEFINED
#define _SIZE_T_DEFINED
typedef __SIZE_TYPE__ size_t;
#endif

/** @brief Wide-character type (C only; C++ has a built-in `wchar_t`). */
#ifndef __cplusplus
#ifndef _WCHAR_T_DEFINED
#define _WCHAR_T_DEFINED
typedef __WCHAR_TYPE__ wchar_t;
#endif
#endif

/** @brief Type with the greatest fundamental alignment. */
#ifndef _MAX_ALIGN_T_DEFINED
#define _MAX_ALIGN_T_DEFINED
typedef struct {
    long long __max_align_ll;
    long double __max_align_ld;
} max_align_t;
#endif

#endif /* _NANVIX_STDDEF_H */

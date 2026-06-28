/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_FLOAT_H
#define _NANVIX_FLOAT_H

/**
 * @file float.h
 * @brief Characteristics of floating-point types.
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers. Every limit is derived
 * from compiler-predefined macros (`__FLT_MAX__`, `__DBL_MANT_DIG__`, ...),
 * which both clang and gcc supply for the active target, so the values always
 * match the selected ABI.
 */

/*==================================================================================================
 * Rounding and evaluation
 *==================================================================================================*/

/** @brief Rounding mode of floating-point addition. */
#if defined(__has_builtin) && __has_builtin(__builtin_flt_rounds)
#define FLT_ROUNDS (__builtin_flt_rounds())
#else
#define FLT_ROUNDS 1
#endif

/** @brief Evaluation format of floating-point operations. */
#define FLT_EVAL_METHOD __FLT_EVAL_METHOD__

/** @brief Radix of the exponent representation. */
#define FLT_RADIX __FLT_RADIX__

/** @brief Number of decimal digits that survive a round trip through `long double`. */
#define DECIMAL_DIG __DECIMAL_DIG__

/*==================================================================================================
 * float
 *==================================================================================================*/

#define FLT_MANT_DIG __FLT_MANT_DIG__
#define FLT_DIG __FLT_DIG__
#define FLT_MIN_EXP __FLT_MIN_EXP__
#define FLT_MIN_10_EXP __FLT_MIN_10_EXP__
#define FLT_MAX_EXP __FLT_MAX_EXP__
#define FLT_MAX_10_EXP __FLT_MAX_10_EXP__
#define FLT_MAX __FLT_MAX__
#define FLT_EPSILON __FLT_EPSILON__
#define FLT_MIN __FLT_MIN__
#define FLT_TRUE_MIN __FLT_DENORM_MIN__
#define FLT_DECIMAL_DIG __FLT_DECIMAL_DIG__
#define FLT_HAS_SUBNORM __FLT_HAS_DENORM__

/*==================================================================================================
 * double
 *==================================================================================================*/

#define DBL_MANT_DIG __DBL_MANT_DIG__
#define DBL_DIG __DBL_DIG__
#define DBL_MIN_EXP __DBL_MIN_EXP__
#define DBL_MIN_10_EXP __DBL_MIN_10_EXP__
#define DBL_MAX_EXP __DBL_MAX_EXP__
#define DBL_MAX_10_EXP __DBL_MAX_10_EXP__
#define DBL_MAX __DBL_MAX__
#define DBL_EPSILON __DBL_EPSILON__
#define DBL_MIN __DBL_MIN__
#define DBL_TRUE_MIN __DBL_DENORM_MIN__
#define DBL_DECIMAL_DIG __DBL_DECIMAL_DIG__
#define DBL_HAS_SUBNORM __DBL_HAS_DENORM__

/*==================================================================================================
 * long double
 *==================================================================================================*/

#define LDBL_MANT_DIG __LDBL_MANT_DIG__
#define LDBL_DIG __LDBL_DIG__
#define LDBL_MIN_EXP __LDBL_MIN_EXP__
#define LDBL_MIN_10_EXP __LDBL_MIN_10_EXP__
#define LDBL_MAX_EXP __LDBL_MAX_EXP__
#define LDBL_MAX_10_EXP __LDBL_MAX_10_EXP__
#define LDBL_MAX __LDBL_MAX__
#define LDBL_EPSILON __LDBL_EPSILON__
#define LDBL_MIN __LDBL_MIN__
#define LDBL_TRUE_MIN __LDBL_DENORM_MIN__
#define LDBL_DECIMAL_DIG __LDBL_DECIMAL_DIG__
#define LDBL_HAS_SUBNORM __LDBL_HAS_DENORM__

#endif /* _NANVIX_FLOAT_H */

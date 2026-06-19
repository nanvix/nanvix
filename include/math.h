/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_MATH_H
#define _NANVIX_MATH_H

/**
 * @file math.h
 * @brief Mathematical functions.
 *
 * Declares software floating-point math functions covering trigonometric,
 * exponential, logarithmic, rounding, and classification operations.
 * Implemented by the libc_math Rust crate.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Real-Floating Types
 *==================================================================================================*/

typedef float float_t;
typedef double double_t;

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#ifndef HUGE_VAL
#define HUGE_VAL (__builtin_huge_val())
#endif

#ifndef HUGE_VALF
#define HUGE_VALF (__builtin_huge_valf())
#endif

#ifndef HUGE_VALL
#define HUGE_VALL (__builtin_huge_vall())
#endif

#ifndef INFINITY
#define INFINITY (__builtin_inff())
#endif

#ifndef NAN
#define NAN (__builtin_nanf(""))
#endif

/*==================================================================================================
 * Floating-Point Classification Constants
 *==================================================================================================*/

#define FP_NAN 0
#define FP_INFINITE 1
#define FP_ZERO 2
#define FP_SUBNORMAL 3
#define FP_NORMAL 4

/* Values returned by ilogb() for a zero or NaN argument. */
#define FP_ILOGB0 (-2147483647 - 1)
#define FP_ILOGBNAN 2147483647

/*==================================================================================================
 * Error Handling
 *==================================================================================================*/

#define MATH_ERRNO 1
#define MATH_ERREXCEPT 2
#define math_errhandling 0

/*==================================================================================================
 * Classification Functions (internal)
 *==================================================================================================*/

extern int __fpclassifyf(float x);
extern int __fpclassifyd(double x);
extern int __isnanf(float x);
extern int __isnand(double x);
extern int __isinff(float x);
extern int __isinfd(double x);
extern int __signbitf(float x);
extern int __signbitd(double x);

/*==================================================================================================
 * Classification Macros
 *==================================================================================================*/

#define fpclassify(x) (sizeof(x) == sizeof(float) ? __fpclassifyf(x) : __fpclassifyd(x))

#define isnan(x) (sizeof(x) == sizeof(float) ? __isnanf(x) : __isnand(x))

#define isinf(x) (sizeof(x) == sizeof(float) ? __isinff(x) : __isinfd(x))

#define signbit(x) (sizeof(x) == sizeof(float) ? __signbitf(x) : __signbitd(x))

#define isfinite(x) __builtin_isfinite(x)
#define isnormal(x) __builtin_isnormal(x)

/*==================================================================================================
 * Comparison Macros
 *==================================================================================================*/

#define isgreater(x, y) __builtin_isgreater(x, y)
#define isgreaterequal(x, y) __builtin_isgreaterequal(x, y)
#define isless(x, y) __builtin_isless(x, y)
#define islessequal(x, y) __builtin_islessequal(x, y)
#define islessgreater(x, y) __builtin_islessgreater(x, y)
#define isunordered(x, y) __builtin_isunordered(x, y)

/*==================================================================================================
 * Trigonometric Functions
 *==================================================================================================*/

extern double sin(double x);
extern double cos(double x);
extern double tan(double x);
extern float sinf(float x);
extern float cosf(float x);
extern float tanf(float x);

/*==================================================================================================
 * Hyperbolic Functions
 *==================================================================================================*/

extern double sinh(double x);
extern double cosh(double x);
extern double tanh(double x);
extern double asinh(double x);
extern double acosh(double x);
extern double atanh(double x);

/*==================================================================================================
 * Inverse Trigonometric Functions
 *==================================================================================================*/

extern double asin(double x);
extern double acos(double x);
extern double atan(double x);
extern double atan2(double y, double x);
extern float asinf(float x);
extern float acosf(float x);
extern float atanf(float x);
extern float atan2f(float y, float x);

/*==================================================================================================
 * Exponential and Logarithmic Functions
 *==================================================================================================*/

extern double exp(double x);
extern double exp2(double x);
extern double log(double x);
extern double log2(double x);
extern double log10(double x);
extern double pow(double x, double y);
extern double expm1(double x);
extern double log1p(double x);
extern long lrint(double x);
extern double erf(double x);
extern double erfc(double x);
extern double lgamma(double x);
extern double tgamma(double x);
extern double gamma(double x);
extern double nextafter(double x, double y);
extern double remainder(double x, double y);
extern float expf(float x);
extern float exp2f(float x);
extern float logf(float x);
extern float log2f(float x);
extern float log10f(float x);
extern float powf(float x, float y);

/*==================================================================================================
 * Square Root and Cube Root
 *==================================================================================================*/

extern double sqrt(double x);
extern double cbrt(double x);
extern double hypot(double x, double y);
extern float sqrtf(float x);
extern float cbrtf(float x);
extern float hypotf(float x, float y);

/*==================================================================================================
 * Rounding and Truncation
 *==================================================================================================*/

extern double ceil(double x);
extern double floor(double x);
extern double round(double x);
extern double trunc(double x);
extern float ceilf(float x);
extern float floorf(float x);
extern float roundf(float x);
extern float truncf(float x);

/*==================================================================================================
 * Absolute Value and Remainder
 *==================================================================================================*/

extern double fabs(double x);
extern double fmod(double x, double y);
extern float fabsf(float x);
extern float fmodf(float x, float y);

/*==================================================================================================
 * Floating-Point Manipulation
 *==================================================================================================*/

extern double copysign(double x, double y);
extern double ldexp(double x, int exp);
extern double scalbn(double x, int n);
extern double frexp(double x, int *exp);
extern double modf(double x, double *iptr);
extern float copysignf(float x, float y);
extern float ldexpf(float x, int exp);
extern float scalbnf(float x, int n);
extern float frexpf(float x, int *exp);
extern float modff(float x, float *iptr);

/*==================================================================================================
 * Min / Max
 *==================================================================================================*/

extern double fmin(double x, double y);
extern double fmax(double x, double y);
extern float fminf(float x, float y);
extern float fmaxf(float x, float y);

/*==================================================================================================
 * Fused Multiply-Add
 *==================================================================================================*/

extern double fma(double x, double y, double z);
extern float fmaf(float x, float y, float z);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_MATH_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_FENV_H
#define _NANVIX_FENV_H

/**
 * @file fenv.h
 * @brief Floating-point environment.
 *
 * Declares the floating-point rounding-mode and exception interfaces. The
 * rounding-mode constants follow the x86 control-word layout.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Floating-point environment. */
typedef int fenv_t;

/** @brief Floating-point exception flags. */
typedef int fexcept_t;

/*==================================================================================================
 * Rounding modes
 *==================================================================================================*/

#define FE_TONEAREST 0x0000
#define FE_DOWNWARD 0x0400
#define FE_UPWARD 0x0800
#define FE_TOWARDZERO 0x0c00

/*==================================================================================================
 * Exception flags
 *==================================================================================================*/

#define FE_INVALID 0x01
#define FE_DENORMAL 0x02
#define FE_DIVBYZERO 0x04
#define FE_OVERFLOW 0x08
#define FE_UNDERFLOW 0x10
#define FE_INEXACT 0x20
#define FE_ALL_EXCEPT                                                                              \
    (FE_INVALID | FE_DENORMAL | FE_DIVBYZERO | FE_OVERFLOW | FE_UNDERFLOW | FE_INEXACT)

/** @brief Default floating-point environment. */
#define FE_DFL_ENV ((const fenv_t *)-1)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int fegetround(void);
extern int fesetround(int mode);
extern int feclearexcept(int excepts);
extern int feraiseexcept(int excepts);
extern int fetestexcept(int excepts);
extern int fegetenv(fenv_t *envp);
extern int fesetenv(const fenv_t *envp);
extern int feholdexcept(fenv_t *envp);
extern int feupdateenv(const fenv_t *envp);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_FENV_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_LOCALE_H
#define _NANVIX_LOCALE_H

/**
 * @file locale.h
 * @brief Localization.
 *
 * Declares functions and types for locale-specific formatting. Only the
 * C/POSIX locale is supported. Implemented by the libc_locale Rust crate.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define LC_CTYPE 0
#define LC_NUMERIC 1
#define LC_TIME 2
#define LC_COLLATE 3
#define LC_MONETARY 4
#define LC_MESSAGES 5
#define LC_ALL 6

/*==================================================================================================
 * Types
 *==================================================================================================*/

#ifndef _LOCALE_T_DEFINED
#define _LOCALE_T_DEFINED
typedef void *locale_t;
#endif

/** @brief Locale-specific numeric formatting information. */
struct lconv {
    char *decimal_point;     /**< Decimal point character.            */
    char *thousands_sep;     /**< Thousands separator.                */
    char *grouping;          /**< Digit grouping.                     */
    char *int_curr_symbol;   /**< International currency symbol.      */
    char *currency_symbol;   /**< Local currency symbol.              */
    char *mon_decimal_point; /**< Monetary decimal point.             */
    char *mon_thousands_sep; /**< Monetary thousands separator.       */
    char *mon_grouping;      /**< Monetary digit grouping.            */
    char *positive_sign;     /**< Positive sign string.               */
    char *negative_sign;     /**< Negative sign string.               */
    char int_frac_digits;    /**< International fractional digits.    */
    char frac_digits;        /**< Local fractional digits.            */
    char p_cs_precedes;      /**< Currency symbol precedes positive.  */
    char p_sep_by_space;     /**< Space separates symbol and positive.*/
    char n_cs_precedes;      /**< Currency symbol precedes negative.  */
    char n_sep_by_space;     /**< Space separates symbol and negative.*/
    char p_sign_posn;        /**< Positive sign position.             */
    char n_sign_posn;        /**< Negative sign position.             */
};

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern char *setlocale(int category, const char *locale);
extern struct lconv *localeconv(void);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_LOCALE_H */

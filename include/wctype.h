/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_WCTYPE_H
#define _NANVIX_WCTYPE_H

/**
 * @file wctype.h
 * @brief Wide character classification and conversion.
 *
 * Declares functions for testing and mapping wide characters. Implemented by
 * the libc_wctype Rust crate.
 */

#include <locale.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

#ifndef _WINT_T_DEFINED
#define _WINT_T_DEFINED
typedef int wint_t;
#endif

#ifndef _WCTRANS_T_DEFINED
#define _WCTRANS_T_DEFINED
typedef int wctrans_t;
#endif

#ifndef _WCTYPE_T_DEFINED
#define _WCTYPE_T_DEFINED
typedef int wctype_t;
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#ifndef WEOF
#define WEOF ((wint_t)(-1))
#endif

/*==================================================================================================
 * Classification
 *==================================================================================================*/

extern int iswalpha(wint_t wc);
extern int iswalpha_l(wint_t wc, locale_t locale);
extern int iswdigit(wint_t wc);
extern int iswdigit_l(wint_t wc, locale_t locale);
extern int iswalnum(wint_t wc);
extern int iswalnum_l(wint_t wc, locale_t locale);
extern int iswspace(wint_t wc);
extern int iswspace_l(wint_t wc, locale_t locale);
extern int iswblank(wint_t wc);
extern int iswblank_l(wint_t wc, locale_t locale);
extern int iswupper(wint_t wc);
extern int iswupper_l(wint_t wc, locale_t locale);
extern int iswlower(wint_t wc);
extern int iswlower_l(wint_t wc, locale_t locale);
extern int iswprint(wint_t wc);
extern int iswprint_l(wint_t wc, locale_t locale);
extern int iswgraph(wint_t wc);
extern int iswgraph_l(wint_t wc, locale_t locale);
extern int iswpunct(wint_t wc);
extern int iswpunct_l(wint_t wc, locale_t locale);
extern int iswcntrl(wint_t wc);
extern int iswcntrl_l(wint_t wc, locale_t locale);
extern int iswxdigit(wint_t wc);
extern int iswxdigit_l(wint_t wc, locale_t locale);
extern int iswctype(wint_t wc, wctype_t charclass);
extern int iswctype_l(wint_t wc, wctype_t charclass, locale_t locale);

/*==================================================================================================
 * Conversion
 *==================================================================================================*/

extern wint_t towupper(wint_t wc);
extern wint_t towupper_l(wint_t wc, locale_t locale);
extern wint_t towlower(wint_t wc);
extern wint_t towlower_l(wint_t wc, locale_t locale);
extern wint_t towctrans(wint_t wc, wctrans_t desc);
extern wint_t towctrans_l(wint_t wc, wctrans_t desc, locale_t locale);

/*==================================================================================================
 * Descriptors
 *==================================================================================================*/

extern wctrans_t wctrans(const char *charclass);
extern wctrans_t wctrans_l(const char *charclass, locale_t locale);
extern wctype_t wctype(const char *property);
extern wctype_t wctype_l(const char *property, locale_t locale);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_WCTYPE_H */

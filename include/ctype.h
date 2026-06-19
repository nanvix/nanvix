/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_CTYPE_H
#define _NANVIX_CTYPE_H

/**
 * @file ctype.h
 * @brief Character classification and conversion.
 *
 * Declares functions for testing and mapping characters. All functions operate
 * on values representable as unsigned char or the value EOF. Implemented by
 * the libc_ctype Rust crate.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Classification
 *==================================================================================================*/

extern int isalpha(int c);
extern int isdigit(int c);
extern int isalnum(int c);
extern int isspace(int c);
extern int isblank(int c);
extern int isupper(int c);
extern int islower(int c);
extern int isprint(int c);
extern int isgraph(int c);
extern int ispunct(int c);
extern int iscntrl(int c);
extern int isxdigit(int c);
extern int isascii(int c);

/*==================================================================================================
 * Conversion
 *==================================================================================================*/

extern int toupper(int c);
extern int tolower(int c);
extern int toascii(int c);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_CTYPE_H */

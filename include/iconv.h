/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_ICONV_H
#define _NANVIX_ICONV_H

/**
 * @file iconv.h
 * @brief Codeset conversion.
 *
 * Nanvix provides an identity-passthrough `iconv` (sufficient for ASCII/UTF-8
 * workloads); it does not perform real codeset conversion.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/** @brief Conversion descriptor. */
typedef void *iconv_t;

extern iconv_t iconv_open(const char *tocode, const char *fromcode);
extern size_t iconv(
    iconv_t cd, char **inbuf, size_t *inbytesleft, char **outbuf, size_t *outbytesleft);
extern int iconv_close(iconv_t cd);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_ICONV_H */

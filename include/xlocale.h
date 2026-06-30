/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_XLOCALE_H
#define _NANVIX_XLOCALE_H

/**
 * @file xlocale.h
 * @brief Compatibility shim for the POSIX xlocale API.
 *
 * Re-includes the headers that declare the locale-management functions and the
 * per-locale `*_l` functions. Provided for toolchains and sources that include
 * <xlocale.h> directly; Nanvix declares each symbol in its primary header.
 */

#include <locale.h>
#include <ctype.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>

#endif /* _NANVIX_XLOCALE_H */

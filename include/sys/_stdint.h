/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS__STDINT_H
#define _NANVIX_SYS__STDINT_H

/**
 * @file sys/_stdint.h
 * @brief Compatibility shim for newlib's internal `<sys/_stdint.h>`.
 *
 * Some ports (e.g. QuickJS) include the newlib-internal `<sys/_stdint.h>`
 * directly to obtain the fixed-width integer types. Nanvix declares those types
 * in `<stdint.h>`, so forward to it.
 */

#include <stdint.h>

#endif /* _NANVIX_SYS__STDINT_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_BYTESWAP_H
#define _NANVIX_BYTESWAP_H

/**
 * @file byteswap.h
 * @brief Byte-order swapping macros.
 *
 * `bswap_16()`, `bswap_32()`, and `bswap_64()` reverse the byte order of a 16-,
 * 32-, or 64-bit integer. They map directly to the compiler's `__builtin_bswap*`
 * intrinsics.
 */

#define bswap_16(x) __builtin_bswap16(x)
#define bswap_32(x) __builtin_bswap32(x)
#define bswap_64(x) __builtin_bswap64(x)

#endif /* _NANVIX_BYTESWAP_H */

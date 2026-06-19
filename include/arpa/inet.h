/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_ARPA_INET_H
#define _NANVIX_ARPA_INET_H

/**
 * @file arpa/inet.h
 * @brief Internet address manipulation.
 *
 * Declares the byte-order conversion helpers and the textual IPv4 address
 * conversion interfaces. The byte-order helpers are provided as static inlines
 * (the target is little-endian, so they are simple byte swaps); the inet_*
 * prototypes are generated from the posix crate.
 */

#include <netinet/in.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Byte Order
 *==================================================================================================*/

/*
 * Host/network byte-order conversion. The Nanvix user target is little-endian,
 * so host-to-network (big-endian) conversions are byte swaps. Provided as static
 * inlines because the in-tree C library exposes no htonl/htons symbols, matching
 * the common C-library practice of defining these in the header.
 */
static inline uint16_t htons(uint16_t __hostshort)
{
    return __builtin_bswap16(__hostshort);
}

static inline uint32_t htonl(uint32_t __hostlong)
{
    return __builtin_bswap32(__hostlong);
}

static inline uint16_t ntohs(uint16_t __netshort)
{
    return __builtin_bswap16(__netshort);
}

static inline uint32_t ntohl(uint32_t __netlong)
{
    return __builtin_bswap32(__netlong);
}

/*==================================================================================================
 * Address Conversion
 *==================================================================================================*/

extern in_addr_t inet_addr(const char *cp);
extern char *inet_ntoa(struct in_addr in);
extern const char *inet_ntop(int af, const void *src, char *dst, socklen_t size);
extern int inet_pton(int af, const char *src, void *dst);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_ARPA_INET_H */

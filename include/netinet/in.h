/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_NETINET_IN_H
#define _NANVIX_NETINET_IN_H

/**
 * @file netinet/in.h
 * @brief Internet address family.
 *
 * Declares the IPv4 address types and structures and the IP protocol constants.
 * The types and layouts mirror the Rust definitions in the sysapi crate
 * (netinet_in.rs).
 */

#include <sys/socket.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define IPPROTO_IP 0 /**< Internet protocol. */
#define IPPROTO_ICMP 1 /**< Control message protocol. */
#define IPPROTO_TCP 6 /**< Transmission control protocol. */
#define IPPROTO_UDP 17 /**< User datagram protocol. */
#define IPPROTO_IPV6 41 /**< Internet protocol version 6. */
#define IPPROTO_RAW 255 /**< Raw IP packets protocol. */
#define INADDR_ANY ((in_addr_t)0x00000000) /**< Address to accept any incoming messages. */
#define INADDR_LOOPBACK ((in_addr_t)0x7f000001) /**< Loopback address (127.0.0.1). */
#define INADDR_BROADCAST ((in_addr_t)0xffffffff) /**< Address to send to all hosts. */
#define INADDR_NONE ((in_addr_t)0xffffffff) /**< Invalid address. */

#define INET_ADDRSTRLEN 16 /**< Max length of an IPv4 address string. */
#define INET6_ADDRSTRLEN 46 /**< Max length of an IPv6 address string. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Internet address (IPv4). */
typedef uint32_t in_addr_t;

/** @brief Internet port. */
typedef uint16_t in_port_t;

/** @brief Internet address structure. */
struct in_addr {
    in_addr_t s_addr; /**< IPv4 address. */
};

/** @brief Internet socket address (IPv4). */
struct sockaddr_in {
    unsigned char sin_len;   /**< Total length.   */
    sa_family_t sin_family;  /**< Address family. */
    in_port_t sin_port;      /**< Port number.    */
    struct in_addr sin_addr; /**< IPv4 address.   */
    unsigned char sin_zero[8]; /**< Padding.      */
};

/** @brief IPv6 address. */
struct in6_addr {
    union {
        uint8_t __u6_addr8[16];
        uint16_t __u6_addr16[8];
        uint32_t __u6_addr32[4];
    } __in6_union;
};
#define s6_addr __in6_union.__u6_addr8
#define s6_addr16 __in6_union.__u6_addr16
#define s6_addr32 __in6_union.__u6_addr32

/** @brief Internet socket address (IPv6). */
struct sockaddr_in6 {
    unsigned char sin6_len;     /**< Total length.        */
    sa_family_t sin6_family;    /**< Address family.      */
    in_port_t sin6_port;        /**< Port number.         */
    uint32_t sin6_flowinfo;     /**< Traffic class/flow.  */
    struct in6_addr sin6_addr;  /**< IPv6 address.        */
    uint32_t sin6_scope_id;     /**< Scope ID.            */
};

#define IN6ADDR_ANY_INIT                                                                           \
    {                                                                                              \
        {                                                                                          \
            {                                                                                      \
                0                                                                                  \
            }                                                                                      \
        }                                                                                          \
    }
#define IN6ADDR_LOOPBACK_INIT                                                                      \
    {                                                                                              \
        {                                                                                          \
            {                                                                                      \
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1                                     \
            }                                                                                      \
        }                                                                                          \
    }

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_NETINET_IN_H */

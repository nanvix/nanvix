/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_NETDB_H
#define _NANVIX_NETDB_H

/**
 * @file netdb.h
 * @brief Network database operations.
 *
 * The structure layouts mirror the Rust definitions in the sysapi crate
 * (netdb.rs).
 */

#include <netinet/in.h>
#include <sys/socket.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Structures
 *==================================================================================================*/

/** @brief Host database entry. */
struct hostent {
    char *h_name;        /**< Official host name.        */
    char **h_aliases;    /**< Alias list.                */
    int h_addrtype;      /**< Host address type.         */
    int h_length;        /**< Length of address.         */
    char **h_addr_list;  /**< List of addresses.         */
};
#define h_addr h_addr_list[0]

/** @brief Service database entry. */
struct servent {
    char *s_name;     /**< Official service name. */
    char **s_aliases; /**< Alias list.            */
    int s_port;       /**< Port number.           */
    char *s_proto;    /**< Protocol to use.       */
};

/** @brief Protocol database entry. */
struct protoent {
    char *p_name;     /**< Official protocol name. */
    char **p_aliases; /**< Alias list.             */
    int p_proto;      /**< Protocol number.        */
};

/** @brief Address information. */
struct addrinfo {
    int ai_flags;             /**< Input flags.                 */
    int ai_family;            /**< Address family.              */
    int ai_socktype;          /**< Socket type.                 */
    int ai_protocol;          /**< Protocol.                    */
    socklen_t ai_addrlen;     /**< Length of ai_addr.           */
    char *ai_canonname;       /**< Canonical name for the host. */
    struct sockaddr *ai_addr; /**< Socket address.              */
    struct addrinfo *ai_next; /**< Next structure in the list.  */
};

/*==================================================================================================
 * Flags and error codes
 *==================================================================================================*/

#define AI_PASSIVE 0x0001
#define AI_CANONNAME 0x0002
#define AI_NUMERICHOST 0x0004
#define AI_NUMERICSERV 0x0008
#define AI_V4MAPPED 0x0800
#define AI_ALL 0x0100
#define AI_ADDRCONFIG 0x0400

#define NI_NUMERICHOST 0x0001
#define NI_NUMERICSERV 0x0002
#define NI_NOFQDN 0x0004
#define NI_NAMEREQD 0x0008
#define NI_DGRAM 0x0010
#define NI_MAXHOST 1025
#define NI_MAXSERV 32

#define EAI_BADFLAGS (-1)
#define EAI_NONAME (-2)
#define EAI_AGAIN (-3)
#define EAI_FAIL (-4)
#define EAI_FAMILY (-6)
#define EAI_SOCKTYPE (-7)
#define EAI_SERVICE (-8)
#define EAI_MEMORY (-10)
#define EAI_SYSTEM (-11)
#define EAI_OVERFLOW (-12)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int getaddrinfo(
    const char *node, const char *service, const struct addrinfo *hints, struct addrinfo **res);
extern void freeaddrinfo(struct addrinfo *res);
extern const char *gai_strerror(int errcode);
extern int getnameinfo(
    const struct sockaddr *sa, socklen_t salen, char *host, socklen_t hostlen, char *serv,
    socklen_t servlen, int flags);
extern struct hostent *gethostbyname(const char *name);
extern struct servent *getservbyname(const char *name, const char *proto);

/** @brief Error code set by the legacy host-lookup functions. */
extern int h_errno;

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_NETDB_H */

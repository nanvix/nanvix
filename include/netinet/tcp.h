/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_NETINET_TCP_H
#define _NANVIX_NETINET_TCP_H

/**
 * @file netinet/tcp.h
 * @brief TCP protocol definitions.
 */

#ifdef __cplusplus
extern "C" {
#endif

#define TCP_NODELAY 1
#define TCP_MAXSEG 2
#define TCP_CORK 3
#define TCP_KEEPIDLE 4
#define TCP_KEEPINTVL 5
#define TCP_KEEPCNT 6

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_NETINET_TCP_H */

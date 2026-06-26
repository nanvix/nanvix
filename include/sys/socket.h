/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_SOCKET_H
#define _NANVIX_SYS_SOCKET_H

/**
 * @file sys/socket.h
 * @brief Main sockets header.
 *
 * Declares the socket address structures, the address-family and socket-type
 * constants, and the core socket interfaces. The constants and layouts mirror the
 * Rust definitions in the sysapi crate (sys_socket.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define AF_UNSPEC 0 /**< Unspecified address family. */
#define AF_UNIX 1 /**< UNIX domain sockets. */
#define AF_INET 2 /**< Internet domain sockets (IPv4). */
#define AF_INET6 10 /**< Internet domain sockets (IPv6). */
#define SOCK_STREAM 1 /**< Sequenced, reliable, connection-mode byte streams. */
#define SOCK_DGRAM 2 /**< Connectionless, unreliable datagrams. */
#define SOCK_RAW 3 /**< Raw protocol interface. */
#define SOCK_RDM 4 /**< Reliably-delivered messages. */
#define SOCK_SEQPACKET 5 /**< Sequenced, reliable, connection-mode records. */
#define SOL_SOCKET 0xffff /**< Options at the socket level. */
#define SO_REUSEADDR 0x0004 /**< Reuse of local addresses is supported. */
#define SO_KEEPALIVE 0x0008 /**< Connections are kept alive. */
#define SO_BROADCAST 0x0020 /**< Broadcast messages are supported. */
#define SO_LINGER 0x0080 /**< Socket lingers on close. */
#define SO_SNDBUF 0x1001 /**< Send buffer size. */
#define SO_RCVBUF 0x1002 /**< Receive buffer size. */
#define SO_ERROR 0x1007 /**< Socket error status. */
#define SO_TYPE 0x1008 /**< Socket type. */
#define SHUT_RD 0 /**< Disable further receives. */
#define SHUT_WR 1 /**< Disable further sends. */
#define MSG_OOB 0x0001 /**< Out-of-band data. */
#define MSG_PEEK 0x0002 /**< Leave received data in queue. */
#define MSG_DONTROUTE 0x0004 /**< Send without using routing tables. */
#define MSG_TRUNC 0x0020 /**< Normal data truncated. */
#define MSG_DONTWAIT 0x0040 /**< Nonblocking I/O. */
#define MSG_WAITALL 0x0100 /**< Wait for full request or error. */
#define MSG_NOSIGNAL 0x4000 /**< Do not generate SIGPIPE. */
#define SHUT_RDWR 2 /**< Disable further sends and receives. */
#define SOMAXCONN 128 /**< Maximum listen backlog. */
#define _SS_PADSIZE 14 /**< Size of the sockaddr_storage padding field. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Socket address family type. */
typedef unsigned char sa_family_t;

/** @brief Socket address length type. */
typedef unsigned int socklen_t;

/** @brief Generic socket address. */
struct sockaddr {
    unsigned char sa_len;    /**< Total length.   */
    sa_family_t sa_family;   /**< Address family. */
    char sa_data[14];        /**< Address data.   */
};

/** @brief Socket address storage, large enough for any address family. */
struct sockaddr_storage {
    unsigned char ss_len;    /**< Total length.   */
    sa_family_t ss_family;   /**< Address family. */
    char __ss_pad1[14];      /**< Padding.        */
};

/** @brief Linger option for SO_LINGER. */
struct linger {
    int l_onoff;  /**< Whether linger is enabled. */
    int l_linger; /**< Linger time, in seconds.   */
};

/*==================================================================================================
 * Sockets
 *==================================================================================================*/

extern int socket(int domain, int typ, int protocol);
extern int socketpair(int domain, int typ, int protocol, int *socket_fds);
extern int bind(int sockfd, const struct sockaddr *sockaddr, socklen_t len);
extern int listen(int sockfd, int backlog);
extern int accept(int sockfd, struct sockaddr *sockaddr, socklen_t *len);
extern int connect(int sockfd, const struct sockaddr *sockaddr, socklen_t len);
extern int getsockname(int sockfd, struct sockaddr *sockaddr, socklen_t *len);
extern int getpeername(int sockfd, struct sockaddr *sockaddr, socklen_t *len);
extern ssize_t send(int sockfd, const void *buf, size_t len, int flags);
extern ssize_t recv(int sockfd, void *buf, size_t len, int flags);
extern ssize_t sendto(int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *sockaddr, socklen_t addrlen);
extern ssize_t recvfrom(int sockfd, void *buf, size_t len, int flags, struct sockaddr *sockaddr, socklen_t *addrlen);
extern int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen);
extern int getsockopt(int sockfd, int level, int optname, void *optval, socklen_t *optlen);
extern int shutdown(int sockfd, int how);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_SOCKET_H */

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef COMMON_H_
#define COMMON_H_

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>

// Creates a pair of connected sockets.
extern void new_socket_pair(int domain, int type, int protocol, int sockfds[2]);

// Tests if we succeed to create a socket.
extern void test_create_socket(int domain, int type, int protocol);

// Tests if we succeed to bind a socket.
extern void test_bind_socket(int domain,
                             int type,
                             int protocol,
                             const struct sockaddr *addr,
                             socklen_t addrlen);

// Tests if we succeed to create a listening socket.
extern void test_listen_socket(int domain,
                               int type,
                               int protocol,
                               const struct sockaddr *addr,
                               socklen_t addrlen);

// Tests if we succeed to get the name of a bound socket.
extern void test_getsockname_bound_socket(int domain,
                                          int type,
                                          int protocol,
                                          const struct sockaddr *addr,
                                          socklen_t addrlen);

// Tests if we succeed to get the name of a listening socket.
extern void test_getsockname_listening_socket(int domain,
                                              int type,
                                              int protocol,
                                              const struct sockaddr *addr,
                                              socklen_t addrlen);

// Tests if we succeed to get the name of a socket.
extern void test_get_sockname(int domain,
                              int type,
                              int protocol,
                              const struct sockaddr *sockaddr,
                              socklen_t addrlen);

extern void test_unix_sockets(char sun_path[]);
extern void test_inet_sockets(in_port_t sin_port, struct in_addr sin_addr);

#endif

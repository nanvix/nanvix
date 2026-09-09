/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <arpa/inet.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

#ifndef NANVIX_HOST_WINDOWS
// Tests if we succeed to create a pair of connected sockets.
static void test_create_socket_pair(int domain, int type, int protocol)
{
    int sockfds[2];
    new_socket_pair(domain, type, protocol, sockfds);

    int ret = close(sockfds[0]);
    assert(ret == 0);

    ret = close(sockfds[1]);
    assert(ret == 0);
}

// Tests if we succeed to get the name of a pair of connected sockets.
static void test_getpeername(int domain, int type, int protocol)
{
    int sockfds[2] = {-1, -1};
    new_socket_pair(domain, type, protocol, sockfds);

    struct sockaddr sockaddr_self = {0};
    socklen_t addrlen_self = sizeof(sockaddr_self);
    int ret = getsockname(sockfds[0], &sockaddr_self, &addrlen_self);
    assert(ret == 0);

    struct sockaddr sockaddr_peer = {0};
    socklen_t addrlen_peer = sizeof(sockaddr_peer);
    ret = getpeername(sockfds[0], &sockaddr_peer, &addrlen_peer);
    assert(ret == 0);

    // Check if the address family and the port are what we expect.
    assert(memcmp(&sockaddr_self.sa_len, &sockaddr_peer.sa_len, sizeof(sockaddr_self.sa_len)) == 0);
    assert(memcmp(&sockaddr_self.sa_family,
                  &sockaddr_peer.sa_family,
                  sizeof(sockaddr_self.sa_family)) == 0);
    assert(memcmp(&sockaddr_self.sa_data, &sockaddr_peer.sa_data, sizeof(sockaddr_self.sa_data)) ==
           0);

    ret = close(sockfds[0]);
    assert(ret == 0);

    ret = close(sockfds[1]);
    assert(ret == 0);
}

// Tests if we succeed to send and receive data through a pair of connected sockets.
static void test_send_recv(int domain, int type, int protocol)
{
    int sockfds[2] = {-1, -1};
    new_socket_pair(domain, type, protocol, sockfds);

    const char *msg = "hello";
    int ret = send(sockfds[0], msg, strlen(msg), 0);
    assert(ret == (int)strlen(msg));

    char buf[6] = {0};
    ret = recv(sockfds[1], buf, 6, 0);
    assert(ret == (int)strlen(msg));
    assert(memcmp(msg, buf, strlen(msg)) == 0);

    ret = close(sockfds[0]);
    assert(ret == 0);

    ret = close(sockfds[1]);
    assert(ret == 0);
}

// Tests if we succeed to shutdown a pair of connected sockets.
static void test_shutdown(int domain, int type, int protocol)
{
    int sockfds[2] = {-1, -1};
    new_socket_pair(domain, type, protocol, sockfds);

    int ret = shutdown(sockfds[0], SHUT_RDWR);
    assert(ret == 0);

    ret = shutdown(sockfds[1], SHUT_RDWR);
    assert(ret == 0);

    ret = close(sockfds[0]);
    assert(ret == 0);

    ret = close(sockfds[1]);
    assert(ret == 0);
}
#endif

// Tests operations on pathname UNIX sockets.
void test_unix_pathname_sockets(const char sun_path[], const char unlink_path[])
{
    fprintf(stderr, "testing UNIX pathname sockets ... ");

    int domain = AF_UNIX;
    int type = SOCK_STREAM;
    int protocol = IPPROTO_IP;

    struct sockaddr_un sockaddr = {
        .sun_len = sizeof(struct sockaddr_un),
        .sun_family = domain,
    };
    strncpy(sockaddr.sun_path, sun_path, sizeof(sockaddr.sun_path) - 1);
    sockaddr.sun_path[sizeof(sockaddr.sun_path) - 1] = '\0';

    // Clean up any leftover socket file from a previous run.
    unlink(unlink_path);

    test_create_socket(domain, type, protocol);

    test_bind_socket(domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    assert(unlink(unlink_path) == 0);

    test_listen_socket(
        domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    assert(unlink(unlink_path) == 0);

    test_getsockname_bound_socket(
        domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    assert(unlink(unlink_path) == 0);

    test_getsockname_listening_socket(
        domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    assert(unlink(unlink_path) == 0);

    fprintf(stderr, "passed\n");
}

// Tests operations on unnamed UNIX socket pairs independently of hostfs.
void test_unix_socket_pairs(void)
{
#ifdef NANVIX_HOST_WINDOWS
    fprintf(stderr, "skipping UNIX socket pairs (socketpair is unsupported on Windows)\n");
#else
    fprintf(stderr, "testing UNIX socket pairs ... ");

    int domain = AF_UNIX;
    int type = SOCK_STREAM;
    int protocol = IPPROTO_IP;

    test_create_socket_pair(domain, type, protocol);
    test_getpeername(domain, type, protocol);
    test_send_recv(domain, type, protocol);
    test_shutdown(domain, type, protocol);

    fprintf(stderr, "passed\n");
#endif
}

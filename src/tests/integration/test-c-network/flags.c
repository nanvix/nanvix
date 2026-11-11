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
#include <errno.h>
#include <netinet/in.h>
#include <poll.h>
#include <stdbool.h>
#include <stdio.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// All socket creation flags.
#define ALL_SOCKET_FLAGS (SOCK_CLOEXEC | SOCK_CLOFORK | SOCK_NONBLOCK)

// A bit of the socket type that is neither part of a socket type nor a socket creation flag.
#define UNSUPPORTED_SOCKET_FLAG 0x01000000

//==================================================================================================
// Private Functions
//==================================================================================================

// Returns whether a child process that fork() creates inherits a descriptor.
static bool is_inherited(int fd)
{
    pid_t pid = fork();
    assert(pid >= 0);
    if (pid == 0) {
        struct stat st = {0};
        _exit((fstat(fd, &st) == 0) ? 0 : 1);
    }
    int status = 0;
    assert(waitpid(pid, &status, 0) == pid);
    assert(WIFEXITED(status));
    return (WEXITSTATUS(status) == 0);
}

// Tests socket() with socket creation flags.
static void test_socket(struct in_addr sin_addr)
{
    // A socket created with all socket creation flags is a socket of the requested type: a stream
    // socket that can listen for connections.
    int sockfd = socket(AF_INET, SOCK_STREAM | ALL_SOCKET_FLAGS, IPPROTO_TCP);
    assert(sockfd >= 0);
    struct sockaddr_in addr = {
        .sin_len = sizeof(addr),
        .sin_family = AF_INET,
        .sin_port = htons(0),
        .sin_addr = sin_addr,
    };
    assert(bind(sockfd, (const struct sockaddr *)&addr, sizeof(addr)) == 0);
    assert(listen(sockfd, 1) == 0);
    assert(close(sockfd) == 0);

    // Each socket creation flag is accepted on its own.
    const int flags[] = {SOCK_CLOEXEC, SOCK_CLOFORK, SOCK_NONBLOCK};
    for (size_t i = 0; i < sizeof(flags) / sizeof(flags[0]); i++) {
        sockfd = socket(AF_INET, SOCK_DGRAM | flags[i], IPPROTO_UDP);
        assert(sockfd >= 0);
        assert(close(sockfd) == 0);
    }

    // A child inherits a socket created without flags, and a close-on-exec socket, but not a
    // close-on-fork socket, which the parent keeps.
    sockfd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    assert(sockfd >= 0);
    assert(is_inherited(sockfd));
    assert(close(sockfd) == 0);
    sockfd = socket(AF_INET, SOCK_DGRAM | SOCK_CLOEXEC, IPPROTO_UDP);
    assert(sockfd >= 0);
    assert(is_inherited(sockfd));
    assert(close(sockfd) == 0);
    sockfd = socket(AF_INET, SOCK_DGRAM | SOCK_CLOFORK, IPPROTO_UDP);
    assert(sockfd >= 0);
    assert(!is_inherited(sockfd));
    struct stat st = {0};
    assert(fstat(sockfd, &st) == 0);
    assert(S_ISSOCK(st.st_mode));
    assert(close(sockfd) == 0);

    // A bit that is neither part of the socket type nor a socket creation flag is rejected with
    // EINVAL, so that callers can fall back to creating the socket without it.
    errno = 0;
    assert(socket(AF_INET, SOCK_STREAM | UNSUPPORTED_SOCKET_FLAG, IPPROTO_TCP) == -1);
    assert(errno == EINVAL);
}

// Tests socketpair() with socket creation flags.
static void test_socketpair(void)
{
    int sockfds[2] = {-1, -1};

#ifndef NANVIX_HOST_WINDOWS
    // Both sockets of a pair created with all socket creation flags are connected, and a child
    // inherits neither of them.
    assert(socketpair(AF_UNIX, SOCK_STREAM | ALL_SOCKET_FLAGS, 0, sockfds) == 0);
    const char byte = 'f';
    assert(send(sockfds[0], &byte, 1, 0) == 1);
    struct pollfd pfd = {
        .fd = sockfds[1],
        .events = POLLIN,
        .revents = 0,
    };
    assert(poll(&pfd, 1, 1000) == 1);
    char received = 0;
    assert(recv(sockfds[1], &received, 1, 0) == 1);
    assert(received == byte);
    assert(!is_inherited(sockfds[0]));
    assert(!is_inherited(sockfds[1]));
    assert(close(sockfds[0]) == 0);
    assert(close(sockfds[1]) == 0);
#endif

    // Unsupported flags are rejected before a pair is created, so this also holds on hosts that do
    // not support socketpair().
    errno = 0;
    assert(socketpair(AF_UNIX, SOCK_STREAM | UNSUPPORTED_SOCKET_FLAG, 0, sockfds) == -1);
    assert(errno == EINVAL);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests socket creation flags.
void test_socket_flags(struct in_addr sin_addr)
{
    fprintf(stderr, "testing socket creation flags ... ");

    test_socket(sin_addr);
    test_socketpair();

    fprintf(stderr, "passed\n");
}

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Timeout for poll() system calls that wait for readiness.
#define POLL_TIMEOUT 1000

// File used to exercise hostfsd-backed descriptors.
#define POLL_HOSTFS_FILE "/mnt/poll-network-file.tmp"

//==================================================================================================
// Imported Symbols
//==================================================================================================

extern int mount(
    const char *source,
    const char *target,
    const char *filesystemtype,
    unsigned long mountflags);

//==================================================================================================
// Private Functions
//==================================================================================================

// Creates a datagram socket bound to an ephemeral port on the supplied address.
static int new_bound_dgram_socket(struct in_addr addr, struct sockaddr_in *assigned)
{
    int sockfd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    assert(sockfd >= 0);

    struct sockaddr_in bindaddr = {
        .sin_len = sizeof(bindaddr),
        .sin_family = AF_INET,
        .sin_port = htons(0),
        .sin_addr = addr,
    };
    assert(bind(sockfd, (const struct sockaddr *)&bindaddr, sizeof(bindaddr)) == 0);

    socklen_t assigned_len = sizeof(*assigned);
    assert(getsockname(sockfd, (struct sockaddr *)assigned, &assigned_len) == 0);
    assert(assigned_len == sizeof(*assigned));
    assert(assigned->sin_family == AF_INET);

    return sockfd;
}

// Sends one byte to a datagram socket's own address.
static void make_socket_readable(int sockfd, const struct sockaddr_in *self)
{
    const char byte = 's';
    assert(sendto(sockfd,
                  &byte,
                  sizeof(byte),
                  0,
                  (const struct sockaddr *)self,
                  sizeof(*self)) == sizeof(byte));
}

// Tests poll() readiness for a hostfsd-backed regular file.
static void test_poll_hostfsd(void)
{
    int fd = open(POLL_HOSTFS_FILE, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);

    struct pollfd pfd = {
        .fd = fd,
        .events = POLLIN | POLLOUT,
        .revents = 0,
    };
    assert(poll(&pfd, 1, 0) == 1);
    assert(pfd.revents == (POLLIN | POLLOUT));

    assert(close(fd) == 0);
    assert(unlink(POLL_HOSTFS_FILE) == 0);
}

// Tests poll() readiness for a networkd-backed datagram socket.
static void test_poll_networkd(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);
    struct pollfd pfd = {
        .fd = sockfd,
        .events = POLLIN | POLLOUT,
        .revents = 0,
    };

    assert(poll(&pfd, 1, 0) == 1);
    assert((pfd.revents & POLLOUT) != 0);
    assert((pfd.revents & POLLIN) == 0);

    make_socket_readable(sockfd, &self);
    pfd.events = POLLIN;
    pfd.revents = 0;
    assert(poll(&pfd, 1, POLL_TIMEOUT) == 1);
    assert(pfd.revents == POLLIN);

    char byte;
    assert(recvfrom(sockfd, &byte, sizeof(byte), 0, NULL, NULL) == sizeof(byte));
    assert(byte == 's');
    assert(close(sockfd) == 0);
}

// Tests poll() readiness for a VFSD-backed pipe.
static void test_poll_vfsd(void)
{
    int pipefds[2];
    assert(pipe(pipefds) == 0);

    struct pollfd pfds[2] = {
        {.fd = pipefds[0], .events = POLLIN, .revents = (short)-1},
        {.fd = pipefds[1], .events = POLLOUT, .revents = (short)-1},
    };
    assert(poll(pfds, 2, 0) == 1);
    assert(pfds[0].revents == 0);
    assert(pfds[1].revents == POLLOUT);

    const char byte = 'p';
    assert(write(pipefds[1], &byte, sizeof(byte)) == sizeof(byte));
    pfds[0].revents = 0;
    pfds[1].events = 0;
    pfds[1].revents = 0;
    assert(poll(pfds, 2, POLL_TIMEOUT) == 1);
    assert(pfds[0].revents == POLLIN);
    assert(pfds[1].revents == 0);

    char received;
    assert(read(pipefds[0], &received, sizeof(received)) == sizeof(received));
    assert(received == byte);
    assert(close(pipefds[0]) == 0);
    assert(close(pipefds[1]) == 0);
}

// Tests one poll() request containing hostfsd-, networkd-, and VFSD-backed descriptors.
static void test_poll_mixed(struct in_addr sin_addr)
{
    int hostfd = open(POLL_HOSTFS_FILE, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(hostfd >= 0);

    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    int pipefds[2];
    assert(pipe(pipefds) == 0);

    struct pollfd pfds[3] = {
        {.fd = hostfd, .events = POLLIN | POLLOUT, .revents = (short)-1},
        {.fd = sockfd, .events = POLLIN, .revents = (short)-1},
        {.fd = pipefds[0], .events = POLLIN, .revents = (short)-1},
    };
    assert(poll(pfds, 3, 0) == 1);
    assert(pfds[0].revents == (POLLIN | POLLOUT));
    assert(pfds[1].revents == 0);
    assert(pfds[2].revents == 0);

    make_socket_readable(sockfd, &self);
    const char pipe_byte = 'p';
    assert(write(pipefds[1], &pipe_byte, sizeof(pipe_byte)) == sizeof(pipe_byte));

    for (size_t i = 0; i < 3; i++) {
        pfds[i].revents = 0;
    }
    assert(poll(pfds, 3, POLL_TIMEOUT) == 3);
    assert(pfds[0].revents == (POLLIN | POLLOUT));
    assert(pfds[1].revents == POLLIN);
    assert(pfds[2].revents == POLLIN);

    char socket_byte;
    assert(recvfrom(sockfd, &socket_byte, sizeof(socket_byte), 0, NULL, NULL) ==
           sizeof(socket_byte));
    assert(socket_byte == 's');
    char received_pipe_byte;
    assert(read(pipefds[0], &received_pipe_byte, sizeof(received_pipe_byte)) ==
           sizeof(received_pipe_byte));
    assert(received_pipe_byte == pipe_byte);

    assert(close(hostfd) == 0);
    assert(unlink(POLL_HOSTFS_FILE) == 0);
    assert(close(sockfd) == 0);
    assert(close(pipefds[0]) == 0);
    assert(close(pipefds[1]) == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests poll() routing through hostfsd, networkd, VFSD, and all three services together.
void test_poll_services(struct in_addr sin_addr)
{
    fprintf(stderr, "testing poll() with networkd ... ");
    test_poll_networkd(sin_addr);
    fprintf(stderr, "passed\n");

    fprintf(stderr, "testing poll() with vfsd ... ");
    test_poll_vfsd();
    fprintf(stderr, "passed\n");

    if (getenv("NANVIX_TEST_HOSTFS") == NULL) {
        return;
    }

    assert(mount("", "/mnt", "hostfs", 0) == 0);

    fprintf(stderr, "testing poll() with hostfsd ... ");
    test_poll_hostfsd();
    fprintf(stderr, "passed\n");

    fprintf(stderr, "testing poll() with mixed services ... ");
    test_poll_mixed(sin_addr);
    fprintf(stderr, "passed\n");
}

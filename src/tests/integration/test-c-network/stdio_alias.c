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
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests that /dev/stdin follows standard input to a socket and keeps its networkd routing.
void test_socket_stdio_alias(struct in_addr sin_addr)
{
    fprintf(stderr, "testing /dev/stdin on a socket ... ");

    // A datagram socket connected to its own address receives what it sends.
    int sockfd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    assert(sockfd >= 0);
    struct sockaddr_in self = {
        .sin_len = sizeof(self),
        .sin_family = AF_INET,
        .sin_port = htons(0),
        .sin_addr = sin_addr,
    };
    assert(bind(sockfd, (const struct sockaddr *)&self, sizeof(self)) == 0);
    socklen_t self_len = sizeof(self);
    assert(getsockname(sockfd, (struct sockaddr *)&self, &self_len) == 0);
    assert(connect(sockfd, (const struct sockaddr *)&self, sizeof(self)) == 0);

    int saved = dup(STDIN_FILENO);
    assert(saved >= 0);
    assert(dup2(sockfd, STDIN_FILENO) == STDIN_FILENO);
    int alias = open("/dev/stdin", O_RDWR);
    assert(alias >= 0);

    // The alias and its path report the socket.
    struct stat sock_st = {0};
    struct stat alias_st = {0};
    struct stat path_st = {0};
    assert(fstat(sockfd, &sock_st) == 0);
    assert(fstat(alias, &alias_st) == 0);
    assert(stat("/dev/stdin", &path_st) == 0);
    assert(S_ISSOCK(alias_st.st_mode));
    assert(S_ISSOCK(path_st.st_mode));
    assert((alias_st.st_dev == sock_st.st_dev) && (alias_st.st_ino == sock_st.st_ino));
    assert((path_st.st_dev == sock_st.st_dev) && (path_st.st_ino == sock_st.st_ino));
    struct sockaddr_in name = {0};
    socklen_t name_len = sizeof(name);
    assert(getsockname(alias, (struct sockaddr *)&name, &name_len) == 0);
    assert(name.sin_port == self.sin_port);

    // Data flows through the alias in both directions, and keeps flowing once the descriptors that
    // it was opened through are closed.
    char buffer[8] = {0};
    assert(send(alias, "alias", 5, 0) == 5);
    assert(recv(sockfd, buffer, sizeof(buffer), 0) == 5);
    assert(memcmp(buffer, "alias", 5) == 0);
    assert(send(sockfd, "socket", 6, 0) == 6);
    assert(recv(alias, buffer, sizeof(buffer), 0) == 6);
    assert(memcmp(buffer, "socket", 6) == 0);
    assert(dup2(saved, STDIN_FILENO) == STDIN_FILENO);
    assert(close(saved) == 0);
    assert(close(sockfd) == 0);
    assert(send(alias, "kept", 4, 0) == 4);
    assert(recv(alias, buffer, sizeof(buffer), 0) == 4);
    assert(memcmp(buffer, "kept", 4) == 0);
    assert(close(alias) == 0);

    fprintf(stderr, "passed\n");
}

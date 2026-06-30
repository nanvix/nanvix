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
#include <netinet/in.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Creates a datagram socket bound to an ephemeral port on the supplied address and reports the
// address that the system assigned to it.
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
    int ret = bind(sockfd, (const struct sockaddr *)&bindaddr, sizeof(bindaddr));
    assert(ret == 0);

    // Learn the address/port that the system assigned to the socket.
    socklen_t assigned_len = sizeof(*assigned);
    ret = getsockname(sockfd, (struct sockaddr *)assigned, &assigned_len);
    assert(ret == 0);
    assert(assigned_len == sizeof(*assigned));
    assert(assigned->sin_family == AF_INET);

    return sockfd;
}

// Tests datagram `sendto()`/`recvfrom()` with an explicit destination address. A datagram is sent
// to the socket's own address and received back, validating both the payload and the reported
// source address.
static void test_inet_dgram_sendto_recvfrom(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Send a datagram to our own address.
    const char message[] = "hello";
    const size_t message_len = sizeof(message) - 1;
    ssize_t sent =
        sendto(sockfd, message, message_len, 0, (const struct sockaddr *)&self, sizeof(self));
    assert(sent == (ssize_t)message_len);

    // Receive the datagram and capture the source address.
    char buffer[16];
    memset(buffer, 0, sizeof(buffer));
    struct sockaddr_in source;
    memset(&source, 0, sizeof(source));
    socklen_t source_len = sizeof(source);
    ssize_t received =
        recvfrom(sockfd, buffer, sizeof(buffer), 0, (struct sockaddr *)&source, &source_len);
    assert(received == (ssize_t)message_len);
    assert(memcmp(buffer, message, message_len) == 0);

    // The reported source must match the socket's own address (loopback self-send).
    assert(source.sin_family == AF_INET);
    assert(source.sin_addr.s_addr == self.sin_addr.s_addr);
    assert(source.sin_port == self.sin_port);

    int ret = close(sockfd);
    assert(ret == 0);
}

// Tests datagram `sendto()`/`recvfrom()` with a NULL address on a connected socket. When no address
// is supplied, the calls must behave like `send()`/`recv()`.
static void test_inet_dgram_null_address(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Connect the datagram socket to its own address so a NULL destination is valid.
    int ret = connect(sockfd, (const struct sockaddr *)&self, sizeof(self));
    assert(ret == 0);

    // With a NULL address, `sendto()` behaves like `send()`.
    const char message[] = "world";
    const size_t message_len = sizeof(message) - 1;
    ssize_t sent = sendto(sockfd, message, message_len, 0, NULL, 0);
    assert(sent == (ssize_t)message_len);

    // With a NULL address, `recvfrom()` behaves like `recv()`.
    char buffer[16];
    memset(buffer, 0, sizeof(buffer));
    ssize_t received = recvfrom(sockfd, buffer, sizeof(buffer), 0, NULL, NULL);
    assert(received == (ssize_t)message_len);
    assert(memcmp(buffer, message, message_len) == 0);

    ret = close(sockfd);
    assert(ret == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests operations in INET sockets.
void test_inet_sockets(in_port_t sin_port, struct in_addr sin_addr)
{
    // Test configuration.
    int domain = AF_INET;
    int type = SOCK_STREAM;
    int protocol = IPPROTO_TCP;

    struct sockaddr_in sockaddr = {
        .sin_len = sizeof(sockaddr),
        .sin_family = domain,
        .sin_port = sin_port,
        .sin_addr = sin_addr,
    };

    test_create_socket(domain, type, protocol);
    test_bind_socket(domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    test_listen_socket(
        domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));
    test_get_sockname(domain, type, protocol, (const struct sockaddr *)&sockaddr, sizeof(sockaddr));

    // Exercise datagram sendto()/recvfrom() over loopback.
    test_inet_dgram_sendto_recvfrom(sin_addr);
    test_inet_dgram_null_address(sin_addr);
}

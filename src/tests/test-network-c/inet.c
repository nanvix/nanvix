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
#include <unistd.h>

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
}

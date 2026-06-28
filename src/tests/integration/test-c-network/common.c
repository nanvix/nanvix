/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <arpa/inet.h>
#include <assert.h>
#include <netinet/in.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Creates a new unbound socket.
static int new_unbound_socket(int domain, int type, int protocol)
{
    int sockfd = socket(domain, type, protocol);
    assert(sockfd >= 0);
    return sockfd;
}

// Creates a new bound socket.
static int new_bound_socket(int domain,
                            int type,
                            int protocol,
                            const struct sockaddr *addr,
                            socklen_t addrlen)
{
    int sockfd = new_unbound_socket(domain, type, protocol);
    int ret = bind(sockfd, addr, addrlen);
    assert(ret == 0);
    return sockfd;
}

// Creates a listening socket.
static int new_listening_socket(int domain,
                                int type,
                                int protocol,
                                const struct sockaddr *addr,
                                socklen_t addrlen)
{
    int sockfd = new_bound_socket(domain, type, protocol, addr, addrlen);
    int ret = listen(sockfd, 5);
    assert(ret == 0);
    return sockfd;
}

// Creates a pair of connected sockets.
void new_socket_pair(int domain, int type, int protocol, int sockfds[2])
{
    int ret = socketpair(domain, type, protocol, sockfds);
    assert(ret == 0);
}

// Tests if we succeed to create a socket.
void test_create_socket(int domain, int type, int protocol)
{
    int sockfd = new_unbound_socket(domain, type, protocol);

    int ret = close(sockfd);
    assert(ret == 0);
}

// Tests if we succeed to bind a socket.
void test_bind_socket(int domain,
                      int type,
                      int protocol,
                      const struct sockaddr *addr,
                      socklen_t addrlen)
{
    int sockfd = new_bound_socket(domain, type, protocol, addr, addrlen);

    int ret = close(sockfd);
    assert(ret == 0);

    if (addr->sa_family == AF_UNIX) {
        ret = unlink(((struct sockaddr_un *)addr)->sun_path);
        assert(ret == 0);
    }
}

// Tests if we succeed to create a listening socket.
void test_listen_socket(int domain,
                        int type,
                        int protocol,
                        const struct sockaddr *addr,
                        socklen_t addrlen)
{
    int sockfd = new_listening_socket(domain, type, protocol, addr, addrlen);

    int ret = close(sockfd);
    assert(ret == 0);

    if (addr->sa_family == AF_UNIX) {
        ret = unlink(((struct sockaddr_un *)addr)->sun_path);
        assert(ret == 0);
    }
}

// Tests if we succeed to get the name of a bound socket.
void test_getsockname_bound_socket(int domain,
                                   int type,
                                   int protocol,
                                   const struct sockaddr *addr,
                                   socklen_t addrlen)
{
    int sockfd = new_bound_socket(domain, type, protocol, addr, addrlen);

    struct sockaddr addrbuf;
    socklen_t addrlenbuf = sizeof(addrbuf);

    int ret = getsockname(sockfd, &addrbuf, &addrlenbuf);
    assert(ret == 0);

    // Check if the address family and the port are what we expect.
    assert(memcmp(&addr->sa_len, &addrbuf.sa_len, sizeof(addr->sa_len)) == 0);
    assert(memcmp(&addr->sa_family, &addrbuf.sa_family, sizeof(addr->sa_family)) == 0);
    assert(memcmp(&addr->sa_data, &addrbuf.sa_data, sizeof(addr->sa_data)) == 0);

    ret = close(sockfd);
    assert(ret == 0);

    if (addr->sa_family == AF_UNIX) {
        ret = unlink(((struct sockaddr_un *)addr)->sun_path);
        assert(ret == 0);
    }
}

// Tests if we succeed to get the name of a listening socket.
void test_getsockname_listening_socket(int domain,
                                       int type,
                                       int protocol,
                                       const struct sockaddr *addr,
                                       socklen_t addrlen)
{
    int sockfd = new_listening_socket(domain, type, protocol, addr, addrlen);

    struct sockaddr addrbuf;
    socklen_t addrlenbuf = sizeof(addrbuf);

    int ret = getsockname(sockfd, &addrbuf, &addrlenbuf);
    assert(ret == 0);

    // Check if the address family and the port are what we expect.
    assert(memcmp(&addr->sa_len, &addrbuf.sa_len, sizeof(addr->sa_len)) == 0);
    assert(memcmp(&addr->sa_family, &addrbuf.sa_family, sizeof(addr->sa_family)) == 0);
    assert(memcmp(&addr->sa_data, &addrbuf.sa_data, sizeof(addr->sa_data)) == 0);

    ret = close(sockfd);
    assert(ret == 0);

    if (addr->sa_family == AF_UNIX) {
        ret = unlink(((struct sockaddr_un *)addr)->sun_path);
        assert(ret == 0);
    }
}

// Tests if we succeed to get the name of a socket.
void test_get_sockname(int domain,
                       int type,
                       int protocol,
                       const struct sockaddr *sockaddr,
                       socklen_t addrlen)
{
    test_getsockname_bound_socket(domain, type, protocol, sockaddr, addrlen);
    test_getsockname_listening_socket(domain, type, protocol, sockaddr, addrlen);
}

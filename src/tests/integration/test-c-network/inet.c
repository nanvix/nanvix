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
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/uio.h>
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

// Tests `send()`/`recv()` on a connected datagram socket over loopback. A message is sent to the
// socket's own address and received back through the connected endpoint. This exercises the
// pull-based payload transfer used by `recv()`, which delivers the payload out-of-band directly
// into the user buffer rather than inline in the response message.
static void test_inet_dgram_send_recv(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Connect the datagram socket to its own address so `send()`/`recv()` operate on it.
    int ret = connect(sockfd, (const struct sockaddr *)&self, sizeof(self));
    assert(ret == 0);

    // Send a message through the connected socket.
    const char message[] = "recv-pull";
    const size_t message_len = sizeof(message) - 1;
    ssize_t sent = send(sockfd, message, message_len, 0);
    assert(sent == (ssize_t)message_len);

    // Receive the message back through the connected socket. The receive buffer is larger than the
    // payload, so `recv()` must report exactly the number of bytes delivered by the pull.
    char buffer[16];
    memset(buffer, 0, sizeof(buffer));
    ssize_t received = recv(sockfd, buffer, sizeof(buffer), 0);
    assert(received == (ssize_t)message_len);
    assert(memcmp(buffer, message, message_len) == 0);

    ret = close(sockfd);
    assert(ret == 0);
}

// Tests that `recv()` can deliver a payload larger than a single page in one call. A multi-page
// datagram is sent with `sendto()` -- which transfers the whole datagram atomically through a
// single scatter/gather push -- to the socket's own address, then received back with one `recv()`.
// This exercises the multi-page scatter/gather pull that the raised `MAX_DATA_SIZE` enables: before
// the limit was lifted every transfer was capped at one page, so `recv()` could not return this
// many bytes in a single call.
static void test_inet_dgram_recv_large(struct in_addr sin_addr)
{
    // A three-page payload: comfortably above the old single-page limit, and well within both the
    // scatter/gather ceiling and the socket receive buffer.
    enum { ONE_PAGE = 4096, PAYLOAD_SIZE = 3 * ONE_PAGE };
    static unsigned char sndbuf[PAYLOAD_SIZE];
    static unsigned char rcvbuf[PAYLOAD_SIZE + ONE_PAGE];

    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Fill the payload with a position-dependent pattern so truncation or reordering is detectable.
    for (size_t i = 0; i < PAYLOAD_SIZE; i++) {
        sndbuf[i] = (unsigned char)((i * 31u + 7u) & 0xFFu);
    }

    // `sendto()` delivers the whole datagram in a single push, so the peer receives one
    // PAYLOAD_SIZE-byte datagram rather than page-sized fragments.
    ssize_t sent =
        sendto(sockfd, sndbuf, PAYLOAD_SIZE, 0, (const struct sockaddr *)&self, sizeof(self));
    assert(sent == (ssize_t)PAYLOAD_SIZE);

    // A single `recv()` must return the entire multi-page datagram. The receive buffer is larger
    // than the datagram, so the reported count reflects the bytes actually delivered by the pull.
    memset(rcvbuf, 0, sizeof(rcvbuf));
    ssize_t received = recv(sockfd, rcvbuf, sizeof(rcvbuf), 0);
    assert(received == (ssize_t)PAYLOAD_SIZE);
    assert(received > (ssize_t)ONE_PAGE);
    assert(memcmp(rcvbuf, sndbuf, PAYLOAD_SIZE) == 0);

    int ret = close(sockfd);
    assert(ret == 0);
}

// Tests scatter-gather `sendmsg()`/`recvmsg()` with an explicit destination address. A message
// split across multiple iovecs is sent to the socket's own address and received back into a
// different set of iovecs, validating payload reassembly and the reported source address.
static void test_inet_dgram_sendmsg_recvmsg(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Assemble the outgoing message from three separate buffers.
    const char part0[] = "hello, ";
    const char part1[] = "scatter-";
    const char part2[] = "gather";
    const size_t total_len = (sizeof(part0) - 1) + (sizeof(part1) - 1) + (sizeof(part2) - 1);
    struct iovec send_iov[3] = {
        {.iov_base = (void *)part0, .iov_len = sizeof(part0) - 1},
        {.iov_base = (void *)part1, .iov_len = sizeof(part1) - 1},
        {.iov_base = (void *)part2, .iov_len = sizeof(part2) - 1},
    };
    struct msghdr send_msg;
    memset(&send_msg, 0, sizeof(send_msg));
    send_msg.msg_name = &self;
    send_msg.msg_namelen = sizeof(self);
    send_msg.msg_iov = send_iov;
    send_msg.msg_iovlen = 3;

    ssize_t sent = sendmsg(sockfd, &send_msg, 0);
    assert(sent == (ssize_t)total_len);

    // Receive the datagram back, scattering it across two buffers and capturing the source address.
    char head[8];
    char tail[32];
    memset(head, 0, sizeof(head));
    memset(tail, 0, sizeof(tail));
    struct iovec recv_iov[2] = {
        {.iov_base = head, .iov_len = sizeof(head)},
        {.iov_base = tail, .iov_len = sizeof(tail)},
    };
    struct sockaddr_in source;
    memset(&source, 0, sizeof(source));
    struct msghdr recv_msg;
    memset(&recv_msg, 0, sizeof(recv_msg));
    recv_msg.msg_name = &source;
    recv_msg.msg_namelen = sizeof(source);
    recv_msg.msg_iov = recv_iov;
    recv_msg.msg_iovlen = 2;

    ssize_t received = recvmsg(sockfd, &recv_msg, 0);
    assert(received == (ssize_t)total_len);

    // Reassemble the payload from the scatter buffers and validate it.
    const char expected[] = "hello, scatter-gather";
    char assembled[64];
    memset(assembled, 0, sizeof(assembled));
    memcpy(assembled, head, sizeof(head));
    memcpy(assembled + sizeof(head), tail, total_len - sizeof(head));
    assert(memcmp(assembled, expected, total_len) == 0);

    // The message header must report the source address of the datagram (loopback self-send).
    assert(recv_msg.msg_namelen == sizeof(source));
    assert(source.sin_family == AF_INET);
    assert(source.sin_addr.s_addr == self.sin_addr.s_addr);
    assert(source.sin_port == self.sin_port);

    // No ancillary data was delivered.
    assert(recv_msg.msg_controllen == 0);

    int ret = close(sockfd);
    assert(ret == 0);
}

// Tests scatter-gather `sendmsg()`/`recvmsg()` with a NULL address on a connected socket. When no
// address is supplied, the calls must behave like `send()`/`recv()` while still honoring the
// scatter-gather buffers.
static void test_inet_dgram_sendmsg_recvmsg_connected(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    // Connect the datagram socket to its own address so a NULL destination is valid.
    int ret = connect(sockfd, (const struct sockaddr *)&self, sizeof(self));
    assert(ret == 0);

    // With a NULL address, `sendmsg()` behaves like `send()`.
    const char part0[] = "msg-";
    const char part1[] = "connected";
    const size_t total_len = (sizeof(part0) - 1) + (sizeof(part1) - 1);
    struct iovec send_iov[2] = {
        {.iov_base = (void *)part0, .iov_len = sizeof(part0) - 1},
        {.iov_base = (void *)part1, .iov_len = sizeof(part1) - 1},
    };
    struct msghdr send_msg;
    memset(&send_msg, 0, sizeof(send_msg));
    send_msg.msg_iov = send_iov;
    send_msg.msg_iovlen = 2;

    ssize_t sent = sendmsg(sockfd, &send_msg, 0);
    assert(sent == (ssize_t)total_len);

    // With a NULL address, `recvmsg()` behaves like `recv()`.
    char buffer[32];
    memset(buffer, 0, sizeof(buffer));
    struct iovec recv_iov[1] = {
        {.iov_base = buffer, .iov_len = sizeof(buffer)},
    };
    struct msghdr recv_msg;
    memset(&recv_msg, 0, sizeof(recv_msg));
    recv_msg.msg_iov = recv_iov;
    recv_msg.msg_iovlen = 1;

    ssize_t received = recvmsg(sockfd, &recv_msg, 0);
    assert(received == (ssize_t)total_len);
    assert(memcmp(buffer, "msg-connected", total_len) == 0);

    ret = close(sockfd);
    assert(ret == 0);
}

// Tests that `sendmsg()` does not silently ignore ancillary data. Control messages are not
// supported yet, so a caller that provides a control buffer must receive an explicit error.
static void test_inet_dgram_sendmsg_control_unsupported(struct in_addr sin_addr)
{
    struct sockaddr_in self;
    int sockfd = new_bound_dgram_socket(sin_addr, &self);

    const char message[] = "control";
    struct iovec send_iov[1] = {
        {.iov_base = (void *)message, .iov_len = sizeof(message) - 1},
    };
    char control[sizeof(int)];
    struct msghdr send_msg;
    memset(&send_msg, 0, sizeof(send_msg));
    send_msg.msg_name = &self;
    send_msg.msg_namelen = sizeof(self);
    send_msg.msg_iov = send_iov;
    send_msg.msg_iovlen = 1;
    send_msg.msg_control = control;
    send_msg.msg_controllen = sizeof(control);

    errno = 0;
    ssize_t sent = sendmsg(sockfd, &send_msg, 0);
    assert(sent == -1);
    assert(errno == EOPNOTSUPP);

    int ret = close(sockfd);
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

    // Exercise connected-datagram send()/recv() over loopback.
    test_inet_dgram_send_recv(sin_addr);

    // Exercise a multi-page recv() that returns more than one page in a single call.
    test_inet_dgram_recv_large(sin_addr);

    // Exercise scatter-gather sendmsg()/recvmsg() over loopback.
    test_inet_dgram_sendmsg_recvmsg(sin_addr);
    test_inet_dgram_sendmsg_recvmsg_connected(sin_addr);
    test_inet_dgram_sendmsg_control_unsupported(sin_addr);
}

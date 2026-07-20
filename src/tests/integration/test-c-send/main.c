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
#include <stddef.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// IPv4 loopback address (127.0.0.1) in host byte order.
#define LOOPBACK_ADDR 0x7f000001u

// Payload length exercised by the push-based transfer. It is deliberately far larger than the
// number of bytes that once fit inline in a single IPC message (the previous chunked `send()` path
// carried only a couple dozen bytes per round trip) and stays below both a memory page and the
// socket buffer, so a single-threaded send/recv round trip cannot deadlock.
#define PAYLOAD_LEN 1000

// Number of bytes that fit in one page-bounded push.
#define PAGE_BYTES 4096u

// Payload length that spans several page-bounded send() calls while staying comfortably below a
// loopback socket's buffer pipeline, so a single-threaded send-then-receive of the whole payload
// never blocks and therefore cannot deadlock.
#define LARGE_PAYLOAD_LEN (16u * 1024u)

//==================================================================================================
// Private Functions
//==================================================================================================

// Fills a buffer with a deterministic, position-dependent pattern.
static void fill_pattern(char *buffer, size_t len)
{
    for (size_t i = 0; i < len; i++) {
        buffer[i] = (char)((i * 31u + 7u) & 0xffu);
    }
}

// Sends the whole buffer, resubmitting the remainder on a short send. A stream `send()` may
// transfer fewer bytes than requested, so the caller loops until everything is written.
static void send_all(int sockfd, const char *buffer, size_t len)
{
    size_t offset = 0;
    while (offset < len) {
        ssize_t sent = send(sockfd, buffer + offset, len - offset, 0);
        assert(sent > 0);
        offset += (size_t)sent;
    }
    assert(offset == len);
}

// Receives exactly `len` bytes, looping until the whole payload has been drained.
static void recv_all(int sockfd, char *buffer, size_t len)
{
    size_t offset = 0;
    while (offset < len) {
        ssize_t received = recv(sockfd, buffer + offset, len - offset, 0);
        assert(received > 0);
        offset += (size_t)received;
    }
    assert(offset == len);
}

// Establishes a pair of connected stream sockets over the IPv4 loopback interface. A listening
// socket is bound to an ephemeral port, a client connects to it, and the accepted endpoint is
// returned alongside the client endpoint.
static void connect_loopback_pair(int *client_out, int *accepted_out)
{
    int server = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    assert(server >= 0);

    struct sockaddr_in bind_addr = {
        .sin_len = sizeof(bind_addr),
        .sin_family = AF_INET,
        .sin_port = htons(0),
        .sin_addr = {.s_addr = htonl(LOOPBACK_ADDR)},
    };
    assert(bind(server, (const struct sockaddr *)&bind_addr, sizeof(bind_addr)) == 0);
    assert(listen(server, 1) == 0);

    // Learn the ephemeral port the system assigned to the listening socket.
    struct sockaddr_in listen_addr;
    memset(&listen_addr, 0, sizeof(listen_addr));
    socklen_t listen_len = sizeof(listen_addr);
    assert(getsockname(server, (struct sockaddr *)&listen_addr, &listen_len) == 0);
    assert(listen_addr.sin_family == AF_INET);

    int client = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    assert(client >= 0);
    assert(connect(client, (const struct sockaddr *)&listen_addr, sizeof(listen_addr)) == 0);

    int accepted = accept(server, NULL, NULL);
    assert(accepted >= 0);

    assert(close(server) == 0);

    *client_out = client;
    *accepted_out = accepted;
}

// Sends a payload from `from_fd`, receives it on `to_fd`, and verifies that the bytes round trip
// unchanged. This exercises the push-based `send()` payload transfer end to end.
static void test_send_payload(int from_fd, int to_fd)
{
    char sent[PAYLOAD_LEN];
    fill_pattern(sent, sizeof(sent));

    send_all(from_fd, sent, sizeof(sent));

    char received[PAYLOAD_LEN];
    memset(received, 0, sizeof(received));
    recv_all(to_fd, received, sizeof(received));

    assert(memcmp(sent, received, sizeof(sent)) == 0);
}

// Sends the whole buffer, returning the largest number of bytes transferred by a single send()
// call. A stream send() may accept fewer bytes than offered, so the caller loops until everything
// is written, tracking the largest single transfer to verify the page-bounded push.
static size_t send_all_max_chunk(int sockfd, const char *buffer, size_t len)
{
    size_t offset = 0;
    size_t max_chunk = 0;
    while (offset < len) {
        ssize_t sent = send(sockfd, buffer + offset, len - offset, 0);
        assert(sent > 0);
        if ((size_t)sent > max_chunk) {
            max_chunk = (size_t)sent;
        }
        offset += (size_t)sent;
    }
    assert(offset == len);
    return max_chunk;
}

// Static buffers keep the multi-page payload off the stack.
static char large_sent[LARGE_PAYLOAD_LEN];
static char large_received[LARGE_PAYLOAD_LEN];

// Transfers a multi-page payload over a connected stream socket and verifies both that no single
// send() exceeds the page-bounded push limit and that every byte round trips unchanged. The payload
// fits within the socket buffer pipeline, so the whole buffer is sent before it is drained without a
// concurrent reader and without deadlocking.
static void test_send_large_payload(int from_fd, int to_fd)
{
    fill_pattern(large_sent, sizeof(large_sent));
    memset(large_received, 0, sizeof(large_received));

    size_t max_chunk = send_all_max_chunk(from_fd, large_sent, sizeof(large_sent));

    recv_all(to_fd, large_received, sizeof(large_received));

    // The wrapper caps each transfer at one page, so a larger stream write must complete through
    // repeated short sends.
    assert(max_chunk <= PAGE_BYTES);
    assert(memcmp(large_sent, large_received, sizeof(large_sent)) == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Validates the push-based `send()` payload transfer over a connected stream socket.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    int client = -1;
    int accepted = -1;
    connect_loopback_pair(&client, &accepted);

    // Exercise `send()` in both directions of the connection.
    test_send_payload(client, accepted);
    test_send_payload(accepted, client);

    assert(close(client) == 0);
    assert(close(accepted) == 0);

    // Exercise a payload that spans multiple page-bounded send() calls over a fresh connection in
    // both directions.
    connect_loopback_pair(&client, &accepted);
    test_send_large_payload(client, accepted);
    test_send_large_payload(accepted, client);
    assert(close(client) == 0);
    assert(close(accepted) == 0);

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}

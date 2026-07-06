/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <arpa/inet.h>
#include <assert.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Returns the number of entries in an addrinfo linked list.
static size_t ai_count(const struct addrinfo *list)
{
    size_t count = 0;
    for (const struct addrinfo *node = list; node != NULL; node = node->ai_next) {
        count++;
    }
    return count;
}

// Asserts that an addrinfo node carries an IPv4 socket address with the expected network-order
// address and (host-order) port.
static void check_ipv4(const struct addrinfo *ai, in_addr_t expect_net_addr, uint16_t expect_port)
{
    assert(ai->ai_family == AF_INET);
    assert(ai->ai_addr != NULL);
    assert(ai->ai_addrlen == sizeof(struct sockaddr_in));

    const struct sockaddr_in *sin = (const struct sockaddr_in *)(const void *)ai->ai_addr;
    assert(sin->sin_family == AF_INET);
    assert(sin->sin_addr.s_addr == expect_net_addr);
    assert(sin->sin_port == htons(expect_port));
}

//==================================================================================================
// getaddrinfo(): numeric host resolution (#458)
//==================================================================================================

// Tests that a numeric host with a pinned stream socket type yields a single TCP entry.
static void test_numeric_host_stream(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "8080", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);

    // A pinned socket type produces exactly one entry.
    assert(ai_count(res) == 1);
    assert(res->ai_next == NULL);
    assert(res->ai_socktype == SOCK_STREAM);
    assert(res->ai_protocol == IPPROTO_TCP);
    check_ipv4(res, htonl(0x7f000001), 8080);

    freeaddrinfo(res);
}

// Tests that a numeric host with a pinned datagram socket type yields a single UDP entry.
static void test_numeric_host_dgram(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_DGRAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("192.168.1.1", "53", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);

    assert(ai_count(res) == 1);
    assert(res->ai_socktype == SOCK_DGRAM);
    assert(res->ai_protocol == IPPROTO_UDP);
    check_ipv4(res, inet_addr("192.168.1.1"), 53);

    freeaddrinfo(res);
}

// Tests that an unspecified socket type produces one stream and one datagram entry, in that order.
static void test_unspecified_socktype(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("10.0.0.1", "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    assert(ai_count(res) == 2);

    const struct addrinfo *first = res;
    const struct addrinfo *second = res->ai_next;
    assert(first->ai_socktype == SOCK_STREAM);
    assert(first->ai_protocol == IPPROTO_TCP);
    assert(second->ai_socktype == SOCK_DGRAM);
    assert(second->ai_protocol == IPPROTO_UDP);
    assert(second->ai_next == NULL);

    check_ipv4(first, inet_addr("10.0.0.1"), 80);
    check_ipv4(second, inet_addr("10.0.0.1"), 80);

    freeaddrinfo(res);
}

// Tests that null hints behave like an unconstrained IPv4 lookup.
static void test_null_hints(void)
{
    struct addrinfo *res = NULL;
    int ret = getaddrinfo("8.8.8.8", "443", NULL, &res);
    assert(ret == 0);
    assert(res != NULL);

    // Every entry must describe the same IPv4 endpoint.
    for (const struct addrinfo *node = res; node != NULL; node = node->ai_next) {
        check_ipv4(node, inet_addr("8.8.8.8"), 443);
    }

    freeaddrinfo(res);
}

// Tests that AI_NUMERICHOST accepts a numeric host.
static void test_numeric_host_flag(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_NUMERICHOST;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    check_ipv4(res, htonl(0x7f000001), 80);

    freeaddrinfo(res);
}

// Tests that a protocol hint alone selects the matching socket type.
static void test_protocol_selects_socktype(void)
{
    struct addrinfo hints;
    struct addrinfo *res;

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_protocol = IPPROTO_TCP;
    res = NULL;
    assert(getaddrinfo("127.0.0.1", "80", &hints, &res) == 0);
    assert(ai_count(res) == 1);
    assert(res->ai_socktype == SOCK_STREAM);
    assert(res->ai_protocol == IPPROTO_TCP);
    freeaddrinfo(res);

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_protocol = IPPROTO_UDP;
    res = NULL;
    assert(getaddrinfo("127.0.0.1", "80", &hints, &res) == 0);
    assert(ai_count(res) == 1);
    assert(res->ai_socktype == SOCK_DGRAM);
    assert(res->ai_protocol == IPPROTO_UDP);
    freeaddrinfo(res);
}

// Tests that contradictory protocol and socket-type hints are rejected.
static void test_protocol_rejects_incompatible_socktype(void)
{
    struct addrinfo hints;
    struct addrinfo *res;

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_protocol = IPPROTO_UDP;
    res = NULL;
    assert(getaddrinfo("127.0.0.1", "80", &hints, &res) == EAI_SOCKTYPE);
    assert(res == NULL);

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_DGRAM;
    hints.ai_protocol = IPPROTO_TCP;
    res = NULL;
    assert(getaddrinfo("127.0.0.1", "80", &hints, &res) == EAI_SOCKTYPE);
    assert(res == NULL);
}

// Tests that an unspecified socket type cannot be inferred from an unsupported protocol.
static void test_protocol_rejects_unknown_socktype(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_protocol = IPPROTO_RAW;

    struct addrinfo *res = NULL;
    assert(getaddrinfo("127.0.0.1", "80", &hints, &res) == EAI_SERVICE);
    assert(res == NULL);
}

// Tests that a raw socket type is accepted and passed through with the requested protocol.
static void test_raw_socktype(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_RAW;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "0", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    assert(ai_count(res) == 1);
    assert(res->ai_socktype == SOCK_RAW);

    freeaddrinfo(res);
}

//==================================================================================================
// getaddrinfo(): passive and loopback defaults
//==================================================================================================

// Tests that a null host with AI_PASSIVE resolves to the wildcard address.
static void test_passive_wildcard(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo(NULL, "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    check_ipv4(res, htonl(INADDR_ANY), 80);

    freeaddrinfo(res);
}

// Tests that a null host without AI_PASSIVE resolves to the loopback address.
static void test_null_host_loopback(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo(NULL, "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    check_ipv4(res, htonl(INADDR_LOOPBACK), 80);

    freeaddrinfo(res);
}

//==================================================================================================
// getaddrinfo(): service handling
//==================================================================================================

// Tests that a null service resolves to port zero.
static void test_service_null(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", NULL, &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    check_ipv4(res, htonl(INADDR_LOOPBACK), 0);

    freeaddrinfo(res);
}

// Tests that the boundary numeric ports are accepted.
static void test_service_port_bounds(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    assert(getaddrinfo("127.0.0.1", "0", &hints, &res) == 0);
    check_ipv4(res, htonl(INADDR_LOOPBACK), 0);
    freeaddrinfo(res);

    res = NULL;
    assert(getaddrinfo("127.0.0.1", "65535", &hints, &res) == 0);
    check_ipv4(res, htonl(INADDR_LOOPBACK), 65535);
    freeaddrinfo(res);
}

//==================================================================================================
// getaddrinfo(): canonical name
//==================================================================================================

// Tests that AI_CANONNAME reports the numeric host string.
static void test_canonname_numeric(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_CANONNAME;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    assert(res->ai_canonname != NULL);
    assert(strcmp(res->ai_canonname, "127.0.0.1") == 0);

    freeaddrinfo(res);
}

// Tests that AI_CANONNAME attaches the canonical name to the first entry only.
static void test_canonname_first_only(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_flags = AI_CANONNAME;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
    assert(ret == 0);
    assert(res != NULL);
    assert(ai_count(res) == 2);
    assert(res->ai_canonname != NULL);
    assert(strcmp(res->ai_canonname, "127.0.0.1") == 0);
    assert(res->ai_next->ai_canonname == NULL);

    freeaddrinfo(res);
}

// Tests that AI_CANONNAME reports the synthesized address string for a null host.
static void test_canonname_null_host(void)
{
    struct addrinfo hints;
    struct addrinfo *res;

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_CANONNAME | AI_PASSIVE;
    res = NULL;
    assert(getaddrinfo(NULL, "80", &hints, &res) == 0);
    assert(res->ai_canonname != NULL);
    assert(strcmp(res->ai_canonname, "0.0.0.0") == 0);
    freeaddrinfo(res);

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_CANONNAME;
    res = NULL;
    assert(getaddrinfo(NULL, "80", &hints, &res) == 0);
    assert(res->ai_canonname != NULL);
    assert(strcmp(res->ai_canonname, "127.0.0.1") == 0);
    freeaddrinfo(res);
}

//==================================================================================================
// getaddrinfo(): error handling
//==================================================================================================

// Tests that supplying neither a host nor a service fails with EAI_NONAME.
static void test_both_null(void)
{
    struct addrinfo *res = NULL;
    int ret = getaddrinfo(NULL, NULL, NULL, &res);
    assert(ret == EAI_NONAME);
}

// Tests that a non-numeric host fails with EAI_NONAME, as no resolver is available.
static void test_nonnumeric_host(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("example.com", "80", &hints, &res);
    assert(ret == EAI_NONAME);
    // On error the result list must be left empty.
    assert(res == NULL);

    // AI_NUMERICHOST rejects a non-numeric host the same way.
    hints.ai_flags = AI_NUMERICHOST;
    ret = getaddrinfo("example.com", "80", &hints, &res);
    assert(ret == EAI_NONAME);
}

// Tests that a named service is not resolvable without a services database.
static void test_named_service(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;

    // Without AI_NUMERICSERV a named service is reported as unavailable.
    assert(getaddrinfo("127.0.0.1", "http", &hints, &res) == EAI_SERVICE);

    // With AI_NUMERICSERV a non-numeric service is reported as an unresolvable name.
    hints.ai_flags = AI_NUMERICSERV;
    assert(getaddrinfo("127.0.0.1", "http", &hints, &res) == EAI_NONAME);
}

// Tests that out-of-range and malformed numeric services are rejected.
static void test_service_out_of_range(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    assert(getaddrinfo("127.0.0.1", "65536", &hints, &res) == EAI_SERVICE);
    assert(getaddrinfo("127.0.0.1", "99999", &hints, &res) == EAI_SERVICE);
    assert(getaddrinfo("127.0.0.1", "80x", &hints, &res) == EAI_SERVICE);
    assert(getaddrinfo("127.0.0.1", "-1", &hints, &res) == EAI_SERVICE);
}

// Tests that unknown flags fail with EAI_BADFLAGS.
static void test_bad_flags(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = 0x8000; // Not a recognized AI_* flag.

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
    assert(ret == EAI_BADFLAGS);
}

// Tests that an unsupported address family fails with EAI_FAMILY.
static void test_unsupported_family(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET6; // Only IPv4 is supported.
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("::1", "80", &hints, &res);
    assert(ret == EAI_FAMILY);
}

// Tests that an unsupported socket type fails with EAI_SOCKTYPE.
static void test_unsupported_socktype(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = 999; // Not a recognized SOCK_* type.

    struct addrinfo *res = NULL;
    int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
    assert(ret == EAI_SOCKTYPE);
}

//==================================================================================================
// freeaddrinfo()  (#457)
//==================================================================================================

// Tests that freeaddrinfo() releases a multi-entry list without error and tolerates a null list.
static void test_freeaddrinfo(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_flags = AI_CANONNAME; // Force a canonical-name allocation on the first entry.

    // Repeatedly allocate and free a multi-entry list; a leak or double-free would surface as an
    // allocation failure or a crash across iterations.
    for (int i = 0; i < 64; i++) {
        struct addrinfo *res = NULL;
        int ret = getaddrinfo("127.0.0.1", "80", &hints, &res);
        assert(ret == 0);
        assert(res != NULL);
        assert(ai_count(res) == 2);
        freeaddrinfo(res);
    }

    // Freeing a null list is a well-defined no-op.
    freeaddrinfo(NULL);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests the POSIX network database calls exposed by <netdb.h>.
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

    test_numeric_host_stream();
    test_numeric_host_dgram();
    test_unspecified_socktype();
    test_null_hints();
    test_numeric_host_flag();
    test_protocol_selects_socktype();
    test_protocol_rejects_incompatible_socktype();
    test_protocol_rejects_unknown_socktype();
    test_raw_socktype();
    test_passive_wildcard();
    test_null_host_loopback();
    test_service_null();
    test_service_port_bounds();
    test_canonname_numeric();
    test_canonname_first_only();
    test_canonname_null_host();
    test_both_null();
    test_nonnumeric_host();
    test_named_service();
    test_service_out_of_range();
    test_bad_flags();
    test_unsupported_family();
    test_unsupported_socktype();
    test_freeaddrinfo();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}

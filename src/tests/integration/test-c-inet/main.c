/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <arpa/inet.h>
#include <assert.h>
#include <errno.h>
#include <netinet/in.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

//==================================================================================================
// Macros
//==================================================================================================

#define STATIC_ASSERT_ALIGNMENT(type, alignment) _Static_assert(_Alignof(type) == (alignment), #type)

//==================================================================================================
// Private Functions
//==================================================================================================

// Asserts that inet_ntoa() renders the given host-order address as the expected string.
static void check_ntoa(uint32_t host_addr, const char *expected)
{
    struct in_addr in;
    in.s_addr = htonl(host_addr);

    char *got = inet_ntoa(in);
    assert(got != NULL);
    assert(strcmp(got, expected) == 0);
}

// Asserts that inet_ntop() renders the given host-order IPv4 address as the expected string and
// returns the caller's buffer.
static void check_ntop4(uint32_t host_addr, const char *expected)
{
    struct in_addr in;
    in.s_addr = htonl(host_addr);

    char buf[INET_ADDRSTRLEN];
    memset(buf, 0x55, sizeof(buf));

    const char *got = inet_ntop(AF_INET, &in, buf, sizeof(buf));
    assert(got == buf);
    assert(strcmp(buf, expected) == 0);
}

// Asserts that inet_ntop() renders the given 16-byte IPv6 address as the expected string.
static void check_ntop6(const uint8_t bytes[16], const char *expected)
{
    struct in6_addr in6;
    memcpy(in6.s6_addr, bytes, 16);

    char buf[INET6_ADDRSTRLEN];
    memset(buf, 0x55, sizeof(buf));

    const char *got = inet_ntop(AF_INET6, &in6, buf, sizeof(buf));
    assert(got == buf);
    assert(strcmp(buf, expected) == 0);
}

// Asserts that inet_pton() accepts a valid IPv4 string and stores it in network byte order.
static void check_pton4_ok(const char *src, uint32_t expect_host)
{
    struct in_addr in;
    memset(&in, 0xAA, sizeof(in));

    int ret = inet_pton(AF_INET, src, &in);
    assert(ret == 1);
    assert(in.s_addr == htonl(expect_host));
}

// Asserts that inet_pton() rejects an invalid IPv4 string (returns 0, leaves errno unchanged).
static void check_pton4_bad(const char *src)
{
    struct in_addr in;
    memset(&in, 0xAA, sizeof(in));

    errno = 0;
    int ret = inet_pton(AF_INET, src, &in);
    assert(ret == 0);
    assert(errno == 0);
}

// Asserts that inet_pton() accepts a valid IPv6 string and stores the expected 16 bytes.
static void check_pton6_ok(const char *src, const uint8_t expect[16])
{
    struct in6_addr in6;
    memset(&in6, 0xAA, sizeof(in6));

    int ret = inet_pton(AF_INET6, src, &in6);
    assert(ret == 1);
    assert(memcmp(in6.s6_addr, expect, 16) == 0);
}

// Asserts that inet_pton() rejects an invalid IPv6 string (returns 0, leaves errno unchanged).
static void check_pton6_bad(const char *src)
{
    struct in6_addr in6;
    memset(&in6, 0xAA, sizeof(in6));

    errno = 0;
    int ret = inet_pton(AF_INET6, src, &in6);
    assert(ret == 0);
    assert(errno == 0);
}

//==================================================================================================
// inet_ntoa() (#595)
//==================================================================================================

// Tests inet_ntoa() across representative IPv4 addresses.
static void test_inet_ntoa(void)
{
    check_ntoa(0x00000000, "0.0.0.0");
    check_ntoa(0x7f000001, "127.0.0.1");
    check_ntoa(0xc0a80101, "192.168.1.1");
    check_ntoa(0x08080808, "8.8.8.8");
    check_ntoa(0xffffffff, "255.255.255.255");
    check_ntoa(0x01020304, "1.2.3.4");
}

// Tests that inet_ntoa() reuses static storage that later calls overwrite, as POSIX permits.
static void test_inet_ntoa_static_storage(void)
{
    struct in_addr a = {.s_addr = htonl(0x0a000001)};
    char *first = inet_ntoa(a);
    assert(strcmp(first, "10.0.0.1") == 0);

    struct in_addr b = {.s_addr = htonl(0xac100001)};
    char *second = inet_ntoa(b);
    assert(strcmp(second, "172.16.0.1") == 0);

    // POSIX allows the buffer to be reused; the earlier pointer must now observe the newer value.
    assert(first == second);
    assert(strcmp(first, "172.16.0.1") == 0);
}

//==================================================================================================
// inet_ntop() (#592)
//==================================================================================================

// Tests inet_ntop() for IPv4 addresses.
static void test_inet_ntop_ipv4(void)
{
    check_ntop4(0x00000000, "0.0.0.0");
    check_ntop4(0x7f000001, "127.0.0.1");
    check_ntop4(0xc0a80101, "192.168.1.1");
    check_ntop4(0xffffffff, "255.255.255.255");
    check_ntop4(0x01020304, "1.2.3.4");
}

// Tests inet_ntop() for IPv6 addresses, including zero-run compression and embedded IPv4.
static void test_inet_ntop_ipv6(void)
{
    const uint8_t unspecified[16] = {0};
    check_ntop6(unspecified, "::");

    const uint8_t loopback[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1};
    check_ntop6(loopback, "::1");

    const uint8_t full[16] = {0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8};
    check_ntop6(full, "1:2:3:4:5:6:7:8");

    const uint8_t doc[16] = {0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01};
    check_ntop6(doc, "2001:db8::1");

    const uint8_t linklocal[16] = {0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01};
    check_ntop6(linklocal, "fe80::1");

    // The first (leftmost) longest zero run is the one compressed.
    const uint8_t two_runs[16] = {0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1};
    check_ntop6(two_runs, "2001:db8::1:0:0:1");

    // A single zero group is written verbatim rather than compressed.
    const uint8_t single_zero[16] = {0, 1, 0, 0, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7};
    check_ntop6(single_zero, "1:0:2:3:4:5:6:7");

    // IPv4-mapped address uses the trailing dotted-decimal form.
    const uint8_t mapped[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 192, 168, 1, 1};
    check_ntop6(mapped, "::ffff:192.168.1.1");
}

// Tests that inet_ntop() fails with ENOSPC when the output buffer is too small.
static void test_inet_ntop_enospc(void)
{
    struct in_addr in;
    in.s_addr = htonl(0x7f000001); // "127.0.0.1" requires 10 bytes including NUL.

    // One byte short of the required size must fail.
    char small[9];
    errno = 0;
    assert(inet_ntop(AF_INET, &in, small, sizeof(small)) == NULL);
    assert(errno == ENOSPC);

    // The exact required size must succeed.
    char exact[10];
    assert(inet_ntop(AF_INET, &in, exact, sizeof(exact)) == exact);
    assert(strcmp(exact, "127.0.0.1") == 0);

    // IPv6 with a tiny buffer must also fail with ENOSPC.
    struct in6_addr in6;
    memset(in6.s6_addr, 0, 16);
    in6.s6_addr[15] = 1;
    char tiny[2];
    errno = 0;
    assert(inet_ntop(AF_INET6, &in6, tiny, sizeof(tiny)) == NULL);
    assert(errno == ENOSPC);
}

//==================================================================================================
// inet_pton() (#593)
//==================================================================================================

// Tests inet_pton() for valid and invalid IPv4 strings.
static void test_inet_pton_ipv4(void)
{
    check_pton4_ok("0.0.0.0", 0x00000000);
    check_pton4_ok("127.0.0.1", 0x7f000001);
    check_pton4_ok("192.168.1.1", 0xc0a80101);
    check_pton4_ok("255.255.255.255", 0xffffffff);
    check_pton4_ok("1.2.3.4", 0x01020304);

    // Out-of-range octet.
    check_pton4_bad("256.0.0.1");
    // Too few components (inet_pton() does not accept short forms).
    check_pton4_bad("1.2.3");
    check_pton4_bad("1");
    // Too many components.
    check_pton4_bad("1.2.3.4.5");
    // Leading zeros are rejected (no octal interpretation).
    check_pton4_bad("01.2.3.4");
    check_pton4_bad("1.2.3.04");
    // Hexadecimal is not accepted.
    check_pton4_bad("0x7f.0.0.1");
    // Trailing and malformed input.
    check_pton4_bad("1.2.3.4.");
    check_pton4_bad("1.2.3.4 ");
    check_pton4_bad(".1.2.3");
    check_pton4_bad("1..2.3");
    check_pton4_bad("");
    check_pton4_bad("abc");
}

// Tests inet_pton() for valid and invalid IPv6 strings.
static void test_inet_pton_ipv6(void)
{
    const uint8_t unspecified[16] = {0};
    check_pton6_ok("::", unspecified);

    const uint8_t loopback[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1};
    check_pton6_ok("::1", loopback);

    const uint8_t full[16] = {0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8};
    check_pton6_ok("1:2:3:4:5:6:7:8", full);

    const uint8_t doc[16] = {0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01};
    check_pton6_ok("2001:db8::1", doc);
    // Uppercase hexadecimal digits are accepted.
    check_pton6_ok("2001:DB8::1", doc);

    const uint8_t linklocal[16] = {0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01};
    check_pton6_ok("fe80::1", linklocal);

    const uint8_t leading[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x12, 0x34};
    check_pton6_ok("::1234", leading);

    const uint8_t trailing[16] = {0x12, 0x34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    check_pton6_ok("1234::", trailing);

    // IPv4-mapped address with an embedded dotted-decimal tail.
    const uint8_t mapped[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 192, 168, 1, 1};
    check_pton6_ok("::ffff:192.168.1.1", mapped);

    // Invalid forms.
    check_pton6_bad("");
    check_pton6_bad(":");
    check_pton6_bad(":::");
    check_pton6_bad("1::2::3");    // More than one "::".
    check_pton6_bad("12345::");    // Group with more than four hex digits.
    check_pton6_bad("1:2:3:4:5:6:7:8:9"); // Too many groups.
    check_pton6_bad("1:2:3:4:5:6:7");     // Too few groups without "::".
    check_pton6_bad("g::1");       // Non-hexadecimal digit.
    check_pton6_bad("1:2:3:4:5:6:7:8:"); // Trailing single colon.
    check_pton6_bad("::ffff:256.1.1.1"); // Invalid embedded IPv4.
}

//==================================================================================================
// Cross-cutting behavior
//==================================================================================================

// Tests that an unsupported address family is reported with EAFNOSUPPORT.
static void test_unsupported_family(void)
{
    struct in_addr in;
    in.s_addr = 0;
    char buf[INET6_ADDRSTRLEN];

    errno = 0;
    assert(inet_ntop(AF_UNIX, &in, buf, sizeof(buf)) == NULL);
    assert(errno == EAFNOSUPPORT);

    uint8_t dst[16];
    errno = 0;
    assert(inet_pton(AF_UNIX, "1.2.3.4", dst) == -1);
    assert(errno == EAFNOSUPPORT);
}

// Tests that inet_pton() writes exactly the address bytes and nothing beyond them.
static void test_pton_no_overflow(void)
{
    uint8_t buf4[8];
    memset(buf4, 0xAA, sizeof(buf4));
    assert(inet_pton(AF_INET, "1.2.3.4", buf4) == 1);
    assert(buf4[0] == 1 && buf4[1] == 2 && buf4[2] == 3 && buf4[3] == 4);
    assert(buf4[4] == 0xAA && buf4[5] == 0xAA && buf4[6] == 0xAA && buf4[7] == 0xAA);

    uint8_t buf6[20];
    memset(buf6, 0xAA, sizeof(buf6));
    assert(inet_pton(AF_INET6, "::1", buf6) == 1);
    for (int i = 0; i < 15; i++) {
        assert(buf6[i] == 0);
    }
    assert(buf6[15] == 1);
    assert(buf6[16] == 0xAA && buf6[17] == 0xAA && buf6[18] == 0xAA && buf6[19] == 0xAA);
}

// Tests that inet_pton() followed by inet_ntop() reproduces the canonical text form.
static void test_round_trip(void)
{
    static const char *const ipv4[] = {
        "0.0.0.0", "127.0.0.1", "192.168.1.1", "255.255.255.255", "8.8.4.4"};
    for (size_t i = 0; i < sizeof(ipv4) / sizeof(ipv4[0]); i++) {
        struct in_addr in;
        assert(inet_pton(AF_INET, ipv4[i], &in) == 1);
        char buf[INET_ADDRSTRLEN];
        assert(inet_ntop(AF_INET, &in, buf, sizeof(buf)) == buf);
        assert(strcmp(buf, ipv4[i]) == 0);
    }

    static const char *const ipv6[] = {
        "::", "::1", "1:2:3:4:5:6:7:8", "2001:db8::1", "fe80::1", "::ffff:192.168.1.1"};
    for (size_t i = 0; i < sizeof(ipv6) / sizeof(ipv6[0]); i++) {
        struct in6_addr in6;
        assert(inet_pton(AF_INET6, ipv6[i], &in6) == 1);
        char buf[INET6_ADDRSTRLEN];
        assert(inet_ntop(AF_INET6, &in6, buf, sizeof(buf)) == buf);
        assert(strcmp(buf, ipv6[i]) == 0);
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests POSIX Internet address conversion calls exposed by <arpa/inet.h>.
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

    STATIC_ASSERT_ALIGNMENT(struct in_addr, _Alignof(uint32_t));
    STATIC_ASSERT_ALIGNMENT(struct sockaddr_in, _Alignof(uint32_t));
    STATIC_ASSERT_ALIGNMENT(struct in6_addr, _Alignof(uint32_t));
    STATIC_ASSERT_ALIGNMENT(struct sockaddr_in6, _Alignof(uint32_t));

    test_inet_ntoa();
    test_inet_ntoa_static_storage();
    test_inet_ntop_ipv4();
    test_inet_ntop_ipv6();
    test_inet_ntop_enospc();
    test_inet_pton_ipv4();
    test_inet_pton_ipv6();
    test_unsupported_family();
    test_pton_no_overflow();
    test_round_trip();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}

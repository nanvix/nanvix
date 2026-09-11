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
#include <limits.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Macros
//==================================================================================================

/**
 * @brief Performs a static assertion.
 *
 * @param a Expression to assert.
 * @param b Expected value.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT(a, b) ((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))

/**
 * @brief Performs a static assertion on the size of a type.
 *
 * @param a Type to assert.
 * @param b Expected size.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_SIZE(a, b) STATIC_ASSERT(sizeof(a), b)

/**
 * @brief Performs a static assertion on the alignment of a type.
 *
 * @param a Type to assert.
 * @param b Expected alignment.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_ALIGNMENT(a, b) STATIC_ASSERT(_Alignof(a), b)

//==================================================================================================
// Constants
//==================================================================================================

// Seed for random number generation.
#ifndef __RELEASE
#define SEED 0
#else
#define SEED 42
#endif

// Keep the host-relative socket path within `sockaddr_un.sun_path`.
#define UNIX_SOCKET_HOST_DIRECTORY "bin/n/"
#define UNIX_SOCKET_NAME_LEN 6
#define UNIX_SOCKET_PATH_LEN (sizeof(UNIX_SOCKET_HOST_DIRECTORY) + UNIX_SOCKET_NAME_LEN - 1)

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests networking system calls.
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

    // Sanity check size of `sa_family_t` type.
    STATIC_ASSERT_SIZE(sa_family_t, sizeof(uint8_t));

    // Sanity check size of `in_port_t` type.
    STATIC_ASSERT_SIZE(in_port_t, sizeof(uint16_t));

    // Sanity check size of `in_addr_t` type.
    STATIC_ASSERT_SIZE(in_addr_t, sizeof(uint32_t));

    // Sanity check size of `sockaddr_storage` structure.
    STATIC_ASSERT_SIZE(struct sockaddr_storage,
                       sizeof(unsigned char) +        // ss_len
                           sizeof(sa_family_t) +      // ss_family
                           _SS_PADSIZE * sizeof(char) // __ss_pad1
    );

    // Sanity check size of `sockaddr` structure.
    STATIC_ASSERT_SIZE(struct sockaddr,
                       sizeof(unsigned char) +   // sa_len
                           sizeof(sa_family_t) + // sa_family
                           14 * sizeof(char)     // sa_data
    );
    STATIC_ASSERT_SIZE(struct sockaddr, sizeof(struct sockaddr_storage));

    // Sanity check size of `in_addr` structure.
    STATIC_ASSERT_SIZE(struct in_addr, sizeof(in_addr_t));

    // Sanity check size of `sockaddr_in` structure.
    STATIC_ASSERT_SIZE(struct sockaddr_in,
                       sizeof(uint8_t) +            // sin_len
                           sizeof(sa_family_t) +    // sin_family
                           sizeof(in_port_t) +      // sin_port
                           sizeof(struct in_addr) + // sin_addr
                           8 * sizeof(char)         // sin_zero
    );
    STATIC_ASSERT_SIZE(struct sockaddr_in, sizeof(struct sockaddr_storage));

    // Sanity check size of `sockaddr_un` structure.
    STATIC_ASSERT_SIZE(struct sockaddr_un,
                       sizeof(unsigned char) +       // sun_len
                           sizeof(sa_family_t) +     // sun_family
                           SUNPATHLEN * sizeof(char) // sun_path
    );
    STATIC_ASSERT_SIZE(struct sockaddr_un, sizeof(struct sockaddr_storage));

    srand(SEED);

    in_port_t sin_port = htons(1992);
    struct in_addr sin_addr = {.s_addr = htonl(0x7f000001)};

    test_inet_sockets(sin_port, sin_addr);
    test_poll_services(sin_addr);

    test_unix_socket_pairs();

    if (getenv("NANVIX_TEST_HOSTFS") != NULL) {
        char cwd[PATH_MAX];
        assert(getcwd(cwd, sizeof(cwd)) != NULL);
        assert(chdir("/mnt") == 0);
        char sun_name[UNIX_SOCKET_NAME_LEN];
        for (int i = 0; i < UNIX_SOCKET_NAME_LEN - 1; i++) {
            sun_name[i] = 'a' + (rand() % 26);
        }
        sun_name[UNIX_SOCKET_NAME_LEN - 1] = '\0';
        char sun_path[UNIX_SOCKET_PATH_LEN];
        int length = snprintf(sun_path, sizeof(sun_path), "%s%s", UNIX_SOCKET_HOST_DIRECTORY,
                              sun_name);
        assert(length > 0 && (size_t)length < sizeof(sun_path));
        test_unix_pathname_sockets(sun_path, sun_name);
        assert(chdir(cwd) == 0);
    } else {
#ifdef NANVIX_HOST_WINDOWS
        fprintf(stderr, "skipping UNIX pathname sockets (hostfs not enabled)\n");
#else
        char sun_path[UNIX_SOCKET_NAME_LEN];
        for (int i = 0; i < UNIX_SOCKET_NAME_LEN - 1; i++) {
            sun_path[i] = 'a' + (rand() % 26);
        }
        sun_path[UNIX_SOCKET_NAME_LEN - 1] = '\0';
        test_unix_pathname_sockets(sun_path, sun_path);
#endif
    }

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}

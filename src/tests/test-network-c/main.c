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
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Seed for random number generation.
#ifndef __RELEASE
#define SEED 0
#else
#define SEED 42
#endif

// Length of the UNIX socket name (including the null terminator).
// We set this so it fits on socaddr.sa_data.
#define UNIX_SOCKET_NAME_LEN 9

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

    srand(SEED);

    in_port_t sin_port = htons(1992);
    struct in_addr sin_addr = {.s_addr = htonl(0x7f000001)};
    char sun_path[UNIX_SOCKET_NAME_LEN];
    for (int i = 0; i < UNIX_SOCKET_NAME_LEN - 1; i++) {
        sun_path[i] = 'a' + (rand() % 26);
    }
    sun_path[UNIX_SOCKET_NAME_LEN - 1] = '\0';

    test_inet_sockets(sin_port, sin_addr);
    test_unix_sockets(sun_path);

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}

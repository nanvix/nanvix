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

    in_port_t sin_port = htons(1992);
    struct in_addr sin_addr = {.s_addr = htonl(0x7f000001)};
    char sun_path[] = "/tmp/nanvix-test-socket";

    test_inet_sockets(sin_port, sin_addr);
    test_unix_sockets(sun_path);

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}

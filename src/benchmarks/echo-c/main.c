// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#include <stddef.h>
#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

#define MAX_REQUEST_SIZE 4096

//==================================================================================================
// Global Variables
//==================================================================================================

char buffer[MAX_REQUEST_SIZE];

//==================================================================================================
// Standalone Functions
//==================================================================================================

int main(void)
{
    ssize_t nread = 0;

    while (1) {
        nread = read(STDIN_FILENO, buffer, MAX_REQUEST_SIZE);
        if (nread < 0) {
            break; // Error encountered.
        } else if (nread == 0) {
            break; // End of file reached.
        }

        if (nread > 0) {
            write(STDOUT_FILENO, buffer, nread);
        }
    }

    return 0;
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

constexpr size_t MAX_REQUEST_SIZE = 4096;

//==================================================================================================
// Global Variables
//==================================================================================================

static char buffer[MAX_REQUEST_SIZE];

//==================================================================================================
// Standalone Functions
//==================================================================================================

int main()
{
    ssize_t nread = 0;

    while (true) {
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

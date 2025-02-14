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
    ssize_t nread;
    size_t n = 0;

    while (1) {
        nread = fread(buffer + n, 1, MAX_REQUEST_SIZE - n, stdin);
        if (nread < 0) {
            break; // Error encountered.
        } else if (nread == 0) {
            break; // End of file reached.
        } else {
            n += nread; // Read some bytes.
        }
    }

    if (n > 0) {
        fwrite(buffer, 1, n, stdout);
        fflush(stdout);
    }

    return 0;
}

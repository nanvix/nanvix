// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#include <iostream>
#include <vector>

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
    size_t n = 0;

    while (true) {
        std::cin.read(&buffer[n], MAX_REQUEST_SIZE - n);
        std::streamsize nread = std::cin.gcount();
        if (nread < 0) {
            break; // Error encountered.
        } else if (nread == 0) {
            break; // End of file reached.
        } else {
            n += nread; // Read some bytes.
        }
    }

    if (n > 0) {
        std::cout.write(buffer, n);
        std::cout.flush();
    }

    return 0;
}

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
    while (true) {
        std::cin.read(&buffer[0], MAX_REQUEST_SIZE);
        std::streamsize nread = std::cin.gcount();
        if (nread < 0) {
            break; // Error encountered.
        } else if (nread == 0) {
            break; // End of file reached.
        }

        if (nread > 0) {
            std::cout.write(buffer, nread);
            std::cout.flush();
        }
    }

    return 0;
}

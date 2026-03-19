// UART/IO core for NanVix port
// Uses POSIX read()/write() — NanVix provides these via its POSIX layer.

#include <unistd.h>
#include "py/mpconfig.h"

// Receive single character from stdin.
int mp_hal_stdin_rx_chr(void) {
    unsigned char c = 0;
    int r = read(STDIN_FILENO, &c, 1);
    (void)r;
    return c;
}

// Send string of given length to stdout.
mp_uint_t mp_hal_stdout_tx_strn(const char *str, mp_uint_t len) {
    int r = write(STDOUT_FILENO, str, len);
    (void)r;
    return len;
}

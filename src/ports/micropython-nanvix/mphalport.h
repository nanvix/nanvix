// MicroPython HAL (Hardware Abstraction Layer) for NanVix
// Uses POSIX clock_gettime for timing, read/write for I/O.

#include <time.h>

static inline mp_uint_t mp_hal_ticks_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (mp_uint_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

static inline mp_uint_t mp_hal_ticks_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (mp_uint_t)(ts.tv_sec * 1000000 + ts.tv_nsec / 1000);
}

static inline void mp_hal_set_interrupt_char(char c) {
    (void)c;
}

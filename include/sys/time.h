/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_TIME_H
#define _NANVIX_SYS_TIME_H

/**
 * @file sys/time.h
 * @brief Time-of-day types and interfaces.
 *
 * Declares `struct timeval`, `struct timezone`, and the `gettimeofday`/`utimes`
 * interfaces. The `timeval` layout mirrors the Rust definition in the sysapi
 * crate (sys_select.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Structures
 *==================================================================================================*/

/**
 * @brief Time interval expressed in seconds and microseconds.
 */
struct timeval {
    time_t tv_sec;       /**< Seconds.      */
    suseconds_t tv_usec; /**< Microseconds. */
};

/**
 * @brief Time-zone information (obsolete; retained for compatibility).
 */
struct timezone {
    int tz_minuteswest; /**< Minutes west of Greenwich. */
    int tz_dsttime;     /**< Type of DST correction.    */
};

/*==================================================================================================
 * Convenience macros
 *==================================================================================================*/

#define timerisset(tvp) ((tvp)->tv_sec || (tvp)->tv_usec)
#define timerclear(tvp) ((tvp)->tv_sec = (tvp)->tv_usec = 0)
#define timercmp(a, b, CMP)                                                                        \
    (((a)->tv_sec == (b)->tv_sec) ? ((a)->tv_usec CMP(b)->tv_usec) : ((a)->tv_sec CMP(b)->tv_sec))
#define timeradd(a, b, result)                                                                     \
    do {                                                                                           \
        (result)->tv_sec = (a)->tv_sec + (b)->tv_sec;                                              \
        (result)->tv_usec = (a)->tv_usec + (b)->tv_usec;                                           \
        if ((result)->tv_usec >= 1000000) {                                                        \
            ++(result)->tv_sec;                                                                     \
            (result)->tv_usec -= 1000000;                                                          \
        }                                                                                          \
    } while (0)
#define timersub(a, b, result)                                                                     \
    do {                                                                                           \
        (result)->tv_sec = (a)->tv_sec - (b)->tv_sec;                                              \
        (result)->tv_usec = (a)->tv_usec - (b)->tv_usec;                                           \
        if ((result)->tv_usec < 0) {                                                               \
            --(result)->tv_sec;                                                                     \
            (result)->tv_usec += 1000000;                                                          \
        }                                                                                          \
    } while (0)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

/**
 * @brief Retrieves the current time of day.
 *
 * @param tv Buffer that receives the current time.
 * @param tz Time-zone buffer (obsolete; should be NULL).
 *
 * @returns Zero on success, or -1 on failure with `errno` set.
 */
extern int gettimeofday(struct timeval *tv, void *tz);

/**
 * @brief Sets the system-wide clock.
 *
 * @param tv Buffer holding the time to set.
 * @param tz Time-zone buffer (obsolete; should be NULL).
 *
 * @returns Zero on success, or -1 on failure with `errno` set.
 */
extern int settimeofday(const struct timeval *tv, const void *tz);

/**
 * @brief Sets the access and modification times of a file.
 *
 * @param filename Path of the target file.
 * @param times    Two-element array of access and modification times.
 *
 * @returns Zero on success, or -1 on failure with `errno` set.
 */
extern int utimes(const char *filename, const struct timeval times[2]);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_TIME_H */

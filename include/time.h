/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_TIME_H
#define _NANVIX_TIME_H

/**
 * @file time.h
 * @brief Date and time.
 *
 * Declares functions for calendar time manipulation and conversion.
 * Implemented by the libc_time Rust crate.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

#ifndef _TIME_T_DEFINED
#define _TIME_T_DEFINED
typedef long long time_t;
#endif

#ifndef _CLOCK_T_DEFINED
#define _CLOCK_T_DEFINED
typedef long long clock_t;
#endif

#ifndef _CLOCKID_T_DEFINED
#define _CLOCKID_T_DEFINED
typedef int clockid_t;
#endif

#ifndef CLOCKS_PER_SEC
#define CLOCKS_PER_SEC 1000000
#endif

#ifndef CLOCK_REALTIME
#define CLOCK_REALTIME 1
#endif

#ifndef CLOCK_PROCESS_CPUTIME_ID
#define CLOCK_PROCESS_CPUTIME_ID 2
#endif

#ifndef CLOCK_THREAD_CPUTIME_ID
#define CLOCK_THREAD_CPUTIME_ID 3
#endif

#ifndef CLOCK_MONOTONIC
#define CLOCK_MONOTONIC 4
#endif

/** @brief Broken-down time representation. */
struct tm {
    int tm_sec;   /**< Seconds [0, 60].            */
    int tm_min;   /**< Minutes [0, 59].             */
    int tm_hour;  /**< Hours [0, 23].               */
    int tm_mday;  /**< Day of month [1, 31].        */
    int tm_mon;   /**< Months since January [0, 11].*/
    int tm_year;  /**< Years since 1900.            */
    int tm_wday;  /**< Days since Sunday [0, 6].    */
    int tm_yday;  /**< Days since Jan 1 [0, 365].   */
    int tm_isdst; /**< Daylight Saving Time flag.   */
};

/** @brief Time specification for clock_gettime(). */
struct timespec {
    time_t tv_sec; /**< Seconds.     */
    long tv_nsec;  /**< Nanoseconds. */
};

/*==================================================================================================
 * Calendar Time
 *==================================================================================================*/

extern time_t time(time_t *tloc);
extern clock_t clock(void);
extern double difftime(time_t time1, time_t time0);
extern time_t mktime(struct tm *timeptr);

/*==================================================================================================
 * Time Conversion
 *==================================================================================================*/

extern struct tm *gmtime(const time_t *timep);
extern struct tm *gmtime_r(const time_t *timep, struct tm *result);
extern struct tm *localtime(const time_t *timep);
extern struct tm *localtime_r(const time_t *timep, struct tm *result);

/*==================================================================================================
 * Time Formatting
 *==================================================================================================*/

extern char *asctime(const struct tm *timeptr);
extern char *asctime_r(const struct tm *timeptr, char *buf);
extern char *ctime(const time_t *timep);
extern char *ctime_r(const time_t *timep, char *buf);
extern size_t strftime(char *s, size_t max, const char *format,
                       const struct tm *timeptr);

/*==================================================================================================
 * Clocks
 *==================================================================================================*/

extern int clock_gettime(clockid_t clock_id, struct timespec *tp);
extern int clock_getres(clockid_t clock_id, struct timespec *res);
extern int clock_settime(clockid_t clock_id, const struct timespec *tp);
extern int nanosleep(const struct timespec *req, struct timespec *rem);

/*==================================================================================================
 * Time Parsing
 *==================================================================================================*/

extern char *strptime(const char *s, const char *format, struct tm *tm);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_TIME_H */

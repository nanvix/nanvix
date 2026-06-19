/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_TIMES_H
#define _NANVIX_SYS_TIMES_H

/**
 * @file sys/times.h
 * @brief Process times.
 *
 * Declares the tms structure and the times() interface. The layout mirrors the
 * Rust definition in the sysapi crate (sys_times.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Process and waited-for-children CPU times, in clock ticks. */
struct tms {
    clock_t tms_utime;  /**< User CPU time.                    */
    clock_t tms_stime;  /**< System CPU time.                  */
    clock_t tms_cutime; /**< User CPU time of terminated children.  */
    clock_t tms_cstime; /**< System CPU time of terminated children.*/
};

/*==================================================================================================
 * Process Times
 *==================================================================================================*/

extern clock_t times(struct tms *buffer);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_TIMES_H */

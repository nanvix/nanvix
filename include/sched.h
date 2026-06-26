/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SCHED_H
#define _NANVIX_SCHED_H

/**
 * @file sched.h
 * @brief Execution scheduling.
 *
 * Declares the scheduling policy constants and the sched_param structure used to
 * get and set scheduling parameters. Mirrors the Rust definitions in the sysapi
 * crate (sched.rs).
 */

#include <sys/types.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define SCHED_OTHER 0 /**< Another scheduling policy. */
#define SCHED_FIFO 1 /**< First in-first out scheduling policy. */
#define SCHED_RR 2 /**< Round-robin scheduling policy. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Scheduling parameters. */
struct sched_param {
    int sched_priority; /**< Process or thread execution scheduling priority. */
};

/*==================================================================================================
 * Scheduling
 *==================================================================================================*/

extern int sched_yield(void);
extern int sched_getaffinity(pid_t pid, size_t cpusetsize, void *mask);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SCHED_H */

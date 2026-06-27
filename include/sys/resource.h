/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_RESOURCE_H
#define _NANVIX_SYS_RESOURCE_H

/**
 * @file sys/resource.h
 * @brief Resource limits and priorities.
 *
 * Declares the resource limit interfaces (getrlimit/setrlimit) and the process
 * priority interfaces (getpriority/setpriority), together with the rlimit
 * structure and the associated resource and priority identifiers. The rlimit
 * layout mirrors the Rust definition in the sysapi crate (sys_resource.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Resource Limit Identifiers
 *==================================================================================================*/

#define RLIMIT_CPU 0      /**< CPU time, in seconds.          */
#define RLIMIT_FSIZE 1    /**< Maximum file size.             */
#define RLIMIT_DATA 2     /**< Maximum data segment size.     */
#define RLIMIT_STACK 3    /**< Maximum stack size.            */
#define RLIMIT_CORE 4     /**< Maximum core file size.        */
#define RLIMIT_RSS 5      /**< Maximum resident set size.     */
#define RLIMIT_NPROC 6    /**< Maximum number of processes.   */
#define RLIMIT_NOFILE 7   /**< Maximum number of open files.  */
#define RLIMIT_MEMLOCK 8  /**< Maximum locked-in-memory size. */
#define RLIMIT_AS 9       /**< Maximum address space size.    */
#define RLIMIT_NLIMITS 10 /**< Number of resource limits.     */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Resource limit value type. */
typedef unsigned long rlim_t;

/** @brief Resource limit. */
struct rlimit {
    rlim_t rlim_cur; /**< Soft limit. */
    rlim_t rlim_max; /**< Hard limit. */
};

/*==================================================================================================
 * Resource Limit Values
 *==================================================================================================*/

/** @brief Unlimited resource value. */
#define RLIM_INFINITY ((rlim_t)-1)
/** @brief Unrepresentable saved soft-limit value. */
#define RLIM_SAVED_CUR RLIM_INFINITY
/** @brief Unrepresentable saved hard-limit value. */
#define RLIM_SAVED_MAX RLIM_INFINITY

/*==================================================================================================
 * Priority Identifiers
 *==================================================================================================*/

#define PRIO_PROCESS 0 /**< Identifies a process.       */
#define PRIO_PGRP 1    /**< Identifies a process group. */
#define PRIO_USER 2    /**< Identifies a user.          */

/*==================================================================================================
 * Resource Limits
 *==================================================================================================*/

extern int getrlimit(int resource, struct rlimit *rlim);
extern int setrlimit(int resource, const struct rlimit *rlim);

/*==================================================================================================
 * Process Priorities
 *==================================================================================================*/

extern int getpriority(int which, int who);
extern int setpriority(int which, int who, int prio);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_RESOURCE_H */

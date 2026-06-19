/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_UTSNAME_H
#define _NANVIX_SYS_UTSNAME_H

/**
 * @file sys/utsname.h
 * @brief System name structure.
 *
 * Declares the utsname structure and the uname() interface. The layout mirrors
 * the Rust definition in the syscall crate (sys/utsname).
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define _UTSNAME_LENGTH 64 /**< Length of each utsname field. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief System name structure. */
struct utsname {
    char sysname[_UTSNAME_LENGTH];  /**< Operating system name.   */
    char nodename[_UTSNAME_LENGTH]; /**< Network node name.       */
    char release[_UTSNAME_LENGTH];  /**< Operating system release.*/
    char version[_UTSNAME_LENGTH];  /**< Operating system version.*/
    char machine[_UTSNAME_LENGTH];  /**< Hardware identifier.     */
};

/*==================================================================================================
 * System Information
 *==================================================================================================*/

extern int uname(struct utsname *name);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_UTSNAME_H */

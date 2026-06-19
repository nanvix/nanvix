/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_UN_H
#define _NANVIX_SYS_UN_H

/**
 * @file sys/un.h
 * @brief UNIX domain socket address.
 *
 * Declares the UNIX-domain socket address structure. The layout mirrors the Rust
 * definition in the sysapi crate (sys_un.rs).
 */

#include <sys/socket.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define SUNPATHLEN 14 /**< Size of the sun_path field. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief UNIX domain socket address. */
struct sockaddr_un {
    unsigned char sun_len;     /**< Total length.   */
    sa_family_t sun_family;    /**< Address family. */
    char sun_path[SUNPATHLEN]; /**< Socket path.    */
};

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_UN_H */

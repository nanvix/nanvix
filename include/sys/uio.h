/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_UIO_H
#define _NANVIX_SYS_UIO_H

/**
 * @file sys/uio.h
 * @brief Vectored I/O.
 *
 * Declares the iovec structure and the scatter/gather I/O interfaces. The layout
 * mirrors the Rust definition in the sysapi crate (sys_uio.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief An I/O vector. */
struct iovec {
    void *iov_base; /**< Base address of the buffer.  */
    size_t iov_len; /**< Length of the buffer.        */
};

/*==================================================================================================
 * Vectored I/O
 *==================================================================================================*/

extern ssize_t readv(int32_t fd, const struct iovec *iov, int32_t iovcnt);
extern ssize_t writev(int fd, const struct iovec *iov, int iovcnt);
extern ssize_t preadv(int32_t fd, const struct iovec *iov, int32_t iovcnt, off_t offset);
extern ssize_t pwritev(int32_t fd, const struct iovec *iov, int iovcnt, off_t offset);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_UIO_H */

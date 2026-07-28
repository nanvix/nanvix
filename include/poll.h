/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_POLL_H
#define _NANVIX_POLL_H

/**
 * @file poll.h
 * @brief Synchronous I/O multiplexing via poll().
 *
 * The `pollfd` layout mirrors the Rust definition in the sysapi crate
 * (poll.rs).
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

typedef unsigned int nfds_t;

/** @brief A descriptor to be polled. */
struct pollfd {
    int fd;        /**< File descriptor.        */
    short events;  /**< Requested events.       */
    short revents; /**< Returned events.        */
};

/*==================================================================================================
 * Event flags
 *==================================================================================================*/

#define POLLIN 0x0001
#define POLLPRI 0x0002
#define POLLOUT 0x0004
#define POLLERR 0x0008
#define POLLHUP 0x0010
#define POLLNVAL 0x0020
#define POLLRDNORM POLLIN
#define POLLRDBAND 0x0080
#define POLLWRNORM POLLOUT
#define POLLWRBAND 0x0100

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int poll(struct pollfd *fds, nfds_t nfds, int timeout);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_POLL_H */

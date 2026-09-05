/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_SELECT_H
#define _NANVIX_SYS_SELECT_H

/**
 * @file sys/select.h
 * @brief Synchronous I/O multiplexing.
 *
 * Declares `fd_set`, the `FD_*` manipulation macros, and `select()`. The
 * `fd_set` layout mirrors the Rust definition in the sysapi crate
 * (sys_select.rs).
 */

#include <sys/time.h>
#include <sys/types.h>

/* `restrict` is C99-only; expand it to the keyword in C and to nothing in C++,
   where it is a parse error (even as `__restrict`) inside array parameters. */
#ifndef __nanvix_restrict
#ifdef __cplusplus
#define __nanvix_restrict
#else
#define __nanvix_restrict restrict
#endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

#define FD_SETSIZE 64

#define _NANVIX_NFDBITS (8 * (int)sizeof(unsigned long))

/** @brief A set of file descriptors. */
typedef struct {
    unsigned long fds_bits[FD_SETSIZE / (8 * sizeof(unsigned long))];
} fd_set;

/*==================================================================================================
 * Manipulation macros
 *==================================================================================================*/

#define FD_ZERO(set)                                                                               \
    do {                                                                                           \
        unsigned int _i;                                                                           \
        for (_i = 0; _i < sizeof(fd_set) / sizeof(unsigned long); _i++)                            \
            ((fd_set *)(set))->fds_bits[_i] = 0;                                                   \
    } while (0)
#define FD_SET(fd, set)                                                                            \
    ((set)->fds_bits[(fd) / _NANVIX_NFDBITS] |= (1UL << ((fd) % _NANVIX_NFDBITS)))
#define FD_CLR(fd, set)                                                                            \
    ((set)->fds_bits[(fd) / _NANVIX_NFDBITS] &= ~(1UL << ((fd) % _NANVIX_NFDBITS)))
#define FD_ISSET(fd, set)                                                                          \
    (((set)->fds_bits[(fd) / _NANVIX_NFDBITS] & (1UL << ((fd) % _NANVIX_NFDBITS))) != 0)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int select(int nfds,
                  fd_set *__nanvix_restrict readfds,
                  fd_set *__nanvix_restrict writefds,
                  fd_set *__nanvix_restrict exceptfds,
                  struct timeval *__nanvix_restrict timeout);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_SELECT_H */

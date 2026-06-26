/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_WAIT_H
#define _NANVIX_SYS_WAIT_H

/**
 * @file sys/wait.h
 * @brief Process-termination wait interface.
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Options
 *==================================================================================================*/

#define WNOHANG 1
#define WUNTRACED 2

/*==================================================================================================
 * Status-decoding macros
 *==================================================================================================*/

#define WIFEXITED(status) (((status) & 0x7f) == 0)
#define WEXITSTATUS(status) (((status) >> 8) & 0xff)
#define WIFSIGNALED(status) (((status) & 0x7f) != 0 && ((status) & 0x7f) != 0x7f)
#define WTERMSIG(status) ((status) & 0x7f)
#define WIFSTOPPED(status) (((status) & 0xff) == 0x7f)
#define WSTOPSIG(status) WEXITSTATUS(status)
/* Non-standard but widely used (glibc/BSD): reports whether the child produced
 * a core dump. Nanvix never sets the core-dump bit, so WCOREDUMP() is always
 * false, but the macro lets portable software referencing it compile. */
#define WCOREFLAG 0x80
#define WCOREDUMP(status) ((status) & WCOREFLAG)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern pid_t waitpid(pid_t pid, int *status, int options);
extern pid_t wait(int *status);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_WAIT_H */

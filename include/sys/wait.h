#ifndef _SYS_WAIT_H
#define _SYS_WAIT_H

#ifdef __cplusplus
extern "C" {
#endif

#include <sys/types.h>

#define WNOHANG 1
#define WUNTRACED 2

#define WIFEXITED(w)	((w >> 8) & 1)
#define WIFSIGNALED(w)	((w >> 9) & 1)
#define WIFSTOPPED(w)	(((w) >> 10) & 1)
#define WEXITSTATUS(w)	(w & 0xff)
#define WTERMSIG(w)     ((w >> 16) & 0xff)
#define WSTOPSIG	WEXITSTATUS

pid_t wait (int *);
pid_t waitpid (pid_t, int *, int);

#ifdef _COMPILING_NEWLIB
pid_t _wait (int *);
#endif

/* Provide prototypes for most of the _<systemcall> names that are
   provided in newlib for some compilers.  */
pid_t _wait (int *);

#ifdef __cplusplus
};
#endif

#endif

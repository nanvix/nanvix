#ifndef _SIGNAL_H_
#define _SIGNAL_H_

	#define NR_SIGNALS 23
	#define NSIG       23

	/* Signals. */
	#define SIGNULL  0
	#define SIGKILL  1
	#define SIGSTOP  2
	#define SIGURG   3
	#define SIGABRT  4
	#define SIGBUS   5
	#define SIGCHLD  6
	#define SIGCONT  7
	#define SIGFPE   8
	#define SIGHUP   9
	#define SIGILL  10
	#define SIGINT  11
	#define SIGPIPE 12
	#define SIGQUIT 13
	#define SIGSEGV 14
	#define SIGTERM 15
	#define SIGTSTP 16
	#define SIGTTIN 17
	#define SIGTTOU 18
	#define SIGALRM 19
	#define SIGUSR1 20
	#define SIGUSR2 21
	#define SIGTRAP 22

	/* Handlers. */
	#define _SIG_DFL 1
	#define SIG_DFL ((_sig_func_ptr)_SIG_DFL)	/* Default action */
	#define _SIG_IGN 2
	#define SIG_IGN ((_sig_func_ptr)_SIG_IGN)	/* Ignore action */

	/* Other constants. */
	#define _SIG_ERR 0
	#define SIG_ERR ((_sig_func_ptr)_SIG_ERR)	/* Error return */

#ifndef _ASM_FILE_

	#include "_ansi.h"
	#include <sys/signal.h>

	_BEGIN_STD_C

	typedef int	sig_atomic_t;		/* Atomic entity type (ANSI) */
	#ifndef _POSIX_SOURCE
	typedef _sig_func_ptr sig_t;		/* BSD naming */
	typedef _sig_func_ptr sighandler_t;	/* glibc naming */
	#endif /* !_POSIX_SOURCE */

	struct _reent;

	_sig_func_ptr _EXFUN(_signal_r, (struct _reent *, int, _sig_func_ptr));
	int	_EXFUN(_raise_r, (struct _reent *, int));

	#ifndef _REENT_ONLY
	_sig_func_ptr _EXFUN(signal, (int, _sig_func_ptr));
	int	_EXFUN(raise, (int));
	void	_EXFUN(psignal, (int, const char *));
	#endif

	_END_STD_C

#endif /* _ASM_FILE_ */

#endif /* _SIGNAL_H_ */

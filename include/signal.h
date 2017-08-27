/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

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

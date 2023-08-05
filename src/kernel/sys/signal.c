/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <signal.h>

/*
 * Manages signal handling.
 */
PUBLIC sighandler_t sys_signal
(int sig, sighandler_t func, void (*restorer)(void))
{
	sighandler_t old_func;

	/* Invalid signal. */
	if ((sig < 0) || (sig >= NR_SIGNALS))
		return (SIG_ERR);

	/* Cannot be caught or ignored. */
	if ((sig == SIGKILL) || (sig == SIGSTOP))
		return (SIG_ERR);

	/* Set signal handler. */
	old_func = curr_proc->handlers[sig];
	curr_proc->restorer = restorer;
	curr_proc->handlers[sig] = func;

	return (old_func);
}

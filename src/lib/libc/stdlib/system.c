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

/**
 * @file
 *
 * @brief system() implementation.
 */

#include <sys/types.h>
#include <sys/wait.h>

#include <unistd.h>
#include <stdlib.h>
#include <signal.h>

/**
 * @brief Issues a command.
 *
 * @param command Command to issue.
 *
 * @returns If @p command is a null pointer, non-zero value is returned to
 *          indicate that a command processor is available, and zero otherwise.
 *
 *          If @p command is not a null pointer,  the termination status of the
 *          command language is returned.
 */
int system(const char *command)
{
	int status;                   /* Exit status.       */
	pid_t pid;                    /* Process ID.        */
	const char *argv[4];          /* Command arguments. */
	sighandler_t sigint_handler;  /* SIGINT handler.    */
	sighandler_t sigquit_handler; /* SIGQUIT handler.   */

	/* Ignore SIGINT and SIGQUIT. */
	sigint_handler = signal(SIGINT, SIG_IGN);
	sigquit_handler = signal(SIGQUIT, SIG_IGN);

	/* Build command arguments. */
	argv[0] = "sh";
	argv[1] = "-c";
	argv[2] = (command == NULL) ? "exit 0" : command;
	argv[3] = NULL;

	if ((pid = fork()) < 0)
		return ((command == NULL) ? 0 : -1);

	/* Execute command. */
	else if (pid == 0)
	{
		execvp("sh", (char * const *)argv);
		exit(-1);
	}

	/* Wait for child. */
	wait(&status);

	/* Restore signal handlers. */
	signal(SIGINT, sigint_handler);
	signal(SIGQUIT, sigquit_handler);

	return ((command == NULL) ? (status == 0) : WTERMSIG(status));
}

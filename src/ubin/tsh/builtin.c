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

#include <sys/wait.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "builtin.h"
#include "tsh.h"

/*
 * Changes the current working directory.
 */
static int builtin_cd(int argc, const char **argv)
{
	/* Wrong usage. */
	if (argc < 2)
	{
		fputs("cd: insufficient arguments\n", stderr);
		return (EXIT_FAILURE);
	}

	/* Change working directory. */
	if (chdir(argv[1]) == -1)
	{
		fputs("cd: failed\n", stderr);
		return (EXIT_FAILURE);
	}

	return (EXIT_SUCCESS);
}

/*
 * Exits the shell.
 */
static int builtin_exit(int argc, const char **argv)
{
	((void)argc);
	((void)argv);

	exit(shret);

	/* Never gets here. */
	return (0);
}

/*
 * Waits for all child processes to complete.
 */
static int builtin_wait(int argc, const char **argv)
{
	int status;

	((void)argc);
	((void)argv);

	/* Wait for all child processes to complete */
	while (wait(&status) != -1)
		/* empty. */;

	return (EXIT_SUCCESS);
}

/*
 * Gets the requested built-in command.
 */
builtin_t getbuiltin(const char *cmdname)
{
	if (!strcmp(cmdname, "cd"))
		return (&builtin_cd);
	else if (!strcmp(cmdname, "exit"))
		return (&builtin_exit);
	else if (!strcmp(cmdname, "wait"))
		return (&builtin_wait);

	return (NULL);
}

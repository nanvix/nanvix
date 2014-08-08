/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * builtin.c - Built-in commands library implementation.
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

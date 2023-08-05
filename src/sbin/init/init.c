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

#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
/**
 * @brief Init table size.
 */
#define INITTAB_SIZE 10

/**
 * @brief Line size.
 */
#define LINE_SIZE 80

/**
 * @brief Maximum number of arguments.
 */
#define NARGS 5

/**
 * @brief Init table.
 */
struct
{
	int used;                   /**< Slot in use? */
	int respaw;                 /**< Re-spawn?    */
	pid_t pid;                  /**< Process ID.  */
	char line[LINE_SIZE];       /**< Raw line.    */
	const char *cmd[NARGS + 1]; /**< Command.     */

} inittab[INITTAB_SIZE];

/**
 * @brief Reads a line from a file.
 *
 * @param fd     Underlying file descriptor.
 * @param line   Line buffer.
 * @param length Maximum line length.
 *
 * @returns Zero in case of EOF and non-zero otherwise.
 */
static int readline(int fd, char *buf, size_t length)
{
	/* Read line. */
	for (size_t i = 0; i < (length - 1); i++)
	{
		char ch;

		if (read(fd, &ch, 1) < 1)
			return (0);

		if (ch == '\n')
		{
			buf[i] = '\0';
			break;
		}

		buf[i] = ch;
	}

	return (1);
}

/**
 * @brief Reads init table into memory.
 *
 * @returns Zero on successful completion and non-zero otherwise.
 */
static int inittab_read(void)
{
	int fd;

	/* Failed to open inittab. */
	if ((fd = open("/etc/inittab", O_RDONLY)) < 0)
		return (-1);

	/* Initialize init table. */
	for (int i = 0; i < INITTAB_SIZE; i++)
		inittab[i].used = 0;

	/* Read init table. */
	for (int i = 0; i < INITTAB_SIZE; i++)
	{
		/* Read line. */
		if (!readline(fd, inittab[i].line, LINE_SIZE))
			break;

		inittab[i].used = 1;
		inittab[i].respaw = (inittab[i].line[0] == 'y') ? 1 : 0;

		/* Parse command. */
		inittab[i].cmd[0] = strtok(&inittab[i].line[2], " ");
		for (int j = 1; j < NARGS; j++)
		{
			if ((inittab[i].cmd[j] = strtok(NULL, " ")) == NULL)
				break;
		}
		inittab[i].cmd[NARGS] = NULL;
	}

	/* House keeping. */
	close(fd);

	return (0);
}

/**
 * @brief Spawns a system process.
 */
static void spawn(int i)
{
	inittab[i].pid = fork();

	/* Failed to fork. */
	if (inittab[i].pid < 0)
		return;

	/* Child process. */
	if (inittab[i].pid == 0)
	{
		const char *cmd;   /* Command.   */
		const char **args; /* Arguments. */

		setpgrp();

		/* Open standard output streams. */
		open("/dev/tty", O_RDONLY);
		open("/dev/tty", O_WRONLY);
		open("/dev/tty", O_WRONLY);

		/* Execute! */
		cmd = inittab[i].cmd[0];
		args = &inittab[i].cmd[1];
		_exit(execve(cmd, (char *const*)args, (char *const*)environ));
	}
}

/*
 * Spawns other system process.
 */
int main(int argc, char **argv)
{
	((void)argc);
	((void)argv);

	/* Read init table. */
	if (inittab_read())
		goto out;

	/* Spawn processes in the inittab. */
	for (int i = 0; inittab[i].used; i++)
		spawn(i);

	/* Wait child processes. */
	while (1)
	{
		pid_t pid;

		sync();
		pid = wait(NULL);

		/* Re-spawn process? */
		for (int i = 0; inittab[i].used; i++)
		{
			/* Not this process. */
			if (inittab[i].pid != pid)
				continue;

			/* Do not respawn. */
			if (!inittab[i].respaw)
				continue;

			spawn(i);
		}
	}

out:
	shutdown();
	return (0);
}

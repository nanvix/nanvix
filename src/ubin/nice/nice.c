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

#include <sys/types.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Default scheduling priority increment. */
#define INC_DEFAULT 10

/*
 * Program arguments.
 */
static struct
{
	int inc;        /* Scheduling priority increment.*/
	char **command; /* Program to execute.           */
} args = { INC_DEFAULT, NULL };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("nice (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2014 Pedro H. Penna\n");
	printf("This is free software under the ");
	printf("GNU General Public License Version 3.\n");
	printf("There is NO WARRANTY, to the extent permitted by law.\n\n");

	exit(EXIT_SUCCESS);
}

/*
 * Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: nice [options] <command> [arguments...]\n\n");
	printf("Brief: Runs a program with modified scheduling priority.\n\n");
	printf("Options:\n");
	printf("  --help         Display this information and exit\n");
	printf("  -n <increment> System scheduling priority increment\n");
	printf("  --version      Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;           /* Loop index.       */
	char *arg;       /* Current argument. */
	int state;       /* Processing state. */
	int set_command; /* Set command?      */

	/* State values. */
	#define READ_ARG 0 /* Read argument.                     */
	#define SET_INC  1 /* Set scheduling priority increment. */

	state = READ_ARG;
	set_command = 1;

	/* Read command line arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Set value. */
		if (state != READ_ARG)
		{
			switch (state)
			{
				/* Set signal number. */
				case SET_INC:
					args.inc = atoi(arg);
					state = READ_ARG;
					break;

				/* Bad usage.*/
				default:
					usage();
			}

			continue;
		}

		/* Parse command line argument. */
		if (!strcmp(arg, "--help")) {
			usage();
		}
		else if (!strcmp(arg, "--version")) {
			version();
		}
		else if (!strcmp(arg, "-n")) {
			state = SET_INC;
		}
		else if (set_command)
		{
			args.command = (char **)&argv[i];
			set_command = 0;
		}
	}

	/* Check if arguments are valid. */
	if (args.command == NULL)
	{
		fprintf(stderr, "nice: missing command\n");
		usage();
	}
}

/*
 * Flushes file system buffers.
 */
int main(int argc, char *const argv[])
{
	pid_t pid;

	getargs(argc, argv);

	pid = fork();

	/* Failed to fork(). */
	if (pid < 0)
	{
		fprintf(stderr, "nice: cannot fork()\n");
		return (EXIT_FAILURE);
	}
	/* Child process. */
	else if (pid == 0)
	{
		if (nice(args.inc) < 0)
		{
			fprintf(stderr, "nice: cannot nice()\n");
			return (EXIT_FAILURE);
		}

		execvp(args.command[0], &args.command[0]);

		fprintf(stderr, "nice: cannot execvp()\n");
		return (EXIT_FAILURE);
	}

	return (EXIT_SUCCESS);
}

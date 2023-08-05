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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/*
 * Program arguments.
 */
static struct
{
	char *source; /* Source file. */
	char *target; /* Target file. */
} args = { NULL, NULL };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("ln (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: ln [options] <source file> <taget file>\n\n");
	printf("Brief: Creates a link between two files.\n\n");
	printf("Options:\n");
	printf("  --help    Display this information and exit\n");
	printf("  --version Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;     /* Loop index.       */
	char *arg; /* Current argument. */
	int state; /* Processing state. */

	/* Processing states. */
	#define SET_NONE   0
	#define SET_SOURCE 1
	#define SET_TARGET 2

	state = SET_SOURCE;

	/* Read command line arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Parse command line argument. */
		if (!strcmp(arg, "--help")) {
			usage();
		}
		else if (!strcmp(arg, "--version")) {
			version();
		}
		else if (state == SET_SOURCE)
		{
			args.source = arg;
			state = SET_TARGET;
		}
		else if (state == SET_TARGET)
		{
			args.target = arg;
			state = SET_NONE;
		}
	}

	/* Missing arguments. */
	if ((args.source == NULL) || (args.target == NULL))
	{
		fprintf(stderr, "ln: missing arguments\n");
		usage();
	}
}

/*
 * Creates a link between two files.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	/* Failed to link(). */
	if (link(args.source, args.target) < 0)
	{
		fprintf(stderr, "ln: cannot link()\n");
		return (EXIT_FAILURE);
	}

	return (EXIT_SUCCESS);
}

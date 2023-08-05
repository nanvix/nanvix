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

#include <sys/stat.h>
#include <limits.h>
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
	char *name1; /* Name of file 1. */
	char *name2; /* Name of file 2. */
} args = { NULL, NULL };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("mv (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: mv [options] <name1> <name2>\n\n");
	printf("Brief: Moves or renames files.\n\n");
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
	#define SET_NONE  0 /* Set none.           */
	#define SET_NAME1 1 /* Set name of file 1. */
	#define SET_NAME2 2 /* Set name of file 2. */

	state = SET_NAME1;

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
		else if (state == SET_NAME1)
		{
			args.name1 = arg;
			state = SET_NAME2;
		}
		else if (state == SET_NAME2)
		{
			args.name2 = arg;
			state = SET_NONE;
		}
		else
		{
			fprintf(stderr, "mv: too many operands?\n");
			usage();
		}

	}

	/* Missing operands. */
	if ((args.name1 == NULL) || (args.name2 == NULL))
	{
		fprintf(stderr, "mv: missing operands\n");
		usage();
	}
}

/*
 * Builds name of file 2.
 */
static void buildname(char *name1, char *name2, char *strbuf)
{
	char *p1, *p2;

	while ((*strbuf = *name2) != '\0') {
		strbuf++; name2++;
	}
	strbuf[-1] = '/';
	p1 = p2 = name1;
	while (*name1++ != '\0')
	{
		if (*p1++ == '/') {
			p2 = p1;
		}
	}
	while ((*strbuf = *p2) != '\0') {
		strbuf++; p2++;
	}
}

/*
 * Creates a link between two files
 */
int main(int argc, char *const argv[])
{
	char *name2;           /* Name of file 2. */
	struct stat st;        /* stat() buffer.  */
	char strbuf[PATH_MAX]; /* String buffer.  */

	getargs(argc, argv);

	name2 = args.name2;

	/* Failed to stat(). */
	if (stat(args.name1, &st) < 0)
	{
		fprintf(stderr, "mv: source file does not exist\n");
		return (EXIT_FAILURE);
	}

	/* Moving directory is not allowed. */
	if (S_ISDIR(st.st_mode))
	{
		fprintf(stderr, "mv: cannot move a directory\n");
		return (EXIT_FAILURE);
	}

again:
	/* File 2 already exits... */
	if (stat(name2, &st) == 0)
	{
		/* Directory. */
		if (S_ISDIR(st.st_mode))
		{
			buildname(args.name1, name2, strbuf);
			name2 = strbuf;
			goto again;
		}

		/* Failed to unlink(). */
		if (unlink(name2) < 0)
		{
			fprintf(stderr, "mv: cannot remove %s\n", name2);
			return (EXIT_FAILURE);
		}
	}

	/* Failed to link(). */
	if (link(args.name1, name2) < 0)
	{
		fprintf(stderr, "mv: cannot link()\n");
		return (EXIT_FAILURE);
	}

	/* Failed to unlink(). */
	if (unlink(args.name1) < 0)
	{
		fprintf(stderr, "mv: cannot unlink()\n");
		return (EXIT_FAILURE);
	}

	return (EXIT_SUCCESS);
}

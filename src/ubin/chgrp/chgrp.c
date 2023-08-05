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
#include <sys/types.h>
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
struct
{
	gid_t group;    /* File's group owner. */
	char *filename; /* Filename.           */
} args = { 0, NULL };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("chgrp (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: chgrp [options] <group owner> <file>\n\n");
	printf("Brief: Change file's group owner.\n\n");
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
	int i;      /* Loop index.       */
	char *arg;  /* Working argument. */
	int state;  /* Processing state. */

	/* Processing states. */
	#define GET_OPTIONS  0
	#define GET_GROUP    1
	#define GET_FILENAME 2

	/* Get program arguments. */
	state = GET_OPTIONS;
	for (i = 1; i < argc; /* noop*/)
	{
		arg = argv[i];

		/* Get file's group owner. */
		if (state == GET_GROUP)
		{
			args.group = atoi(arg);
			state = GET_FILENAME;
		}

		/* Get filename. */
		else if (state == GET_FILENAME)
		{
			args.filename = arg;
			break;
		}

		/* Get options. */
		else
		{
			/* Done. */
			if (arg[0] != '-')
			{
				state = GET_GROUP;
				continue;
			}

			/* Display help information. */
			else if (!strcmp(arg, "--help")) {
				usage();
			}

			/* Display program version. */
			else if (!strcmp(arg, "--version")) {
				version();
			}
		}

		i++;
	}

	/* Missing argument(s). */
	if (args.filename == NULL)
	{
		fprintf(stderr, "chown: missing argument(s)\n");
		usage();
	}
}

/*
 * Change file's group owner.
 */
static void do_chgrp(const char *filename, uid_t group)
{
	gid_t owner;    /* File's owner. */
	struct stat st; /* File status.  */

	/* Get file status. */
	if (stat(filename, &st) < 0)
	{
		fprintf(stderr, "chgrp: cannot stat()\n");
		exit(EXIT_FAILURE);
	}

	owner = st.st_uid;

	/* Change file's group owner. */
	if (chown(filename, owner, group) < 0)
	{
		fprintf(stderr, "chgrp: cannot chown()\n");
		exit(EXIT_FAILURE);
	}
}

/*
 * Change file's owner.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	do_chgrp(args.filename, args.group);

	return (EXIT_FAILURE);
}


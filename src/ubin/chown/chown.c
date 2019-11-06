/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
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
	uid_t owner;    /* File's owner. */
	char *filename; /* Filename.     */
} args = { 0, NULL };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("chown (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: chown [options] <owner> <file>\n\n");
	printf("Brief: Change file's owner.\n\n");
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
	#define GET_OWNER    1
	#define GET_FILENAME 2

	/* Get program arguments. */
	state = GET_OPTIONS;
	for (i = 1; i < argc; /* noop*/)
	{
		arg = argv[i];

		/* Get file's owner. */
		if (state == GET_OWNER)
		{
			args.owner = atoi(arg);
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
				state = GET_OWNER;
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
 * Change file's owner.
 */
static void do_chown(const char *filename, uid_t owner)
{
	gid_t group;    /* File's owner group. */
	struct stat st; /* File status.        */

	/* Get file status. */
	if (stat(filename, &st) < 0)
	{
		fprintf(stderr, "chown: cannot stat()\n");
		exit(EXIT_FAILURE);
	}

	group = st.st_gid;

	/* Change file's owner. */
	if (chown(filename, owner, group) < 0)
	{
		fprintf(stderr, "chown: cannot chown()\n");
		exit(EXIT_FAILURE);
	}
}

/*
 * Change file's owner.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	do_chown(args.filename, args.owner);

	return (EXIT_FAILURE);
}

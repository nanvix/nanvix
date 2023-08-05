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
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Program arguments. */
static mode_t mode = 0;
static const char *filename = NULL;

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("chmod (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: chmod [options] <mode> <file>\n\n");
	printf("Brief: Change file's mode.\n\n");
	printf("Options:\n");
	printf("  --help    Display this information and exit\n");
	printf("  --version Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Changes file's mode.
 */
static void do_chmod(const char *filename, mode_t mode)
{
	int ret;

	/* Invalid mode. */
	if (mode > 0777)
	{
		fprintf(stderr, "chmod: invalid mode %o\n", mode);
		usage();
	}

	mode = ((mode & 0001) ? S_IXOTH : 0) |
	       ((mode & 0002) ? S_IWOTH : 0) |
	       ((mode & 0004) ? S_IROTH : 0) |
	       ((mode & 0010) ? S_IXGRP : 0) |
	       ((mode & 0020) ? S_IWGRP : 0) |
	       ((mode & 0040) ? S_IRGRP : 0) |
	       ((mode & 0100) ? S_IXUSR : 0) |
	       ((mode & 0200) ? S_IWUSR : 0) |
	       ((mode & 0400) ? S_IRUSR : 0);

	ret = chmod(filename, mode);

	/* Failed to change file's mode. */
	if (ret == -1)
	{
		fprintf(stderr, "chmod: cannot chmod\n");
		exit(EXIT_FAILURE);
	}
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;      /* Loop index.       */
	char *arg;  /* Working argument. */
	int serrno; /* Saved errno. */

	/* Get program arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Display help information. */
		if (!strcmp(arg, "--help"))
			usage();

		/* Display program version. */
		else if (!strcmp(arg, "--version"))
			version();

		/* Get file's mode. */
		else if (mode == 0)
		{
			serrno = errno;
			errno = 0;

			mode = strtoul(arg, NULL, 8);

			/* Failed to convert string. */
			if (errno)
			{
				fprintf(stderr, "chmod: invalid mode %s\n", arg);
				usage();
			}

			errno = serrno;
		}

		/* Get filename. */
		else if(filename == NULL)
			filename = arg;

		/* Too many arguments. */
		else
		{
			fprintf(stderr, "chmod: too many arguments\n");
			usage();
		}
	}

	/* Missing argument(s). */
	if ((mode == 0) || (filename == NULL))
	{
		fprintf(stderr, "chmod: missing argument(s)\n");
		usage();
	}
}

/*
 * Launches program.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	do_chmod(filename, mode);

	return (EXIT_SUCCESS);
}

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
#include <errno.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <utime.h>
#include <unistd.h>

/* Software version. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/**
 * @brief Name of the file that should be touched.
 */
static char *filename = NULL;

/**
 * @brief Change file timestamps.
 *
 * @details Change timestamps of the filename named @p filename to current time.
 *
 * @param filename Name of the file.
 *
 * @returns Upon successful completion, zero is returned. Upon failure, -1 is
 *          returned and #errno is set accordingly.
 */
static int touch(const char *filename)
{
	int fd;      /* File descriptor. */
	mode_t mode; /* File mode.       */

	/* Create file, if it does not exist. */
	mode = S_IRUSR | S_IRGRP | S_IROTH | S_IWUSR | S_IWGRP | S_IWOTH;
	fd = open(filename, O_CREAT, mode);
	if (fd < 0)
	{
		fprintf(stderr, "error: failed to create file\n");
		return (errno);
	}
	close(fd);

	return (utime(filename, NULL));
}

/**
 * @brief Prints program version and exits.
 *
 * @details Prints program version and exits gracefully.
 */
static void version(void)
{
	printf("touch (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2014 Pedro H. Penna\n");
	printf("This is free software under the ");
	printf("GNU General Public License Version 3.\n");
	printf("There is NO WARRANTY, to the extent permitted by law.\n\n");

	exit(EXIT_SUCCESS);
}

/**
 * @brief Prints program usage and exits.
 *
 * @details Prints program usage and exits gracefully.
 */
static void usage(void)
{
	printf("Usage: touch <file>\n\n");
	printf("Brief: Changes file timestamps.\n\n");
	printf("Options:\n");
	printf("      --help    Display this information and exit\n");
	printf("      --version Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/**
 * @brief Gets command line arguments.
 *
 * @details Parses @p argc and @p argv in order to get command line arguments.
 *
 * @param argc Argument count.
 * @param argv Argument list.
 */
static void getargs(int argc, char *const argv[])
{
	char *arg;

	/* Get program arguments. */
	for (int i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Display help information. */
		if (!strcmp(arg, "--help"))
			usage();

		/* Display program version. */
		else if (!strcmp(arg, "--version"))
			version();

		/* Get filename. */
		else
			filename = arg;
	}

	/* Bad filename. */
	if (filename == NULL)
	{
		fprintf(stderr, "error: bad filename\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Changes file timestamps.
 *
 * @details Changes the timestamps of the file named @p argv[1] to the current
 *          time.
 *
 * @param argc Argument count.
 * @param argv Argument list.
 *
 * @returns Upon successful completion, zero is returned. Upon failure, a
 *          negative error code is returned instead.
 */
int main(int argc, char *const argv[])
{
	int err;

	getargs(argc, argv);

	err = touch(filename);
	if (err < 0)
	{
		fprintf(stderr, "error: failed to update file timestamps\n");
		return (EXIT_FAILURE);
	}

	return (EXIT_SUCCESS);
}

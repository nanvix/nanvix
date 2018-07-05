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
#include <fcntl.h>
#include <errno.h>

/**
 * @name Program Versioning
 */
/**@{*/
#define PROG_NAME "crtfile" /**< Program name.         */
#define VERSION_MINOR 0     /**< Minor version number. */
#define VERSION_MAJOR 1     /**< Major version number. */
/**@}*/

/*
 * Program arguments.
 */
static struct
{
    char *name; /* Name of file. */
} args = { NULL };

/**
 * @brief Prints program version and exits.
 *
 * @details Prints program version and exits gracefully.
 */
static void version(void)
{
	printf("%s (Nanvix Coreutils) %d.%d\n\n", PROG_NAME, VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2015 Pedro H. Penna\n");
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
	printf("Usage: %s<name>\n\n", PROG_NAME);
	printf("Brief: Create a file.\n\n");
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
	char *arg; /* Current argument. */
    int state; /* Processing state. */

	/* Processing states. */
    #define SET_NONE  0 /* Set none.           */
    #define SET_NAME 1 /* Set name of file. */

	state = SET_NAME;

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
        else if (state == SET_NAME)
		{
			args.name = arg;
			state = SET_NONE;
		}
		else
		{
			fprintf(stderr, "%s: too many operands?\n", PROG_NAME);
			usage();
		}
	}

	/* Missing operands. */
    if ((args.name == NULL))
    {
        fprintf(stderr, "%s: missing operands\n", PROG_NAME);
        usage();
    }
}

/**
 * @brief Create a file.
 *
 * @details Create an empty file.
 *
 * @returns Upon successful completion EXIT_SUCCESS is returned.
 */
int main(int argc, char **argv)
{
	int fd;            

	getargs(argc, argv);

    /* Create new file. */
	fd = open(args.name, O_CREAT, 0644);
    if (fd < 0)
	{
		fprintf(stderr, "%s: open failed: %s\n",
				PROG_NAME, strerror(errno));
		return (EXIT_FAILURE);
	}

    /* House keeping. */
    close(fd);

	return (EXIT_SUCCESS);
}

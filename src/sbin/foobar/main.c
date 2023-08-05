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

/**
 * @name Program Versioning
 */
/**@{*/
#define PROG_NAME  "FooBar" /**< Program name.         */
#define VERSION_MINOR 0     /**< Minor version number. */
#define VERSION_MAJOR 1     /**< Major version number. */
/**@}*/

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
	printf("Usage: %s\n\n", PROG_NAME);
	printf("Brief: Eats CPU time.\n\n");
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
	}
}

/**
 * @brief Eats CPU time.
 *
 * @details Eats CPU time in an infinity loop.
 *
 * @returns Upon successful completion EXIT_SUCCESS is returned.
 */
int main(int argc, char **argv)
{
	getargs(argc, argv);

	/* Eat CPU time. */
	while (1) /* noop */;

	/* Really never gets here. */
	return (EXIT_SUCCESS);
}


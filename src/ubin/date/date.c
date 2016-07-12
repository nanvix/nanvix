/*
 * Copyright(C) 2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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
#include <time.h>
#include <sys/types.h>

/**
 * @name Program Versioning
 */
/**@{*/
#define PROG_NAME  "date" 	/**< Program name.         */
#define VERSION_MAJOR 1 	/**< Major version number. */
#define VERSION_MINOR 0 	/**< Minor version number. */
/**@}*/

/**
 * @brief Returns time since Epoch
 * 
 * @details Returns time in seconds since Epoch (00:00:00 UTC 1st Jan, 1970)
 * 
 * @param Pointer to an memory location for holding time_t type value.
 * 
 * @returns Upon successful completion, this function should return the number
 * 	    of seconds since 00:00:00 UTC 1st January, 1970
 */
static signed date(time_t *curr_time)
{
	return time(curr_time);
}

/**
 * @brief Prints program version and exits.
 * 
 * @details Prints program version and exits gracefully.
 */
static void version(void)
{
	printf("%s (Nanvix Coreutils) %d.%d\n\n", PROG_NAME, VERSION_MAJOR,\
		VERSION_MINOR);
	printf("Copyright(C) 2016-2016 Subhra S. Sarkar\n");
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
	printf("Brief: Returns time elapsed in seconds since Epoch.\n\n");
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
	time_t curr_time = 0;
	
	getargs(argc, argv);
	
	err = date(&curr_time);
	if (err < 0)
	{
		fprintf(stderr, "error: failed to update file timestamps\n");
		return (EXIT_FAILURE);
	}

	printf("%d seconds since Epoch\n", curr_time);
	
	return (EXIT_SUCCESS);
}

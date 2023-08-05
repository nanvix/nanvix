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
#include <unistd.h>
#include <stdlib.h>
#include <string.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Seconds to sleep */
static unsigned seconds;

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("sleep (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: sleep [seconds]\n\n");
	printf("Brief: Puts current process to sleep.\n\n");
	printf("Options:\n");
	printf("      --help    Display this information and exit\n");
	printf("      --version Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;            /* Loop index.            */
	char *arg;        /* Working argument.      */

    int seconds_read; /* Seconds actually read. */

    if (argc != 2)
    {
        usage();
    }

	/* Get program arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Display help information. */
		if (!strcmp(arg, "--help")) {
			usage();
	    }
		/* Display program version. */
		else if (!strcmp(arg, "--version")) {
			version();
        }
		/* Get directory name. */
		else {
			if ((seconds_read = atoi(arg)) <= 0)
                usage();
            seconds = (unsigned) seconds_read;
        }
	}
}

/*
 * Puts current process to sleep.
 */
int main(int argc, char *const argv[])
{
    getargs(argc, argv);

    sleep(seconds);

    return (EXIT_SUCCESS);
}

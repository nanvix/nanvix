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
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Program arguments. */
static char *const *filenames; /* Files to concatenate.           */
static int nfiles = 0;         /* Number of files to concatenate. */

/*
 * Concatenates a file.
 */
static void cat(char *filename)
{
	int fd;           /* File descriptor.        */
	int off;          /* Buffer offset.          */
	char buf[BUFSIZ]; /* Buffer.                 */
	ssize_t n, nread; /* Bytes actually read.    */
	ssize_t nwritten; /* Bytes actually written. */

	fd = open(filename, O_RDONLY);

	/* Failed to open file. */
	if (fd < 0)
	{
		fprintf(stderr, "cat: cannot open %s\n", filename);
		return;
	}

	/* Concatenate file. */
	do
	{
		/* Error while reading. */
		if ((nread = read(fd, buf, BUFSIZ)) < 0)
			fprintf(stderr, "cat: cannot read %s\n", filename);

		n = nread;

		off = 0;
		do
		{
			nwritten = write(fileno(stdout), &buf[off], nread);

			/* Failed to write. */
			if (nwritten < 0)
			{
				fprintf(stderr, "cat: write error\n");
				exit(errno);
			}
			off += nwritten;
		} while ((nread -= nwritten) > 0);
	} while (n == BUFSIZ);

	close(fd);
}

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("cat (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: cat [options] [files]\n\n");
	printf("Brief: Concatenates files.\n\n");
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
	char *arg; /* Working argument. */

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

		/* Get file names. */
		else
		{
			if (nfiles++ == 0)
				filenames = &argv[i];
		}
	}
}

/*
 * Concatenates files.
 */
int main(int argc, char *const argv[])
{
	int i;          /* Loop index.               */
	char *filename; /* Name of the working file. */
	struct stat st; /* Working file's status.    */

	getargs(argc, argv);

	/* Concatenate standard input. */
	if (nfiles == 0)
		cat("/dev/tty");

	/* Concatenate files. */
	else
	{
		for (i = 0; i < nfiles; i++)
		{
			filename = filenames[i];

			/* Concatenate standard input. */
			if (!strcmp(filename, "-"))
				filename = "/dev/tty";

			/* Check if file is not a directory. */
			else
			{
				if (stat(filename, &st) == -1)
				{
					fprintf(stderr, "cat: cannot stat %s\n", filename);
					continue;
				}

				/* File is directory. */
				if (S_ISDIR(st.st_mode))
				{
					fprintf(stderr, "cat: %s is a directory\n", filename);
					continue;
				}
			}

			cat(filename);
		}
	}

	return(EXIT_SUCCESS);
}

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
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Filenames. */
static const char *src = NULL;  /* Source file. */
static const char *dest = NULL; /* Destination file. */

/*
 * Copies a file.
 */
static void do_cp(int src, int dest)
{
	ssize_t count;    /* Bytes read/written.  */
	char buf[BUFSIZ]; /* Buffer.              */

	/* Copy source file into destination file. */
	while ((count = read(src, buf, BUFSIZ)) > 0) {
		count = write(dest, buf, count);

		/* Write error. */
		if (count < 0) {
			fprintf(stderr, "cp: write error\n");
			exit(EXIT_FAILURE);
		}
	}

	/* Error while reading. */
	if (count < 0) {
		fprintf(stderr, "cp: read error\n");
		exit(EXIT_FAILURE);
	}
}

/*
 * Prepares to copy a file.
 */
static void cp(const char *dest, const char *src)
{
	dev_t dev;        /* Source file device number.        */
	ino_t ino;        /* Source file inode number.         */
	int srcfd;        /* Source file file descriptor.      */
	int destfd;       /* Destination file file descriptor. */
	struct stat st;   /* Working file's status.            */
	mode_t mode;      /* Creation mode.                    */

	/* Cannot stat source file. */
	if (stat(src, &st) == -1) {
		fprintf(stderr, "cp: cannot stat %s\n", src);
		exit(EXIT_FAILURE);
	}

	/* Source file is not a regular file.  */
	if(S_ISDIR(st.st_mode)) {
		fprintf(stderr, "cp: %s is not a regular file\n", src);
		exit(EXIT_FAILURE);
	}

	dev = st.st_dev;
	ino = st.st_ino;
	mode = st.st_mode;

	/* Destination file already exists. */
	if (stat(dest, &st) != -1) {
		/* Cannot truncate it. */
		if (!S_ISREG(st.st_mode)) {
			fprintf(stderr, "cp: cannot truncate %s\n", dest);
			exit(EXIT_FAILURE);
		}

		/* Cannot copy a file onto itself. */
		if  ((st.st_dev == dev) && (st.st_ino == ino)) {
			fprintf(stderr, "cp: cannot copy a file onto itself\n");
			exit(EXIT_FAILURE);
		}
	}

	srcfd = open(src, O_RDONLY);

	/* Failed to open source file. */
	if (srcfd == -1) {
		fprintf(stderr, "cp: cannot open %s\n", src);
		exit(EXIT_FAILURE);
	}

	destfd = open(dest, O_WRONLY | O_TRUNC | O_CREAT, mode);

	/* Failed to open destination file. */
	if (destfd == -1) {
		fprintf(stderr, "cp: cannot open %s\n", dest);
		exit(EXIT_FAILURE);
	}

	do_cp(srcfd, destfd);

	close(destfd);
	close(srcfd);
}

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("cp (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2014 Pedro H. Penna\n");
	printf("This is free software under the ");
	printf("GNU General Public License Version 3.\n");
	printf("There is NO WARRANTY, to the extent permitted by law.\n\n");

	exit(EXIT_SUCCESS);
}

/*
 * Prints program usage and exits.
 */
static void usage(int status)
{
	printf("Usage: cp [options] <source> <dest>\n\n");
	printf("Brief: Copies a file.\n\n");
	printf("Options:\n");
	printf("  --help    Display this information and exit\n");
	printf("  --version Display program version and exit\n");

	exit(status);
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;     /* Loop index.       */
	char *arg; /* Working argument. */

	/* Get program arguments. */
	for (i = 1; i < argc; i++) {
		arg = argv[i];

		/* Display help information. */
		if (!strcmp(arg, "--help"))
			usage(EXIT_SUCCESS);

		/* Display program version. */
		else if (!strcmp(arg, "--version"))
			version();

		/* Get source/destination filenames. */
		else {
			/* Get source filename */
			if (src == NULL)
				src = arg;

			/* Get destination filename. */
			else if (dest == NULL)
				dest = arg;

			/* Too many arguments. */
			else {
				fprintf(stderr, "cp: too many arguments\n");
				usage(EXIT_FAILURE);
			}
		}
	}

	/* Missing arguments. */
	if ((src == NULL) || (dest == NULL)) {
		fprintf(stderr, "cp: missing arguments\n");
		usage(EXIT_FAILURE);
	}
}

/*
 * Launches program.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	cp(dest, src);

	return (EXIT_SUCCESS);
}

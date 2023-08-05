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

#include <dirent.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Program flags. */
#define LS_ALL   001     /* Print entries starting with dot? */
#define LS_INODE 002     /* Print inode numbers.             */
static int ls_flags = 0; /* Flags.                           */

/* Name of the directory to list. */
static char *dirname = NULL;

/*
 * Lists contents of a directory.
 */
int ls(const char *pathname)
{
	DIR *dirp;                   /* Directory.               */
	struct dirent *dp;           /* Working directory entry. */
	char filename[NAME_MAX + 1]; /* Working file name.       */

	/* Open directory. */
	if ((dirp = opendir(pathname)) == NULL)
	{
		fprintf(stderr, "ls: cannot open %s\n", pathname);
		return (errno);
	}

	errno = 0;

	/* Read directory entries. */
	filename[NAME_MAX] = '\0';
	while ((dp = readdir(dirp)) != NULL)
	{
		strncpy(filename, dp->d_name, NAME_MAX);

		/* Suppress entries starting with dot. */
		if ((filename[0] == '.') && !(ls_flags & LS_ALL))
			continue;

		/* Print inode number. */
		if (ls_flags & LS_INODE)
			printf("%d ", (int)dp->d_ino);

		printf("%s\n", filename);
	}
	closedir(dirp);

	/* Error while reading. */
	if (errno != 0)
	{
		fprintf(stderr, "ls: cannot read %s\n", pathname);
		return (errno);
	}

	return (EXIT_SUCCESS);
}

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("ls (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
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
	printf("Usage: ls [options] [directory]\n\n");
	printf("Brief: Lists contents of a directory.\n\n");
	printf("Options:\n");
	printf("  -a, --all     List all entries\n");
	printf("  -i, --inode   Print the inode number of each file\n");
	printf("      --help    Display this information and exit\n");
	printf("      --version Display program version and exit\n");

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

		/* Print entries starting with dot. */
		if ((!strcmp(arg, "-a")) || (!strcmp(arg, "--all")))
			ls_flags |= LS_ALL;

		/* Print inode numbers. */
		else if ((!strcmp(arg, "-i")) || (!strcmp(arg, "--inode")))
			ls_flags |= LS_INODE;

		/* Display help information. */
		else if (!strcmp(arg, "--help"))
			usage();

		/* Display program version. */
		else if (!strcmp(arg, "--version"))
			version();

		/* Get directory name. */
		else
			dirname = arg;
	}

	/*
	 * Empty directory name, so use
	 * the current directory.
	 */
	if (dirname == NULL)
		dirname = ".";
}

/*
 * Lists contents of a directory
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	ls(dirname);

	return (EXIT_SUCCESS);
}

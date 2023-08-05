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
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* File name. */
char *filename = NULL;

/*
 * Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: stat [options] <file>\n");
	printf("Brief: Display file status.\n\n");
	printf("Options:\n");
	printf("  --help    Display this information and exit\n");
	printf("  --version Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("stat (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2014 Pedro H. Penna\n");
	printf("This is free software under the ");
	printf("GNU General Public License Version 3.\n");
	printf("There is NO WARRANTY, to the extent permitted by law.\n\n");

	exit(EXIT_SUCCESS);
}

/*
 * Read command arguments.
 */
static void readargs(int argc, char *const argv[])
{
	int i;          /* Loop index.       */
	char *arg;      /* Working argument. */

	filename = NULL;

	/* Parse command arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Print program usage. */
		if (!strcmp(arg, "--help")) {
			usage();
		}

		/* Print program version. */
		else if (!strcmp(arg, "--version")) {
			version();
		}

		/* Get file name. */
		else {
			filename = arg;
		}
	}

	/* Wrong usage. */
	if (filename == NULL)
	{
		printf("stat: missing operand\n");
		usage();
	}
}

/*
 * Returns file type string.
 */
static char *filetype(mode_t mode)
{
	char *type;

	if (S_ISREG(mode))       type = "regular file";
	else if (S_ISDIR(mode))  type = "directory";
	else if (S_ISCHR(mode))  type = "character special file";
	else if (S_ISBLK(mode))  type = "block special file";
	else if (S_ISFIFO(mode)) type = "FIFO";
	else                     type = "unknown";

	return (type);
}

/*
 * Returns file access string.
 */
static char *fileacc(mode_t mode)
{
	static char acess[10] = "---------";

	acess[0] = (mode & S_IROTH) ? 'r' : '-';
	acess[1] = (mode & S_IWOTH) ? 'w' : '-';
	acess[2] = (mode & S_IXOTH) ? 'x' : '-';
	acess[3] = (mode & S_IRGRP) ? 'r' : '-';
	acess[4] = (mode & S_IWGRP) ? 'w' : '-';
	acess[5] = (mode & S_IXGRP) ? 'x' : '-';
	acess[6] = (mode & S_IRUSR) ? 'r' : '-';
	acess[7] = (mode & S_IWUSR) ? 'w' : '-';
	acess[8] = (mode & S_IXUSR) ? 'x' : '-';

	return (acess);
}

/*
 * Displays file status.
 */
int main(int argc, char *const argv[])
{
	struct stat st;

	readargs(argc, argv);

	/* Get file status. */
	if (stat(filename, &st) < 0)
	{
		printf("stat: cannot stat()\n");
		return (errno);
	}

	/* Print file status. */
	printf("    Name: %s\n", filename);
	printf("    Type: %s\n", filetype(st.st_mode));
	printf("    Size: %d bytes\n", (signed)st.st_size);
	printf("   Inode: %u\n", (unsigned)st.st_ino);
	printf("  Device: %xh\n", (unsigned)st.st_dev);
	printf("   Links: %d\n", (signed)st.st_nlink);
	printf("  Access: %s\n",  fileacc(st.st_mode));
	printf(" User ID: %d\n", st.st_uid);
	printf("Group ID: %d\n", st.st_gid);

	return (EXIT_SUCCESS);
}

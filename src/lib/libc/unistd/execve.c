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

#include <nanvix/syscall.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/*
 * Executes a program.
 */
int execve(const char *filename, char *const argv[],  char *const envp[])
{
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_execve),
		  "b" (filename),
		  "c" (argv),
		  "d" (envp)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (ret);
}

/*
 * Executes a program.
 */
int execvp(const char *file, char *const argv[])
{
	int denied; /* Access denied?       */
	char *name; /* Working path name.   */
	int length; /* File name length.    */
	char *path; /* Working path.        */
	char *p;    /* End of working path. */

	/* Use given path. */
	if (strchr(file, '/') != NULL)
	{
		execv(file, (char *const*)argv);
		return (errno);
	}

	denied = 0;
	length = strlen(file) + 1;
	path = getenv("PATH");

	name = malloc(length + strlen(path));
	if (name == NULL)
		return (-1);

	/* Search for executable. */
	do
	{
		/* No directory to search. */
		if ((path == NULL) || (*path == '\0'))
		{
			errno = ENOENT;
			goto error;
		}

		/* Get end of path. */
		p = strchr(path, ':');
		if (p == NULL)
			p = strchr(path, '\0');

		/* Build path name.  */
		memcpy (name, path, p - path);
		name[p - path] = '/';
		memcpy (&name[(p - path) + 1], file, length);

		errno = 0;
		execv(name, argv);

		/* Failed to execute. */
		switch (errno)
		{
			/* Fall through. */
			case EACCES:
				denied = 1;
			case ENOENT:
				break;

			default:
				goto error;
		}

		path = strchr(path, ':');

	} while ((path != NULL) && (*++path != '\0'));

	/*
	 * At least one failure was due to
	 * permissions.
	 */
	if (denied)
		errno = EACCES;

error:
	free(name);
	return (-1);
}


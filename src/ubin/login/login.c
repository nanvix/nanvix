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
#include <stropts.h>
#include <unistd.h>
#include <string.h>

#include <sys/utsname.h>

#include <nanvix/accounts.h>
#include <nanvix/config.h>
#include <dev/tty.h>

#if (MULTIUSER == 1)

/**
 * @brief Authenticates a user in the system.
 *
 * @param name     User name.
 * @param password User's password.
 *
 * @returns One if the user has authentication and zero otherwise.
 */
static int authenticate(const char *name, const char *password)
{
	int ret;          /* Return value.    */
	int file;         /* Passwords file.  */
	struct account a; /* Working account. */

	ret = 1;

	/* Open passwords file. */
	if ((file = open("/etc/passwords", O_RDONLY)) == -1)
	{
		fprintf(stderr, "cannot open password file\n");
		return (0);
	}

	/* Search in the  passwords file. */
	while (read(file, &a, sizeof(struct account)))
	{
		account_decrypt(a.name, USERNAME_MAX, KERNEL_HASH);

		/* No this user. */
		if (strcmp(name, a.name))
			continue;

		account_decrypt(a.password, PASSWORD_MAX, KERNEL_HASH);

		/* Found. */
		if (!strcmp(password, a.password))
		{
			setgid(a.gid);
			setuid(a.uid);
			goto found;
		}
	}

	ret = 0;
	fprintf(stderr, "\nwrong login or password\n\n");

found:

	/* House keeping. */
	close(file);

	return (ret);
}

/**
 * @brief Attempts to login.
 *
 * @returns One if successful login and false otherwise.
 */
static int login(void)
{
	char name[USERNAME_MAX];
	char password[PASSWORD_MAX];

	printf("login: ");
	fgets(name, USERNAME_MAX, stdin);
	printf("password: ");
	fgets(password, PASSWORD_MAX, stdin);

	return (authenticate(name, password));
}

#endif

/*
 * Logins a user.
 */
int main(int argc, char *const argv[])
{
	char *arg[2];        /* Shell arguments.    */
	struct utsname name; /* System information. */

	((void)argc);
	((void)argv);

	arg[0] = "-";
	arg[1] = NULL;

	(void) setvbuf(stdin, NULL, _IOLBF, 0);
	(void) setvbuf(stdout, NULL, _IONBF, 0);

	ioctl(fileno(stdout), TTY_CLEAR);

	/* Get system information. */
	if (uname(&name) != 0)
	{
		fprintf(stderr, "cannot uname()");
		return (EXIT_FAILURE);
	}

	printf("%s %s on %s\n\n", name.sysname, name.version, name.nodename);

#if (MULTIUSER == 1)

	while (!login())
		/* noop */;

#endif

	ioctl(fileno(stdout), TTY_CLEAR);

	printf("Nanvix - A Free Educational Operating System\n\n");
	printf("The programs included with Nanvix system are free software\n");
	printf("under the GNU General Public License Version 3.\n\n");
	printf("Nanvix comes with ABSOLUTELY NO WARRANTY, to the extent\n");
	printf("permitted by applicable law.\n\n");

	execve("/bin/tsh", (char *const*)arg, (char *const*)environ);

	return (EXIT_SUCCESS);
}

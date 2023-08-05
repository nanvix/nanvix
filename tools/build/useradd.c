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

#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/types.h>

#include <accounts.h>

/**
 * @brief Adds a user to the passwords file.
 *
 * @param file     Passwords file.
 * @param user     User name.
 * @param password Password.
 * @param uid      User ID.
 * @param gid      User's group ID.
 */
static void useradd
(FILE *file, const char *name, const char *password, uid_t uid, gid_t gid)
{
	struct account a;

	/* Initialize account. */
	strncpy(a.name, name, USERNAME_MAX);
	strncpy(a.password, password, PASSWORD_MAX);
	a.uid = uid;
	a.gid = gid;

	/* Encrypt data. */
	account_encrypt(a.name, USERNAME_MAX, KERNEL_HASH);
	account_encrypt(a.password, PASSWORD_MAX, KERNEL_HASH);

	/* Write account. */
	fwrite(&a, sizeof(struct account), 1, file);
}

/**
 * @brief Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: useradd <file> <name> <password> <uid> <gid>\n\n");
	printf("Brief: Adds a user account to the system.\n");

	exit(EXIT_SUCCESS);
}

/**
 * @brief Adds a user account to the system.
 */
int main(int argc, char **argv)
{
	FILE *file;
	uid_t uid;
	gid_t gid;
	const char *name;
	const char *password;

	if (argc < 6)
		usage();

	name = argv[2];
	password = argv[3];
	uid = atoi(argv[4]);
	gid = atoi(argv[5]);

	/* Failed to open passwords file. */
	if ((file = fopen(argv[1], "a+")) == NULL)
		return (EXIT_FAILURE);

	useradd(file, name, password, uid, gid);

	/* House keeping. */
	fclose(file);

	return (EXIT_SUCCESS);
}

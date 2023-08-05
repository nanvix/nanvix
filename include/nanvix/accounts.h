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

#ifndef ACCOUNTS_H_
#define ACCOUNTS_H_

	#include <sys/types.h>

	/**
	 * @brief Maximum length for a user name.
	 */
	#define USERNAME_MAX 20

	/**
	 * @brief Maximum length for a password.
	 */
	#define PASSWORD_MAX 10

	/**
	 * @brief User's account information.
	 */
	struct account
	{
		char name[USERNAME_MAX];     /**< User name.       */
		char password[PASSWORD_MAX]; /**< User's password. */
		uid_t uid;                   /**< User's ID.       */
		gid_t gid;                   /**< User's group ID. */
	};

	/**
	 * @brief Encrypts a string.
	 *
	 * @param str String to encode.
	 * @param n   String length.
	 * @param key Encrypting key.
	 */
	extern inline void account_encrypt(char *str, size_t n, int key)
	{
		for (size_t i = 0; i < n; i++)
		{
			if (str[i] == '\0')
				return;

			str[i] += key;
		}
	}

	/**
	 * @brief Decrypts a string.
	 *
	 * @param str String to encode.
	 * @param n   String length.
	 * @param key Encrypting key.
	 */
	extern inline void account_decrypt(char *str, size_t n, int key)
	{
		for (size_t i = 0; i < n; i++)
		{
			if (str[i] == '\0')
				return;

			str[i] -= key;
		}
	}

#endif /* ACCOUNTS_H_ */

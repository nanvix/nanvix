/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/**
 * @file
 * 
 * @brief String library.
 */

#ifndef STRING_H_
#define STRING_H_

	#include <sys/types.h>

	/**
	 * @defgroup stringlib String Library
	 * 
	 * @brief String operations.
	 * 
	 * @todo strcoll()
	 */
	/**@{*/

	extern void *memccpy(void *, const void *, int, size_t);
	extern void *memchr(const void *, int, size_t);
	extern int memcmp(const void *, const void *, size_t);
	extern void *memcpy (void *, const void *, size_t);
	extern void *memmove(void *, const void *, size_t);
	extern void *memset(void *, int, size_t);
	extern char *strcat(char *, const char *);
	extern char *strchr(const char *, int);
	extern int strcmp(const char *, const char *);
	extern char *strcpy(char *, const char *);
	extern size_t strcspn(const char *, const char *);
	extern char *strdup(const char *);
	extern char *strerror(int);
	extern size_t strlen(const char *);
	extern char *strncat(char *, const char *, size_t);
	extern int strncmp(const char *, const char *, size_t);
	extern char *strncpy(char *, const char *, size_t);
	extern size_t strnlen(const char *, size_t);
	extern char *strpbrk(const char *, const char *);
	extern char *strrchr(const char *, int);
	extern size_t strspn(const char *, const char *);
	extern char *strstr(const char *, const char *);
	extern char *strtok(char *, const char *);
	extern size_t strxfrm(char *, const char *, size_t);
	
	/**@}*/

#endif /* STRING_H_ */

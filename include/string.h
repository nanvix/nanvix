/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <string.h> - String library.
 */

#ifndef STRING_H_
#define STRING_H_

	#include <sys/types.h>

	/*
	 * Copy bytes in memory.
	 */
	extern void *memcpy (void* dest, const void *src, size_t n);

	/*
	 * Schans for a character in a string.
	 */
	extern char *strchr(const char *str, int c);

	/*
	 * Compares two strings.
	 */
	extern int strcmp(const char *str1, const char *str2);
	
	/*
	 * Compares two strings.
	 */
	extern int strncmp(const char *str1, const char *str2, size_t n);

	/*
	 * Returns the length of a string.
	 */
	extern size_t strlen(const char * str);

#endif /* STRING_H_ */

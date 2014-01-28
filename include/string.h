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
	 * Concatenates two strings.
	 */
	extern char *strcat(char *dest, const char *src);

	/*
	 * Copies a string.
	 */
	extern char *strcpy(char *dest, const char *src);

	/*
	 * Returns the length of a string.
	 */
	extern size_t strlen(const char * str);
	
	/*
	 * Gets the length of a fixed-size string.
	 */
	extern size_t strnlen(const char *str, size_t maxlen);

#endif /* STRING_H_ */

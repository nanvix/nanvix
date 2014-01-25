/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <string.h> - String library.
 */

#ifndef STRING_H_
#define STRING_H_

	#include <sys/types.h>

	/*
	 * Schans for a character in a string.
	 */
	extern char *strchr(const char *str, int c);

	/*
	 * Returns the length of a string.
	 */
	extern size_t strlen(const char * str);

#endif /* STRING_H_ */

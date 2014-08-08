/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <builtin.h> - Built-in commands library.
 */

#ifndef _BUILTIN_
#define _BUILTIN_

	/*
	 * Built-in command signature.
	 */
	typedef int (*builtin_t)(int, const char**);
	
	/*
	 * Gets the requested built-in command.
	 */
	builtin_t getbuiltin(const char *cmdname);

#endif /* _BUILTIN_ */

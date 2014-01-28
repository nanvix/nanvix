/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdlib/exit.c - exit() and atexit() implementation.
 */

#include <limits.h>
#include <stdlib.h>
#include <unistd.h>

/* Cleanup functions. */
extern void stdio_cleanup(void); /* stdio. */

/* atexit() function table. */
static void (*functab[ATEXIT_MAX])(void);

/* Number of functions registered. */
static int n = 0;

/*
 * Registers a function to run at process termination.
 */
int atexit(void(*func)(void))
{
	/* Function table is full. */
	if (n >= ATEXIT_MAX)
		return (-1);
	
	functab[n++] = func;
	
	return (0);
}

/*
 * Terminates the calling process.
 */
void exit(int status)
{
	/*
	 * Call registered atexit() functions
	 * in the reverse order in which they were
	 * registered.
	 */
	while (n-- > 0)
		functab[n]();
	
	stdio_cleanup();
	
	_exit(status);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdlib/exit.c - exit() implementation.
 */

#include <stdlib.h>
#include <unistd.h>

/*
 * Terminates the calling process.
 */
void exit(int status)
{
	_exit(status);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/iasatty.c - isatty() implementation.
 */

#include <termios.h>

/*
 * Tests whether a file descriptor refers to a terminal.
 */
int isatty(int fd)
{
	struct termios termios;
	
	return (tcgetattr(fd, &termios) == 0);
}

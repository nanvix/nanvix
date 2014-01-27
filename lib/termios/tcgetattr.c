/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * termios/tcgetattr.c - tcgetattr() implementation.
 */

#include <dev/tty.h>
#include <stropts.h>
#include <termios.h>

/*
 * Gets the parameters associated with the terminal
 */
int tcgetattr(int fd, struct termios *termiosp)
{
	return (ioctl(fd, TTY_GETS, termiosp));
}

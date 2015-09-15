/*
 * Copyright(C) 2015-2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * termios/tcsetattr.c - tcsetattr() implementation.
 */

#include <dev/tty.h>
#include <stropts.h>
#include <termios.h>

/*
 * Sets the parameters associated with the terminal
 */
int tcsetattr(int fd, int optional_actions, 
	const struct termios *termiosp)
{
	struct set_termios_attr sta;
	sta.optional_actions = optional_actions;
	sta.termiosp = termiosp;

	return (ioctl(fd, TTY_SETS, &sta));
}

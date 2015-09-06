/*
 * Copyleft(C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * sys/clear.c - clear() system call implementation.
 */

#include <nanvix/dev.h>
#include <dev/tty.h>

/*
 * Clear the screen
 */

PUBLIC int sys_clear()
{
	dev_t tty = DEVID(TTY_MAJOR, 0, CHRDEV);
	cdev_ioctl(tty, TTY_CLEAR, 0);
	return 0;
}

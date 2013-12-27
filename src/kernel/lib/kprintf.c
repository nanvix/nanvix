/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * kprintf.c - Kernel printf()
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <stdarg.h>

/* Kernel's output device. */
PRIVATE dev_t kout = DEVID(NULL_MAJOR, 0, CHRDEV);

/*
 * Change kernel's output device.
 */
PUBLIC void chkout(dev_t dev)
{	
	ssize_t n;
	char buffer[KBUFFER_SIZE];
	
	kout = dev;
	
	/* Flush the content of kernel log. */
	n = klog_read(buffer, KLOG_SIZE);
	
	cdev_write(kout, buffer, n);
}

/*
 * Writes on the screen a formated string.
 */
PUBLIC void kprintf(const char *fmt, ...)
{
	int i;
	va_list args;
	char buffer[KBUFFER_SIZE + 1];
	
	/* Convert to raw string. */
	va_start(args, fmt);
	i = kvsprintf(buffer, fmt, args);
	buffer[i++] = '\n';
	va_end(args);

	/* Save on kernel log and write on kout. */
	cdev_write(kout, buffer, i);
	klog_write(buffer, i);
}

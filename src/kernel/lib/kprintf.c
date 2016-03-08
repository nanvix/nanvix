/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <stdarg.h>

/**
 * @brief Kernel's output device.
 */
PUBLIC dev_t kout = DEVID(NULL_MAJOR, 0, CHRDEV);

/**
 * @brief Changes kernel's output device.
 * 
 * @param dev New output device.
 */
PUBLIC void chkout(dev_t dev)
{	
	ssize_t n;                 /* Number of bytes to be flushed. */
	char buffer[KBUFFER_SIZE]; /* Temporary buffer.              */
	
	kout = dev;
	
	/* Flush the content of kernel log. */
	n = klog_read(0, buffer, KLOG_SIZE);
	
	cdev_write(kout, buffer, n);
}

/**
 * @brief Writes on the screen a formated string.
 * 
 * @param fmt Formated string.
 */
PUBLIC void kprintf(const char *fmt, ...)
{
	int i;                         /* Loop index.              */
	va_list args;                  /* Variable arguments list. */
	char buffer[KBUFFER_SIZE + 1]; /* Temporary buffer.        */
	
	/* Convert to raw string. */
	va_start(args, fmt);
	i = kvsprintf(buffer, fmt, args);
	buffer[i++] = '\n';
	va_end(args);

	/* Save on kernel log and write on kout. */
	cdev_write(kout, buffer, i);
	klog_write(0, buffer, i);
}

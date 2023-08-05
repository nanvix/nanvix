/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
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
 * @brief Skip log_level from a buffer and update it's size
 *
 * @return Buffer without log_level code at the beginning
 */
PUBLIC const char *skip_code(const char *buffer, int *i)
{
	if (get_code(buffer))
	{
		*i = *i-2;
		return buffer+2;
	}
	return buffer;
}

/**
 * @brief Get log_level from a buffer
 *
 * @return log_level or 0 if default log_level
 */
PUBLIC char get_code(const char *buffer)
{
	if ((buffer[0] == KERN_SOH_ASCII) && !(buffer[1] == '\0'))
	{
		if ((buffer[1] >= '0' && buffer[1] <= '7'))
			return buffer[1];

		else
		{
			kpanic("log level error: invalid log level");
			return -1;
		}
	}
	return 0;
}

/**
 * @brief Writes on the screen a formated string.
 *
 * @param fmt Formated string.
 */
PUBLIC void kprintf(const char *fmt, ...)
{
	int i;                         /* Loop index.                              */
	va_list args;                  /* Variable arguments list.                 */
	char buffer[KBUFFER_SIZE + 1]; /* Temporary buffer.                        */
	const char *buffer_no_code;    /* Temporary buffer for log level printing. */

	/* Convert to raw string. */
	va_start(args, fmt);
	i = kvsprintf(buffer, fmt, args);
	buffer[i++] = '\n';
	va_end(args);

	/* Save on kernel log, skip code in case it's not correctly done and write on kout. */
	klog_write(0, buffer, i);
	buffer_no_code = skip_code(buffer, &i);
	cdev_write(kout, buffer_no_code, i);
}
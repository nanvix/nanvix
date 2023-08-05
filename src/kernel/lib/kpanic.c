/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <stdarg.h>

/**
 * @brief Writes a message to the kernel's output device and panics the kernel.
 *
 * @param fmt Formatted message to be written onto kernel's output device.
 */
PUBLIC void kpanic(const char *fmt, ...)
{
	int i;                         /* Loop index.              */
	va_list args;                  /* Variable arguments list. */
	char buffer[KBUFFER_SIZE + 1]; /* Temporary buffer.        */

	kstrncpy(buffer, "PANIC: ", 7);

	/* Convert to raw string. */
	va_start(args, fmt);
	i = kvsprintf(buffer + 7, fmt, args) + 7;
	buffer[i++] = '\n';
	va_end(args);

	/* Save on kernel log and write on kout. */
	cdev_write(kout, buffer, i);
	klog_write(0, buffer, i);

	/*
	 * Disable interrupts, so we cannot
	 * be bothered.
	 */
	disable_interrupts();

	while(1);
	halt();
}

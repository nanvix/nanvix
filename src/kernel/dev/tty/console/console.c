/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <nanvix/config.h>
#include <sys/types.h>
#include <stdint.h>
#include "../tty.h"

/**
 * Console modes table.
 */
PRIVATE const struct console_handler *chandler;

/*
 * Outputs a colored ASCII character on the console device.
 */
PUBLIC void console_put(uint8_t ch, uint32_t color)
{	
	chandler->c_put(ch, color);
}

/*
 * Clears the console.
 */
PUBLIC void console_clear(void)
{
	chandler->c_clear();
}

/*
 * Flushes a buffer on the console device.
 */
PUBLIC void console_write(struct kbuffer *buffer)
{
	chandler->c_write(buffer);
}

/*
 * Register a new console mode handler for the kernel.
 */
PUBLIC int console_register(struct console_handler *c)
{
	if (c != NULL)
	{
		chandler = c;
		return (0);
	}
	else
		return (-1);
}

/*
 * Initializes the console driver.
 */
PUBLIC void console_init(void)
{
	/* Choose an appropriate mode. */
	switch(VIDEO_MODE)
	{
		/* 80x25 mode. */
		case 0:
			ctextmode_init();
			break;

		/* VBE mode. */
		case 1:
			cvbemode_init();
			break;

		default:
			kpanic("console handler does not exist");
	}
}

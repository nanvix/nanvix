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

#ifndef _TTY_H_
#define _TTY_H_

	#include <nanvix/const.h>
	#include <nanvix/klib.h>
	#include <nanvix/pm.h>
	#include <termios.h>

	/* Console colors. */
    #define BLACK         0x00
    #define BLUE          0x01
    #define GREEN         0x02
    #define CYAN          0x03
    #define RED           0x04
    #define MAGNETA       0x05
    #define BROWN         0x06
    #define LIGHT_GREY    0x07
    #define DARK_GREY     0x08
    #define LIGHT_BLUE    0x09
    #define LIGHT_GREEN   0x0A
    #define LIGHT_CYAN    0x0B
    #define LIGHT_RED     0x0C
    #define LIGHT_MAGNETA 0x0D
    #define LIGHT_BROWN   0x0E
    #define WHITE         0x0F

	/**
	 * @brief TTY flags.
	 */
	enum tty_flags
	{
		TTY_STOPPED = (1 << 0) /**< Stopped? */
	};

	/**
	 * @brief TTY device.
	 */
	struct tty
	{
		enum tty_flags flags;  /**< Flags.               */
		struct termios term;   /**< Terminal I/O.        */
		struct process *pgrp;  /**< Process group.       */
		struct kbuffer output; /**< Output buffer.       */
		struct kbuffer rinput; /**< Cooked input buffer. */
		struct kbuffer cinput; /**< Raw input buffer.    */
	};

	/**
	 * @name TTY Functions
	 */
	/**@{*/
	EXTERN void tty_int(unsigned char);
	/**@}*/

	/**
	 * @name Console Functions
	 */
	/**@{*/
	EXTERN void console_put(uint8_t, uint8_t);
	EXTERN void console_init(void);
	EXTERN void console_clear(void);
	EXTERN void console_write(struct kbuffer *);
	/**@}*/

	/**
	 * @name Keyboard Functions
	 */
	/**@{*/
	EXTERN void keyboard_init(void);
	/**@}*/

#endif /* _TTY_H_ */

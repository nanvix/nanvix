/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef TTY_H_
#define TTY_H_

	/**
	 * @brief tty_ioctl() commands.
	 */
	/**@{*/
	#define TTY_CLEAR 0x54100000 /**< Clear console.    */
	#define TTY_GETS  0x54210000 /**< Get tty settings. */
 	#define TTY_SETS  0x54310000 /**< Set tty settings. */
	/**@}*/

	/* Forward definitions. */
	extern void tty_init(void);

#endif /* TTY_H_ */

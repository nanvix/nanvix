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

#ifndef _CONSOLE_H_
#define _CONSOLE_H_

/**
 * @brief Console handler.
 */
struct console_handler
{
	void (*c_put)(uint8_t,uint32_t);
	void (*c_clear)(void);
	void (*c_write)(struct kbuffer *);
};

/* Console handlers initializers. */
EXTERN void ctextmode_init(void);
EXTERN void cvbemode_init(void);

#endif /* _CONSOLE_H_ */

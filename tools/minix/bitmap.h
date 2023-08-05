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

#ifndef _BITMAP_H_
#define _BITMAP_H_

	#include <sys/types.h>
	#include <stdint.h>

	/**
	 * @brief Full bitmap.
	 */
	#define BITMAP_FULL 0xffffffff

	/**
	 * @name Bitmap Operators
	 */
	/**@{*/
	#define IDX(a) ((a) >> 5)   /**< Returns the index of the bit.  */
	#define OFF(a) ((a) & 0x1F) /**< Returns the offset of the bit. */
	/**@}*/

	/**
	 * @brief Sets a bit in a bitmap.
	 *
	 * @param bitmap Bitmap where the bit should be set.
	 * @param pos    Position of the bit that shall be set.
	 */
	#define bitmap_set(bitmap, pos) \
		(((uint32_t *)(bitmap))[IDX(pos)] |= (0x1 << OFF(pos)))

	/**
	 * @brief Clears a bit in a bitmap.
	 *
	 * @param bitmap Bitmap where the bit should be cleared.
	 * @param pos    Position of the bit that shall be cleared.
	 */
	#define bitmap_clear(bitmap, pos) \
		(((uint32_t *)(bitmap))[IDX(pos)] &= ~(0x1 << OFF(pos)))

	/**
	 * @name Bitmap Functions
	 */
	/**@{*/
	extern uint32_t bitmap_first_free(uint32_t *, size_t);
	/**@}*/

#endif /* BITMAP_H_ */

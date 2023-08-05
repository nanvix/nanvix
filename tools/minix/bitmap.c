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

#include <sys/types.h>
#include <stdint.h>

#include "bitmap.h"

/**
 * @brief Searches for the first free bit in a bitmap.
 *
 * @details Searches for the first free bit in a bitmap. In order to speedup
 *          computation, bits are checked in chunks of 4 bytes.
 *
 * @param bitmap Bitmap to be searched.
 * @param size   Size (in bytes) of the bitmap.
 *
 * @returns If a free bit is found, the number of that bit is returned. However,
 *          if no free bit is found #BITMAP_FULL is returned instead.
 */
uint32_t bitmap_first_free(uint32_t *bitmap, size_t size)
{
    uint32_t *max;          /* Bitmap bondary. */
    register uint32_t off;  /* Bit offset.     */
    register uint32_t *idx; /* Bit index.      */

    idx = bitmap;
    max = (idx + (size >> 2));

    /* Find bit index. */
    while (idx < max)
    {
		/* Index found. */
		if (*idx != 0xffffffff)
		{
			off = 0;

			/* Find offset. */
			while (*idx & (0x1 << off))
				off++;

			return (((idx - bitmap) << 5) + off);
		}

		idx++;
	}

	return (BITMAP_FULL);
}

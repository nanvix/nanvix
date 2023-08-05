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

#include <nanvix/klib.h>
#include <nanvix/const.h>

/**
 * @brief Returns the number of bits that are set in a bitmap.
 *
 * @details Counts the number of bits that are set in a bitmap using a
 *          bit-hacking algorithm from Stanford.
 *
 * @param bitmap Bitmap to be searched.
 * @param size   Size (in bytes) of the bitmap.
 *
 * @returns The number of bits that are set in the bitmap.
 */
PRIVATE unsigned bitmap_nset(uint32_t *bitmap, size_t size)
{
	unsigned count; /* Number of bits set. */
	uint32_t *idx;  /* Loop index.         */
	uint32_t *end;  /* End of bitmap.      */
	uint32_t chunk; /* Working chunk.      */

	/* Count the number of bits set. */
	count = 0;
	end = (bitmap + (size >> 2));
	for (idx = bitmap; idx < end; idx++)
	{
		chunk = *idx;

		/*
		 * Fast way for counting number of bits set in a bit map.
		 * I have no idea how does it work. I just got it from here:
		 * https://graphics.stanford.edu/~seander/bithacks.html
		 */
		chunk = chunk - ((chunk >> 1) & 0x55555555);
		chunk = (chunk & 0x33333333) + ((chunk >> 2) & 0x33333333);
		count += (((chunk + (chunk >> 4)) & 0x0F0F0F0F) * 0x01010101) >> 24;
	}

	return (count);
}

/**
 * @brief Returns the number of bits that are cleared in a bitmap.
 *
 * @details Counts the number of bits that are cleared in a bitmap using a
 *          bit-hacking algorithm from Stanford.
 *
 * @param bitmap Bitmap to be searched.
 * @param size   Size (in bytes) of the bitmap.
 *
 * @returns The number of bits that are cleared in the bitmap.
 */
PUBLIC unsigned bitmap_nclear(uint32_t *bitmap, size_t size)
{
	return ((size << 3) - bitmap_nset(bitmap, size));
}

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
PUBLIC bit_t bitmap_first_free(uint32_t *bitmap, size_t size)
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

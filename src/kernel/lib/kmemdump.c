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
#include <nanvix/klib.h>
#include <sys/types.h>

/**
 * @brief Dumps the contents of a memory area.
 *
 * @details Dumps the contents of the memory area pointed to by @p s.
 *
 * @param s Target memory area.
 * @param n Number of bytes to dump.
 */
PUBLIC void kmemdump(const void *s, size_t n)
{
	const unsigned *p  = s;

	/* Dump memory area in chunks. */
	for (size_t i = 0; i < n; i += 16, p += 4)
	{
		/* Do not print zero lines. */
		if (*(p + 0) || *(p + 1) || *(p + 2) || *(p + 3))
		{
			kprintf("[%x]: %x %x %x %x",
				i, *(p + 0), *(p + 1), *(p + 2), *(p + 3));
		}
	}
}

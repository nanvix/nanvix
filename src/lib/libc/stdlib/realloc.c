/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

#include <_ansi.h>
#include <reent.h>
#include <stdlib.h>
#include <malloc.h>

#ifndef _REENT_ONLY

/**
 * @brief Memory allocator.
 *
 * @details Deallocates the old object pointed to by @p ap
 * and returns a pointer to a new object that has the size
 * specified by @p nbytes. The contents of the new object is
 * the same as that of the old object prior to deallocation,
 * up to the lesser of the new and old sizes. Any bytes in the
 * new object beyond the size of the old object have indeterminate
 * values.
 *
 * @return Returns a pointer to the newly allocated memory, which
 * is suitably aligned for any kind of variable and may be different
 * from @p ap, or NULL if the request fails. If @p nbytes was equal 
 * to 0, either NULL or a pointer suitable to be passed to free() is
 * returned. If realloc() fails the original block is left untouched;
 * it is not freed or moved.
 */
void *realloc(void *ap, size_t nbytes)
{
  return _realloc_r (_REENT, ap, nbytes);
}

#endif

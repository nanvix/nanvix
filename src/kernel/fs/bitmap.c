/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/bitmap.c - Bitmap library implementation.
 */

#include <nanvix/const.h>
#include "fs.h"

/*
 * Finds the first free bit in a bitmap.
 */
PUBLIC bit_t bitmap_first_free(uint32_t *bitmap, size_t size)
{
    uint32_t *max;          /* Bitmap bondary. */
    register uint32_t off;  /* Bit offset.     */
    register uint32_t *idx; /* Bit index.      */
    
    idx = bitmap;
    max = (idx + (size >> 5));
    
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

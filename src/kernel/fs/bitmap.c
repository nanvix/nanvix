/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/bitmap.c - Bitmap library implementation.
 */

#include <nanvix/bitmap.h>
#include <nanvix/const.h>

/*
 * Returns the number of bits clear.
 */
PUBLIC int bitmap_nset(uint32_t *bitmap, size_t size)
{
	int count;      /* Number of bits set. */
	uint32_t *idx;  /* Loop index.         */
	uint32_t *end;  /* End of bitmap.      */
	uint32_t chunk; /* Working chunk.      */
	
	/* Count the number of bits set. */
	count = 0;
	end = (bitmap + (size >> 5));
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

/*
 * Returns the number of bits clear in a bitmap.
 */
PUBLIC int bitmap_nclear(uint32_t *bitmap, size_t size)
{
	return (size - bitmap_nset(bitmap, size));
}

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

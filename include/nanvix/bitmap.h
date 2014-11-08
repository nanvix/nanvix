/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <nanvix/bitmap.h> - Bitmap library.
 */

#ifndef BITMAP_H_
#define BITMAP_H_

	#include <nanvix/const.h>
	#include <sys/types.h>
	#include <stdint.h>

	/* Bit number. */
	typedef uint32_t bit_t;
	
	/* Bitmap is full. */
	#define BITMAP_FULL 0xffffffff
	
	/* Bitmap operators. */
	#define IDX(a) ((a) >> 5)   /* Returns the index of the bit.  */
	#define OFF(a) ((a) & 0x1F) /* Returns the offset of the bit. */
	
	/*
	 * Sets a bit in a bitmap.
	 */
	#define bitmap_set(block_map, pos) \
		(((uint32_t *)(block_map))[IDX(pos)] |= (0x1 << OFF(pos)))
	
	/*
	 * Clears a bit in a bitmap.
	 */
	#define bitmap_clear(block_map, pos) \
		(((uint32_t *)(block_map))[IDX(pos)] &= ~(0x1 << OFF(pos)))
		
	/*
	 * Finds the first free bit in a bitmap.
	 */
	EXTERN bit_t bitmap_first_free(uint32_t *bitmap, size_t size);
	
	/*
	 * Returns the number of bits clear in a bitmap.
	 */
	EXTERN unsigned bitmap_nclear(uint32_t *bitmap, size_t size);

#endif /* BITMAP_H_ */

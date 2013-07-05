/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * mm/mm.c - Memory system module.
 */

#include <nanvix/const.h>
#include <nanvix/region.h>

/*
 * Initializes the memory system module.
 */
PUBLIC void mm_init()
{
	/* Initialize memory regions. */
	initreg();
}

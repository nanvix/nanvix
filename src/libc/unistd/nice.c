/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd/nice.c> - nice() system call.
 */

#include <unistd.h>

/*
 * Changes the nice value of the calling process.
 */
int nice(int incr)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_nice),
		  "b" (incr)
	);
	
	return (ret);
}

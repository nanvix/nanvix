/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd/alarm.c> - alarm() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Schedules an alarm signal.
 */
unsigned alarm(unsigned seconds)
{
	unsigned ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_alarm),
		  "b" (seconds)
	);
	
	return (ret);
}

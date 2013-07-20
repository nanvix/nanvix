/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/alarm.c> - alarm() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/pm.h>

/*
 * Schedules an alarm signal.
 */
PUBLIC unsigned sys_alarm(unsigned seconds)
{
	unsigned oldalarm;
	
	oldalarm = curr_proc->alarm;
	
	/* Schedule alarm. */
	if (seconds > 0)
		curr_proc->alarm = ticks + seconds*CLOCK_FREQ;
		
	/* Cancel alarm. */
	else
		curr_proc->alarm = 0;
	
	/* Alarm would ring soon if we had not re-scheduled it. */
	if (oldalarm <= ticks)
		return (0);
	
	return ((oldalarm - ticks)/CLOCK_FREQ);
}

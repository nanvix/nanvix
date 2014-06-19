/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/times.c - times() system call implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/times.h>
#include <errno.h>

/*
 * Internal times().
 */
PRIVATE void do_times(struct tms *buffer)
{
	buffer->tms_utime = curr_proc->utime*CLOCK_FREQ;
	buffer->tms_stime = curr_proc->ktime*CLOCK_FREQ;
	buffer->tms_cutime = curr_proc->cutime*CLOCK_FREQ;
	buffer->tms_cstime = curr_proc->cktime*CLOCK_FREQ;
}

/*
 * Gets process and waited-for child process times.
 */
PUBLIC clock_t sys_times(struct tms *buffer)
{
	/* Not a valid buffer. */
	if (!chkmem(buffer, sizeof(struct tms), MAY_WRITE))
		return (-EINVAL);
	
	do_times(buffer);
	
	return (CURRENT_TIME*CLOCK_FREQ);
}

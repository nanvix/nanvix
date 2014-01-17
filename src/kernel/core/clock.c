/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * timer.c - Clock device driver.
 */

#include <asm/io.h>
#include <i386/i386.h>
#include <nanvix/const.h>
#include <nanvix/int.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/* Clock ticks since system initialization. */
PUBLIC unsigned ticks = 0;

/* Time at system startup. */
PUBLIC unsigned startup_time = 0;

/*
 * Handles a timer interrupt.
 */
PRIVATE void do_clock()
{
	ticks++;
	
	if (KERNEL_RUNNING(curr_proc))
	{
		curr_proc->ktime++;
		return;
	}
	
	curr_proc->utime++;
		
	/* Give up processor time. */
	if (--curr_proc->counter == 0)
		yield();
}

/*
 * Initializes the system's clock.
 */
PUBLIC void clock_init(unsigned freq)
{
	uint16_t freq_divisor;
	
	kprintf("dev: initializing clock device driver");
	
	set_hwint(INT_CLOCK, &do_clock);
	
	freq_divisor = PIT_FREQUENCY/freq;
	
	/* Send control byte: adjust frequency divisor. */
	outputb(PIT_CTRL, 0x36);
	
	/* Send data byte: divisor_low and divisor_high. */
	outputb(PIT_DATA, (byte_t)(freq_divisor & 0xff));
	outputb(PIT_DATA, (byte_t)((freq_divisor >> 8)));
}

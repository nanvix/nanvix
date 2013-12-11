/*
 * 
 */

#ifndef TIMER_H_
#define TIMER_H_

	#include <nanvix/const.h>
	
	/* Clock frequency (in hertz). */
	#define CLOCK_FREQ 100
	
	/* Current time. */
	#define CURRENT_TIME (startup_time + ticks/CLOCK_FREQ)

	/*
	 * Initializes the timer interrupt.
	 */
	EXTERN void clock_init(unsigned freq);

	/* Ticks since system initialization. */
	EXTERN unsigned ticks;
	
	/* Time at system startup. */
	EXTERN unsigned startup_time;
	
#endif /* TIMER_H_ */

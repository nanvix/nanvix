/*
 * 
 */

#ifndef TIMER_H_
#define TIMER_H_

	#include <nanvix/const.h>

	/*
	 * Initializes the timer interrupt.
	 */
	EXTERN void clock_init(unsigned freq);

	/* Ticks since system initialization. */
	EXTERN unsigned ticks;
	
#endif /* TIMER_H_ */

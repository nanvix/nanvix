/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * const.h - Kernel constants
 */

#ifndef CONST_H_
#define CONST_H_

	/* Scope constants. */
	#define PUBLIC         /* Global scope       */
	#define PRIVATE static /* File scope.        */
	#define EXTERN extern  /* Defined elsewhere. */

	/* Logic constants. */
	#define FALSE 0 /* False. */
	#define TRUE  1 /* True.  */
	
	/* Null pointer. */
	#define NULL ((void *)0)

#endif /* CONST_H_ */

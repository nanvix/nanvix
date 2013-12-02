/*
 * Copyright(C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * <dev/ramdisk.h> - RAM disk library.
 */

#ifndef RAMDISK_H_
#define RAMDISK_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>

	/*
	 * DESCRIPTION:
	 *   The ramdisk_init() function initializes the RAM disk device driver. 
	 * 
	 * RETURN VALUE:
	 *   The ramdisk_init() has no return value.
	 * 
	 * ERROS:
	 *   No errors are defined.
	 */
	EXTERN void ramdisk_init(void);

#endif /* RAMDISK_H_ */

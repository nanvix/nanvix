/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stropts.h> - Streams interface.
 */

#ifndef STROPTS_H_
#define STROPTS_H_

	/*
	 * Performs control operations on a device.
	 */
	extern int ioctl(int fd, int cmd, ...);

#endif /* STROPTS_H_ */

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stropts.h> - Streams interface.
 */

#ifndef STROPTS_H_
#define STROPTS_H_

	/**
	 * @brief Major command number in ioctl().
	 */
	#define IOCTL_MAJOR(x) (((x) >> 16) & 0xffff)
	
	/**
	 * @brief Minor command number in ioctl().
	 */
	#define IOCTL_MINOR(x) ((x) & 0xffff)

	/* Forward definitions. */
	extern int ioctl(int fd, int cmd, ...);

#endif /* STROPTS_H_ */

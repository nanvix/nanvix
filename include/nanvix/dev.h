/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dev.h - Uniform device interface
 */

#ifndef DEV_H_
#define DEV_H_

	#include <nanvix/const.h>
	#include <sys/types.h>

	/* Device types. */
	#define CHRDEV 1
	#define BLKDEV 2

	/*
	 * DESCRIPTION:
	 *   The MAJOR() macro returns the major number of a device.
	 */
	#define MAJOR(dev) \
		(((dev) >> 2) >> 4)
	
	/*
	 * DESCRIPTION:
	 *   The MINOR() macro returns the minor number of a device.
	 */
	#define MINOR(dev) \
		(((dev) >> 2) & 0xf)
	
	/*
	 * DESCRIPTION:
	 *   The DEVID() macro returns the device number of a device.
	 */
	#define DEVID(major, minor, type) \
		(((((major) << 4) | (minor)) << 2) | (type))
		
	/*
	 * DESCRIPTION:
	 *   The dev_init() function initializes the device drivers.
	 * 
	 * RETURN VALUE:
	 *   The dev_init() has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void dev_init();

	/*========================================================================*
	 *                             character device                           *
	 *========================================================================*/
	
	/* Character device major numbers. */
	#define NULL_MAJOR 0x0 /* Null device. */
	#define TTY_MAJOR  0x1 /* tty device.  */
	
	/*
	 * Character device.
	 */
	struct cdev
	{
		ssize_t (*read)(unsigned, char *, size_t);        /* Read.  */
		ssize_t (*write)(unsigned, const char *, size_t); /* Write. */
	};
	
	/*
	 * DESCRIPTION:
	 *   The cdev_register() function attempts to register a character device
	 *   with major number major.
	 * 
	 * RETURN VALUE:
	 *   Upon successfull completion, the cdev_register() function returns 0. 
	 *   Upon failure, a negative error code is returned.
	 * 
	 * ERRORS:
	 *   - EINVAL: invalid major number.
	 *   - EBUSY: there is a device registered with the same major number.
	 */
	EXTERN int cdev_register(unsigned major, const struct cdev *dev);
	
	/*
	 * DESCRIPTION:
	 *   The cdev_write() function attempts to write n bytes from the buffer 
	 *   pointed to by buf to the character device identified by dev.
	 * 
	 * RETURN VALUE:
	 *   Upon successfull completion, the cdev_write() function returns the 
	 *   number of bytes actually written to the device. Upon failure, a 
	 *   negative error code is returned.
	 * 
	 * ERRORS:
	 *   - EINVAL: invalid device.
	 */
	EXTERN ssize_t cdev_write(dev_t dev, const void *buf, size_t n);
	
	/*
	 * DESCRIPTION:
	 *   The cdev_read() function attempts to read n bytes from the device 
	 *   identified by dev to the buffer pointed to by buf.
	 * 
	 * RETURN VALUE:
	 *   Upon successfull completion, the cdev_read() function returns the 
	 *   number of bytes actually read from the device. Upon failure, a 
	 *   negative error code is returned.
	 * 
	 * ERRORS:
	 *   - EINVAL: invalid device.
	 */
	EXTERN ssize_t cdev_read(dev_t, void *buf, size_t);
	
	/*========================================================================*
	 *                               block device                             *
	 *========================================================================*/
	
	/*
	 * Block device.
	 */
	struct bdev
	{
		ssize_t (*read)(unsigned, char *, size_t, off_t);        /* Read.  */
		ssize_t (*write)(unsigned, const char *, size_t, off_t); /* Write. */
	};
	
#endif /* DEV_H_ */

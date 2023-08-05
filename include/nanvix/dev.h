/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef DEV_H_
#define DEV_H_

	#include <nanvix/const.h>
	#include <nanvix/fs.h>
	#include <sys/types.h>

	/* Device types. */
	#define CHRDEV 0 /* Charecter device. */
	#define BLKDEV 1 /* Block device.     */

	/*
	 * DESCRIPTION:
	 *   The MAJOR() macro returns the major number of a device.
	 */
	#define MAJOR(dev) \
		(((dev) >> 8) & 0xf)

	/*
	 * DESCRIPTION:
	 *   The MINOR() macro returns the minor number of a device.
	 */
	#define MINOR(dev) \
		(((dev) >> 4) & 0xf)

	/*
	 * DESCRIPTION:
	 *   The DEVID() macro returns the device number of a device.
	 */
	#define DEVID(major, minor, type) \
		(((major) << 8) | ((minor) << 4) | (type))

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
	EXTERN void dev_init(void);

	/*========================================================================*
	 *                             character device                           *
	 *========================================================================*/

	/* Character device major numbers. */
	#define NULL_MAJOR 0x0 /* Null device.       */
	#define TTY_MAJOR  0x1 /* tty device.        */
	#define KLOG_MAJOR 0x2 /* kernel log device. */

	/* NULL device. */
	#define NULL_DEV DEVID(NULL_MAJOR, 0, CHRDEV)

	/**
	 * @brief Character device.
	 */
	struct cdev
	{
		int (*open)(unsigned);                            /* Open.    */
		ssize_t (*read)(unsigned, char *, size_t);        /* Read.    */
		ssize_t (*write)(unsigned, const char *, size_t); /* Write.   */
		int (*ioctl)(unsigned, unsigned, unsigned);       /* Control. */
		int (*close)(dev_t);                              /* Close.   */
	};

	EXTERN int cdev_register(unsigned, const struct cdev *);
	EXTERN ssize_t cdev_write(dev_t, const void *, size_t);

	/*
	 * DESCRIPTION:
	 *   The cdev_read() function attempts to read n bytes from the character
	 *   device identified by dev to the buffer pointed to by buf.
	 *
	 * RETURN VALUE:
	 *   Upon successfull completion, the cdev_read() function returns the
	 *   number of bytes actually read from the device. Upon failure, a
	 *   negative error code is returned.
	 *
	 * ERRORS:
	 *   - EINVAL: invalid character device.
	 *   - ENOTSUP: operation not supported.
	 */
	EXTERN ssize_t cdev_read(dev_t dev, void *buf, size_t n);

	/* Forward definitions. */
	EXTERN int cdev_open(dev_t);
	EXTERN int cdev_close(dev_t);

	/*
	 * Performs control operations on a character device.
	 */
	EXTERN int cdev_ioctl(dev_t dev, unsigned cmd, unsigned arg);

	/*========================================================================*
	 *                               block device                             *
	 *========================================================================*/

	/* Block device major numbers. */
	#define RAMDISK_MAJOR 0x0 /* ramdisk device */
	#define ATA_MAJOR     0x1 /* ATA device     */

	/*
	 * Block device.
	 */
	struct bdev
	{
		ssize_t (*read)(dev_t, char *, size_t, off_t);        /* Read.        */
		ssize_t (*write)(dev_t, const char *, size_t, off_t); /* Write.       */
		int (*readblk)(unsigned, struct buffer *);            /* Read block.  */
		int (*writeblk)(unsigned, struct buffer *);           /* Write block. */
	};

	/*
	 * DESCRIPTION:
	 *   The bdev_register() function attempts to register a block device
	 *   with major number major.
	 *
	 * RETURN VALUE:
	 *   Upon successful completion, the bdev_register() function returns 0.
	 *   Upon failure, a negative error code is returned.
	 *
	 * ERRORS:
	 *   - EINVAL: invalid major number.
	 *   - EBUSY: there is a device registered with the same major number.
	 */
	EXTERN int bdev_register(unsigned major, const struct bdev *dev);

	/*
	 * DESCRIPTION:
	 *   The bdev_write() function attempts to write n bytes from the buffer
	 *   pointed to by buf to the block device identified by dev, starting at
	 *   offset off.
	 *
	 * RETURN VALUE:
	 *   Upon successful completion, the bdev_write() function returns the
	 *   number of bytes actually written to the device. Upon failure, a
	 *   negative error code is returned.
	 *
	 * ERRORS:
	 *   - EINVAL: invalid block device.
	 *   - ENOTSUP: operation not supported.
	 */
	EXTERN ssize_t bdev_write(dev_t dev, const char *buf, size_t n, off_t off);

	/*
	 * DESCRIPTION:
	 *   The bdev_read() function attempts to read n bytes from the block
	 *   device identified by dev to the buffer pointed to by buf, starting at
	 *   offset off.
	 *
	 * RETURN VALUE:
	 *   Upon successfull completion, the bdev_read() function returns the
	 *   number of bytes actually read from the device. Upon failure, a
	 *   negative error code is returned.
	 *
	 * ERRORS:
	 *   - EINVAL: invalid block device.
	 *   - ENOTSUP: operation not supported.
	 */
	EXTERN ssize_t bdev_read(dev_t dev, char *buf, size_t n, off_t off);

	/*
	 * Writes a block to a block device.
	 */
	EXTERN void bdev_writeblk(struct buffer *buf);

	/*
	 * Reads a block from a block device.
	 */
	EXTERN void bdev_readblk(struct buffer *buf);

#endif /* DEV_H_ */

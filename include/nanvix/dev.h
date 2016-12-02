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

	/**
	 * @brief Device types
	 */
	/**@{*/
	#define CHRDEV 0 /**<  Charecter device. */
	#define BLKDEV 1 /**< Block device.      */
	/**@}*/

	/**
	 * @brief Returns the major number of a device.
	 */
	#define MAJOR(dev) \
		(((dev) >> 8) & 0xf)
	
	/**
	 * @brief Returns the minor number of a device.
	 */
	#define MINOR(dev) \
		(((dev) >> 4) & 0xf)
	
	/*
	 * @brief Returns the device number of a device.
	 */
	#define DEVID(major, minor, type) \
		(((major) << 8) | ((minor) << 4) | (type))
		
	/* Forward definitions. */
	EXTERN void dev_init(void);

	/*========================================================================*
	 *                             character device                           *
	 *========================================================================*/
	
	/**
	 * @brief Major numbers for character devices.
	 */
	/**@{*/
	#define NULL_MAJOR 0x0 /**< Null device.       */
	#define TTY_MAJOR  0x1 /**< TTY device.        */
	#define KLOG_MAJOR 0x2 /**< kernel log device. */
	/**@}*/
	
	/**
	 * @brief Null device.
	 */
	#define NULL_DEV DEVID(NULL_MAJOR, 0, CHRDEV)
	
	/**
	 * @brief Character device.
	 */
	struct cdev
	{
		int (*open)(unsigned);                            /**< Open.    */
		ssize_t (*read)(unsigned, char *, size_t);        /**< Read.    */
		ssize_t (*write)(unsigned, const char *, size_t); /**< Write.   */
		int (*ioctl)(unsigned, unsigned, unsigned);       /**< Control. */
		int (*close)(dev_t);                              /**< Close.   */
	};
	
	/* Forward definitions. */
	EXTERN int cdev_register(unsigned, const struct cdev *);
	EXTERN ssize_t cdev_write(dev_t, const void *, size_t);
	EXTERN ssize_t cdev_read(dev_t, void *, size_t);
	EXTERN int cdev_open(dev_t);
	EXTERN int cdev_close(dev_t);
	EXTERN int cdev_ioctl(dev_t, unsigned, unsigned);
	
	/*========================================================================*
	 *                               block device                             *
	 *========================================================================*/
	
	/**
	 * @brief Block major numbers.
	 */
	/**@{*/
	#define RAMDISK_MAJOR 0x0 /**M ramdisk device. */
	#define ATA_MAJOR     0x1 /**< ATA device.     */
	/**@}*/
	
	/**
	 * @brief Block device.
	 */
	struct bdev
	{
		ssize_t (*read)(dev_t, char *, size_t, off_t);        /**< Read.        */
		ssize_t (*write)(dev_t, const char *, size_t, off_t); /**< Write.       */
		int (*readblk)(unsigned, struct buffer *);            /**< Read block.  */
		int (*writeblk)(unsigned, struct buffer *);           /**< Write block. */
	};
	
	/* Forward definitions. */
	EXTERN int bdev_register(unsigned, const struct bdev *);
	EXTERN ssize_t bdev_write(dev_t, const char *, size_t, off_t);
	EXTERN ssize_t bdev_read(dev_t, char *, size_t, off_t);
	EXTERN void bdev_writeblk(struct buffer *);
	EXTERN void bdev_readblk(struct buffer *);
	
#endif /* DEV_H_ */

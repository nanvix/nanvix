/*
 * Copyright(C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dev/dev.c - Uniform device interface.
 */

#include <dev/ata.h>
#include <dev/tty.h>
#include <dev/ramdisk.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <errno.h>

/*============================================================================*
 *                            Character Devices                               *
 *============================================================================*/

/* Number of character devices. */
#define NR_CHRDEV 2

/*
 * Character devices table.
 */
PRIVATE const struct cdev *cdevsw[NR_CHRDEV] = {
	NULL, /* /dev/null */
	NULL  /* /dev/tty  */
};

/*
 * Registers a character device.
 */
PUBLIC int cdev_register(unsigned major, const struct cdev *dev)
{
	/* Invalid major number? */
	if (major >= NR_CHRDEV)
		return (-EINVAL);
	
	/* Device already registered? */
	if ((major == NULL_MAJOR) || (cdevsw[major] != NULL))
		return (-EBUSY);
	
	/* Register character device. */
	cdevsw[major] = dev;
	
	return (0);
}

/*
 * Writes to a character device.
 */
PUBLIC ssize_t cdev_write(dev_t dev, const void *buf, size_t n)
{	
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return ((ssize_t)n);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	/* Operation not supported. */
	if (cdevsw[MAJOR(dev)]->write == NULL)
		return (-ENOTSUP);
	
	return (cdevsw[MAJOR(dev)]->write(MINOR(dev), buf, n));
}

/*
 * Reads from a character device.
 */
PUBLIC ssize_t cdev_read(dev_t dev, void *buf, size_t n)
{
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return (0);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	/* Operation not supported. */
	if (cdevsw[MAJOR(dev)]->read == NULL)
		return (-ENOTSUP);
	
	return (cdevsw[MAJOR(dev)]->read(MINOR(dev), buf, n));
}

/*
 * Opens a character device.
 */
PUBLIC int cdev_open(dev_t dev)
{
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return (0);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	/* Operation not supported. */
	if (cdevsw[MAJOR(dev)]->open == NULL)
		return (-ENOTSUP);
		
	return (cdevsw[MAJOR(dev)]->open(MINOR(dev)));
}

/*
 * Performs control operations on a character device.
 */
PUBLIC int cdev_ioctl(dev_t dev, unsigned cmd, unsigned arg)
{	
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return (-ENODEV);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	/* Operation not supported. */
	if (cdevsw[MAJOR(dev)]->ioctl == NULL)
		return (-ENOTSUP);
		
	return (cdevsw[MAJOR(dev)]->ioctl(MINOR(dev), cmd, arg));
}

/*============================================================================*
 *                              Block Devices                                 *
 *============================================================================*/

/* Number of block devices. */
#define NR_BLKDEV 2

/*
 * Block devices table.
 */
PRIVATE const struct bdev *bdevsw[NR_BLKDEV] = {
	NULL, /* /dev/ramdisk */
	NULL  /* /dev/hdd     */
};

/*
 * Registers a block device.
 */
PUBLIC int bdev_register(unsigned major, const struct bdev *dev)
{
	/* Invalid major number? */
	if (major >= NR_BLKDEV)
		return (-EINVAL);
	
	/* Device already registered? */
	if (bdevsw[major] != NULL)
		return (-EBUSY);
	
	/* Register block device. */
	bdevsw[major] = dev;
	
	return (0);
}

/*
 * Writes to a block device.
 */
PUBLIC ssize_t bdev_write(dev_t dev, const char *buf, size_t n, off_t off)
{
	/* Invalid device. */
	if (bdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	/* Operation not supported. */
	if (bdevsw[MAJOR(dev)]->write == NULL)
		return (-ENOTSUP);
	
	return (bdevsw[MAJOR(dev)]->write(MINOR(dev), buf, n, off));
}

/*
 * Reads from a block device.
 */
PUBLIC ssize_t bdev_read(dev_t dev, char *buf, size_t n, off_t off)
{
	/* Invalid device. */
	if (bdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
		
	/* Operation not supported. */
	if (bdevsw[MAJOR(dev)]->read == NULL)
		return (-ENOTSUP);
	
	return (bdevsw[MAJOR(dev)]->read(MINOR(dev), buf, n, off));
}

/*
 * Writes a block to a block device.
 */
PUBLIC void bdev_writeblk(buffer_t buf)
{
	int err;
	
	/* Invalid device. */
	if (bdevsw[MAJOR(buffer_dev(buf))] == NULL)
		kpanic("writing block to invalid device");
		
	/* Operation not supported. */
	if (bdevsw[MAJOR(buffer_dev(buf))]->writeblk == NULL)
		kpanic("block device cannot write blocks");
	
	/*
	 * We don't have to write back the buffer
	 * to disk, so we just release it and we are done.
	 */
	if (!buffer_is_dirty(buf))
	{
		brelse(buf);
		return;
	}
		
	/*
	 * Write buffer to disk. The low-level I/O function shall
	 * clean the BUFFER_DIRTY flag and release the buffer.
	 */
	err = bdevsw[MAJOR(buffer_dev(buf))]->writeblk(MINOR(buffer_dev(buf)), buf);
	
	/* Failed to write. */
	if (err)
		kpanic("failed to write block to device");
}

/*
 * Reads a block from a block device.
 */
PUBLIC void bdev_readblk(buffer_t buf)
{
	int err;
	
	/* Invalid device. */
	if (bdevsw[MAJOR(buffer_dev(buf))] == NULL)
		kpanic("reading block from invalid device");
		
	/* Operation not supported. */
	if (bdevsw[MAJOR(buffer_dev(buf))]->readblk == NULL)
		kpanic("block device cannot read blocks");
		
	err = bdevsw[MAJOR(buffer_dev(buf))]->readblk(MINOR(buffer_dev(buf)), buf);
	
	if (err)
		kpanic("failed to read block from device");
	
	/* Update buffer flags. */
	buffer_valid(buf, 1);
	buffer_dirty(buf, 0);
}

/*============================================================================*
 *                                 Devices                                    *
 *============================================================================*/

/*
 * Initializes device drivers.
 */
PUBLIC void dev_init(void)
{
	ata_init();
	tty_init();
	ramdisk_init();
}

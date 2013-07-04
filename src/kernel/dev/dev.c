/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dev.c - Uniform device interface
 */

#include <dev/tty.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
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
 * Writes to character device.
 */
PUBLIC ssize_t cdev_write(dev_t dev, const void *buf, size_t n)
{	
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return ((ssize_t)n);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	return (cdevsw[MAJOR(dev)]->write(MINOR(dev), buf, n));
}

/*
 * Read from the character device.
 */
PUBLIC ssize_t cdev_read(dev_t dev, void *buf, size_t n)
{
	/* Null device. */
	if (MAJOR(dev) == NULL_MAJOR)
		return (0);
	
	/* Invalid device. */
	if (cdevsw[MAJOR(dev)] == NULL)
		return (-EINVAL);
	
	return (cdevsw[MAJOR(dev)]->read(MINOR(dev), buf, n));
}

/*
 * Initializes character device drivers.
 */
PUBLIC void dev_init()
{
	/* Initializes character devices. */
	tty_init();
}

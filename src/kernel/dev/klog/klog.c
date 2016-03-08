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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <sys/types.h>
	
/* Error checking. */
#if KLOG_SIZE > KBUFFER_SIZE
	#error "KLOG_SIZE must be smaller than or equal to KBUFFER_SIZE"
#endif

/**
 * @brief Kernel log.
 */
PRIVATE struct
{
	int head;               /**< First element in the buffer.  */
	int tail;               /**< Next free slot in the buffer. */
	char buffer[KLOG_SIZE]; /**< Ring buffer.                  */
} klog = { 0, 0, {0, }};

/**
 * @brief Writes to kernel log.
 * 
 * @param buffer Buffer to be written in the kernel log.
 * @param n      Number of characters to be written in the kernel log.
 * 
 * @returns The number of characters actually written to the kernel log.
 */
PUBLIC ssize_t klog_write(unsigned minor, const char *buffer, size_t n)
{
	int head;      /* Log head.        */
	int tail;      /* Log tail.        */
	const char *p; /* Writing pointer. */
	
	UNUSED(minor);
	
	p = buffer;
	
	/* Read pointers. */
	head = klog.head;
	tail = klog.tail;
	
	/* Copy data to ring buffer. */
	while (n-- > 0)
	{
		klog.buffer[tail] = *p++;
		
		tail = (tail + 1)&(KLOG_SIZE - 1);
		
		if (tail == head)
			head++;
	}
	
	/* Write back pointers. */
	klog.head = head;
	klog.tail = tail;
	
	return ((ssize_t)(p - buffer));
}

/**
 * @brief Reads from kernel log.
 * 
 * @param buffer Buffer where the kernel log should be read to.
 * @param n      Number of characters to read.
 * 
 * @returns The number of characters actually read from the kernel log. 
 */
PUBLIC ssize_t klog_read(unsigned minor, char *buffer, size_t n)
{
	int i;   /* Loop index.      */
	char *p; /* Reading pointer. */
	
	UNUSED(minor);
	
	p = buffer;
	
	i = klog.head;
	
	/* Copy data to ring buffer. */
	while ((n-- > 0) && (i != klog.tail))
	{
		*p++ = klog.buffer[i];
		
		i = (i + 1)&(KLOG_SIZE - 1);
	}
	
	return ((ssize_t)(p - buffer));
}

/**
 * @brief Dummy open() operation.
 */
PRIVATE int klog_open(unsigned minor)
{
	UNUSED(minor);
	
	return (0);
}

/**
 * @brief Dummy close() operation.
 */
PRIVATE int klog_close(unsigned minor)
{
	UNUSED(minor);
	
	return (0);
}

/**
 * @brief Kernel driver
 */
PRIVATE struct cdev klog_driver = {
	&klog_open,  /* open()  */
	&klog_read,  /* read()  */
	&klog_write, /* write() */
	NULL,        /* ioctl() */
	&klog_close  /* close() */
};

/**
 * @brief Initializes the kernel log driver.
 */
PUBLIC void klog_init(void)
{
	cdev_register(KLOG_MAJOR, &klog_driver);
}

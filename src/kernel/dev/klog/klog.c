/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <nanvix/syscall.h>
#include <sys/types.h>

/* Error checking. */
#if KLOG_SIZE > KBUFFER_SIZE
	#error "KLOG_SIZE must be smaller than or equal to KBUFFER_SIZE"
#endif

#define TL_CONST 3

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
 * @brief Add log level code (if present) to buffer and skip it
 *
 * @param buffer        Buffer to be written in the kernel log.
 * @param n             Pointer on the number of characters to be written in the kernel log. (for updating)
 * @param head, tail    Pointers on buffer head and tail
 * @param char_printed  Number of chaf added to buffer for log_level (0 or 3)

 * @returns Buffer with no code, hidden returns with pointers
 */
PRIVATE const char *print_code(const char *buffer, int *n, int *head, int *tail, int *char_printed)
{
	int i;
	char p[3];
	p[0] = get_code(buffer);
	p[1] = ':';
	p[2] = ' ';

	/* log level is default one? */
	if (p[0] == 0)
	{
		return buffer;
	}

	/* Copy data to ring buffer */
	for (i=0;i<3;i++)
	{
		klog.buffer[*tail] = p[i];
		*tail = (*tail + 1)&(KLOG_SIZE - 1);

		if (*tail == *head)
			*head = *head + 1;
	}
	/* 3 character had been added to buffer for log_level printing */
	*char_printed =+ 3;

	return skip_code(buffer,n);
}

/**
 * @brief Add clock ticks to klog.buffer
 *
 * @param head, tail    Pointers on buffer head and tail
 * @param char_printed  Number of chaf added to buffer for log_level (0 or (TL_CONST + size of int returned by sys_gticks()))
 */
PRIVATE void print_ticks(int *head, int *tail, int *char_printed)
{
	/* loop variables */
	int i,j;

	/* temporary buffers */
	char string_ticks[sizeof(int)*8 + 1] = "";
	char string_ticks_const[TL_CONST + 1] = " - ";
	char buffer[sizeof(int)*8 + TL_CONST + 1];

	/* compute clock ticks */
	int ticks_lenght = itoa(string_ticks,(unsigned)sys_gticks(),'d');

	/* put ticks in temporary buffer */
	for(j=0;j<ticks_lenght;j++)
	{
		buffer[j] = string_ticks[j];
	}

	/* put esthetic in temporary buffer */
	for(i=0;i<TL_CONST;i++)
	{
		buffer[i + ticks_lenght] = string_ticks_const[i];
	}

	/* Copy data to ring buffer */
	for (i=0;i<(ticks_lenght + TL_CONST);i++)
	{
		klog.buffer[*tail] = buffer[i];
		*tail = (*tail + 1)&(KLOG_SIZE - 1);

		if (*tail == *head)
			*head = *head + 1;
	}
	/* number of characters added to buffer for ticks printing */
	*char_printed =+ (ticks_lenght + TL_CONST);
}

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

	int lenght = (int) n;
	int char_printed = 0; /* Useful for returning size */

	UNUSED(minor);

	/* Read pointers. */
	head = klog.head;
	tail = klog.tail;

	/* If there is a log_level code, add it in buffer and skip it */
	p = print_code(buffer,&lenght,&head,&tail,&char_printed);

	/* if a code had been printed, then clock ticks since initialization are printed too */
	if (char_printed)
			print_ticks(&head,&tail,&char_printed);

	/* Copy data to ring buffer. */
	while (lenght-- > 0)
	{
		klog.buffer[tail] = *p++;

		tail = (tail + 1)&(KLOG_SIZE - 1);

		if (tail == head)
			head++;
	}

	/* Write back pointers. */
	klog.head = head;
	klog.tail = tail;

	return ((ssize_t)(char_printed + p - buffer));
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
	NULL,        /* write() */
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
/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * klog.c - Kernel log
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <sys/types.h>

/* Error-size checking. */
#if KLOG_SIZE > KBUFFER_SIZE
	#error "KLOG_SIZE must be smaller than or equal to KBUFFER_SIZE"
#endif

/*
 * Kernel log.
 */
PRIVATE struct
{
	int head;               /* First element in the buffer.  */
	int tail;               /* Next free slot in the buffer. */
	char buffer[KLOG_SIZE]; /* Ring buffer.                  */
} klog = { 0, 0, {0, }};

/*
 * Writes to kernel log.
 */
PUBLIC size_t klog_write(const char *buffer, size_t n)
{
	int head;
	int tail;
	const char *p;
	
	p = buffer;
	
	/* Read pointers. */
	head = klog.head;
	tail = klog.tail;
	
	/* Copy data to ring buffer. */
	while (n-- > 0)
	{
		klog.buffer[tail] = *p++;
		
		tail = (tail + 1)%KLOG_SIZE;
		
		if (tail == head)
			head++;
	}
	
	/* Write back pointers. */
	klog.head = head;
	klog.tail = tail;
	
	return ((ssize_t)(p - buffer));
}

/*
 * Reads from kernel log.
 */
PUBLIC size_t klog_read(char *buffer, size_t n)
{
	int i;
	char *p;
	
	p = buffer;
	
	i = klog.head;
	
	/* Copy data to ring buffer. */
	while ((n-- > 0) && (i != klog.tail))
	{
		*p++ = klog.buffer[i];
		
		i = (i + 1)%KLOG_SIZE;
	}
	
	return ((ssize_t)(p - buffer));
}

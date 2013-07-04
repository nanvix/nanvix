/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * klib.h - Kernel library
 */

#ifndef KLIB_H_
#define KLIB_H_

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>
	#include <stdarg.h>

	/*========================================================================*
	 *                               buffer                                   *
	 *========================================================================*/
	 
	/* Kernel buffer size (in bytes). */
	#define KBUFFER_SIZE 1024
	
	/*
	 * Kernel buffer.
	 */
	struct kbuffer
	{
		int head;                  /* First character in the buffer. */
		int tail;                  /* Next free slot in the buffer.  */
		char buffer[KBUFFER_SIZE]; /* Ring buffer.                   */
		struct process *chain;     /* Sleeping chain.                */
	};
	
	/*
	 * DESCRIPTION:
	 *   The KBUFFER_FULL() macro asserts if a kernel buffer is full.
	 */
	#define KBUFFER_FULL(b) \
		(((b.tail + 1)%KBUFFER_SIZE) == b.head)
	
	/*
	 * DESCRIPTION:
	 *   The KBUFFER_EMPTY(b) macro asserts if a kernel buffer is empty.
	 */
    #define KBUFFER_EMPTY(b) \
		(b.head == b.tail)
	
	/*
	 * DESCRIPTION:
	 *   The KBUFFER_PUT() macro puts a character in a kernel buffer.
	 */
    #define KBUFFER_PUT(b, ch) \
		{b.buffer[b.tail] = ch; b.tail = ((b.tail + 1)%KBUFFER_SIZE);}
		
	/*
	 * DESCRIPTION:
	 *   The KBUFFER_GET() macro gets a character from a kernel buffer.
	 */
    #define KBUFFER_GET(b, ch) \
		{ch = b.buffer[b.head]; b.head = ((b.head + 1)%KBUFFER_SIZE);}
	
	/*
	 * DESCRIPTION:
	 *   THe KBUFFER_INIT() macro initializes a kernel buffer.
	 */
	#define KBUFFER_INIT(b) \
		{b.head = 0, b.tail = 0, b.chain = NULL;}

	/*========================================================================*
	 *                               memory                                   *
	 *========================================================================*/

	/*
	 * DESCRIPTION:
	 *   The memset() function copies c (converted to an unsigned char) into 
	 *   each of the first n bytes of the object pointed to by ptr. 
	 * 
	 * RETURN VALUE:
	 *   The memset() function returns ptr. No return value is reserved to 
	 *   indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void *kmemset(void *ptr, int c, size_t n);
	
	/*
	 * DESCRIPTION:
	 *   The memcpy() function shall copy n bytes from the object pointed to by 
	 *   src into the object pointed to by dest. If copying takes place between 
	 *   objects that overlap, the behavior is undefined.
	 *
	 * RETURN VALUE:
	 *   The memcpy() function shall return dest. No return value is reserved 
	 *   to indicate an error.
	 *
	 * ERRORS:
	 *   No errors are defined.
	 */
	PUBLIC void* kmemcpy (void* dest, const void *src, size_t n);
	
	/*
	 * DESCRIPTION:
	 *   The CHKSIZE() macro checks if 'a' agrees on size with 'b'. If 'a' and 
	 *   'b' have the same size, the compilation proceeds as normal, if they 
	 *   don't, the compilation gets aborted.
	 */
	#define CHKSIZE(a, b) \
		((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))
	
	/*
	 * DESCRIPTION:
	 *   The ALIGN() macro aligns the value a to the boundary b.
	 */
	#define ALIGN(a, b) \
		 (((addr_t)(a) + ((b) - 1)) & ~((b) - 1))

	/*========================================================================*
	 *                            formatted output                            *
	 *========================================================================*/

	/*
	 * DESCRIPTION:
	 *   The kvsprintf() function writes formatted data from variable argument 
	 *   list to string.The following formats are accepted:
	 *      
	 *      %c - ASCII character.
	 *      %d - Unsigned decimal.
	 *      %x - Unsigned hexadecimal.
	 *      %s - Null-terminated string.
	 * 
	 * RETURN VALUE:
	 *   The kvsprintf() function returns number of characters actually written.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN int kvsprintf(char *str, const char *fmt, va_list args);
	
	/*
	 * DESCRIPTION:
	 *   The kprintf() function writes formatted data from variable argument 
	 *   list to kernel's output device.The following formats are accepted:
	 *      
	 *      %c - ASCII character.
	 *      %d - Unsigned decimal.
	 *      %x - Unsigned hexadecimal.
	 *      %s - Null-terminated string.
	 * 
	 * RETURN VALUE:
	 *   No return value is defined. 
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void kprintf(const char *fmt, ...);
	
	/*
	 * DESCRIPTION:
	 *   THe chkout() function changes the kernel's output device.
	 * 
	 * RETURN VALUE:
	 *   No return value is reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void chkout(dev_t dev);

	/*========================================================================*
	 *                           logging and debugging                        *
	 *========================================================================*/
	
	/* Kernel log size (in bytes). */
	#define KLOG_SIZE 1024

	/*
	 * DESCRIPTION:
	 *   The klog_write() function attempts to write n bytes from the buffer 
	 *   pointed to by buffer into the kernel log.
	 * 
	 * RETURN VALUE:
	 *   The klog_write() function returns the number of bytes actually written.
	 *   No return value is reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined. 
	 */
	EXTERN size_t klog_write(const char *buffer, size_t n);
	
	/*
	 * DESCRIPTION:
	 *   The klog_read() function attempts to read n bytes from the kernel log 
	 *   into the buffer pointed to by buffer.
	 * 
	 * RETURN VALUE:
	 *   The klog_read() function returns the number of bytes actually read. 
	 *   No return value is reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined. 
	 */
	EXTERN size_t klog_read(char *buffer, size_t n);
	
	/*
	 * DESCRIPTION:
	 *   The kpanic() function writes the message pointed to by msg to the
	 *   kernel's output device and panics the kernel.
	 *
	 * RETURN VALUE:
	 *   The kpanic() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void kpanic(const char *msg);

	/*========================================================================*
	 *                                  other                                 *
	 *========================================================================*/

	/*
	 * DESCRIPTION:
	 *   The UNUSED() macro says to the compiler that a variable is unused.
	 */
	#define UNUSED(a) ((void)a)

#endif /* KLIB_H_ */

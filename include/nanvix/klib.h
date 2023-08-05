/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>Joe Perches <joe@perches.com>
 *              2012-2016 Linus Torvalds <torvalds@linux-foundation.org>
 *              2012-2012 Kay Sievers <kay.sievers@vrfy.org>
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

/**
 * @file nanvix/klib.h
 *
 * @brief Kernel Library
 */

#ifndef NANVIX_KLIB_H_
#define NANVIX_KLIB_H_

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>
	#include <stdarg.h>
	#include <stdint.h>

	/*========================================================================*
	 *                                Bitmap                                  *
	 *========================================================================*/

	/**
	 * @brief Bit number.
	 */
	typedef uint32_t bit_t;

	/**
	 * @brief Full bitmap.
	 */
	#define BITMAP_FULL 0xffffffff

	/**
	 * @name Bitmap Operators
	 */
	/**@{*/
	#define IDX(a) ((a) >> 5)   /**< Returns the index of the bit.  */
	#define OFF(a) ((a) & 0x1F) /**< Returns the offset of the bit. */
	/**@}*/

	/**
	 * @brief Sets a bit in a bitmap.
	 *
	 * @param bitmap Bitmap where the bit should be set.
	 * @param pos    Position of the bit that shall be set.
	 */
	#define bitmap_set(bitmap, pos) \
		(((uint32_t *)(bitmap))[IDX(pos)] |= (0x1 << OFF(pos)))

	/**
	 * @brief Clears a bit in a bitmap.
	 *
	 * @param bitmap Bitmap where the bit should be cleared.
	 * @param pos    Position of the bit that shall be cleared.
	 */
	#define bitmap_clear(bitmap, pos) \
		(((uint32_t *)(bitmap))[IDX(pos)] &= ~(0x1 << OFF(pos)))

	/**
	 * @name Bitmap Functions
	 */
	/**@{*/
	EXTERN bit_t bitmap_first_free(uint32_t *, size_t);
	EXTERN unsigned bitmap_nclear(uint32_t *, size_t);
	/**@}*/

	/*========================================================================*
	 *                                buffer                                  *
	 *========================================================================*/

	/**
	 * @brief Kernel buffer size (in bytes).
	 *
	 * @note This should be 2^x.
	 */
	#define KBUFFER_SIZE 1024

	/**
	 * @param Kernel buffer.
	 */
	struct kbuffer
	{
		unsigned head;                      /**< First character in the buf. */
		unsigned tail;                      /**< Next free slot in the buf.  */
		unsigned char buffer[KBUFFER_SIZE]; /**< Ring buffer.                */
		struct process *chain;              /**< Sleeping chain.             */
	};

	/**
	 * @brief Asserts if a kernel buffer is full.
	 *
	 * @param b Buffer to be queried about.
	 *
	 * @returns True if the buffer is full, and false otherwise.
	 */
	#define KBUFFER_FULL(b) \
		((((b).tail + 1)&(KBUFFER_SIZE - 1)) == (b).head)

	/**
	 * @brief Asserts if a kernel buffer is empty.
	 *
	 * @param b Buffer to be queried about.
	 */
    #define KBUFFER_EMPTY(b) \
		((b).head == (b).tail)

	/**
	 * @brief Puts a character in a kernel buffer.
	 *
	 * @param b  Buffer where the character should be put.
	 * @param ch Character to be put in the buffer.
	 */
    #define KBUFFER_PUT(b, ch)                          \
	{                                                   \
		(b).buffer[(b).tail] = (ch);                    \
		(b).tail = (((b).tail + 1)&(KBUFFER_SIZE - 1)); \
	}                                                   \

	/**
	 * @brief Gets a character from a kernel buffer.
	 *
	 * @param b  Buffer from where the character must be retrieved.
	 * @param ch Store location for the retrieved character.
	 */
    #define KBUFFER_GET(b, ch)                          \
	{                                                   \
		(ch) = (b).buffer[b.head];                      \
		(b).head = (((b).head + 1)&(KBUFFER_SIZE - 1)); \
	}                                                   \

	/**
	 * @brief Initializes a kernel buffer.
	 *
	 * @param b Kernel buffer to be initialized.
	 */
	#define KBUFFER_INIT(b) \
	{                       \
		(b).head = 0;       \
		(b).tail = 0;       \
		(b).chain = NULL;   \
	}                       \

	/**
	 * @brief Takes a character out from a kernel buffer.
	 *
	 * @param b Kernel buffer from where the character must be taken out.
	 */
	#define KBUFFER_TAKEOUT(b)                          \
	{                                                   \
		(b).tail = (((b).tail - 1)&(KBUFFER_SIZE - 1)); \
	}                                                   \

	/*========================================================================*
	 *                              strings                                   *
	 *========================================================================*/

	/**
	 * @name String Functions
	 */
	/**@{*/
	EXTERN int kstrcmp(const char *, const char *);
	EXTERN int kstrncmp(const char *, const char *, size_t);
	EXTERN char *kstrcpy(char *, const char *);
	EXTERN char *kstrncpy(char *, const char *, size_t);
	EXTERN size_t kstrlen(const char *);
	/**@}*/

	/*========================================================================*
	 *                               memory                                   *
	 *========================================================================*/

	/**
	 * @name Memory Functions
	 */
	/**@{*/
	EXTERN void* kmemcpy(void *, const void *, size_t);
	EXTERN void *kmemset(void *, int, size_t);
	/**@}*/

	/**
	 * @brief Aligns a value on a boundary.
	 *
	 * @param x Value to be aligned.
	 * @param a Boundary.
	 *
	 * @returns Aligned value.
	 */
	#define ALIGN(x, a) \
		(((x) + ((a) - 1)) & ~((a) - 1))

	/**
	 * @brief Checks if 'a' agrees on size if 'b'
	 *
	 * @param a Probing size.
	 * @param b Control size.
	 *
	 * @returns Upon success, compilation proceeds as normal. Upon failure,
	 *          a compilation error is generated.
	 */
	#define CHKSIZE(a, b) \
		((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))

	/*========================================================================*
	 *                            formatted output                            *
	 *========================================================================*/

	/* Forward definitions. */
	EXTERN dev_t kout;

	/**
	 * @name Formated Output Functions
	 */
	/**@{*/
	EXTERN int kvsprintf(char *, const char *, va_list);
	PUBLIC int itoa(char *str, unsigned num, int base);
	EXTERN void chkout(dev_t);
	EXTERN void kprintf(const char *, ...);
	/**@}*/

	/*========================================================================*
	 *                           logging and debugging                        *
	 *========================================================================*/
	/* This char declares that following one is a log level */
	#define KERN_SOH        "\001"
	#define KERN_SOH_ASCII  '\001'

	/* Log levels */
	#define KERN_EMERG      KERN_SOH "0"    /* system is unusable */
	#define KERN_ALERT      KERN_SOH "1"    /* action must be taken immediately */
	#define KERN_CRIT       KERN_SOH "2"    /* critical conditions */
	#define KERN_ERR        KERN_SOH "3"    /* error conditions */
	#define KERN_WARNING    KERN_SOH "4"    /* warning conditions */
	#define KERN_NOTICE     KERN_SOH "5"    /* normal but significant condition */
	#define KERN_INFO       KERN_SOH "6"    /* informational */
	#define KERN_DEBUG      KERN_SOH "7"    /* debug-level messages */

	/* default log level is no log level,
	   useful to avoid printing log level in early boot */

	/**
	 * @brief Kernel log size (in characters).
	 *
	 * @note This should be 2^x.
	 */
	#define KLOG_SIZE 1024

	/**
	 * @name Logging and Debugging Functions
	 */
	/**@{*/
	EXTERN ssize_t klog_read(unsigned, char *, size_t);
	EXTERN ssize_t klog_write(unsigned, const char *, size_t);
	EXTERN void kpanic(const char *, ...);
	EXTERN void kmemdump(const void *s, size_t n);
	EXTERN const char *skip_code(const char *buffer, int *i);
	EXTERN char get_code(const char *buffer);
	/**@}*/

	/*========================================================================*
	 *                                  other                                 *
	 *========================================================================*/

	/**
	 * @brief Declares something to be unused.
	 *
	 * @param a Thing.
	 */
	#define UNUSED(a) ((void)a)

	/**
	 * @brief No operation.
	 */
	#define noop()

	/**
	 * @name Random Functions
	 */
	EXTERN int krand(void);
	EXTERN void ksrand(unsigned);

#endif /* NANVIX_KLIB_H_ */

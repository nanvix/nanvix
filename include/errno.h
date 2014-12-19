/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/**
 * @file
 * 
 * @brief System Error Numbers
 */

#ifndef ERRNO_H_
#define ERRNO_H_
	
	/**
	 * @brief Error codes. 
	 */
	#define EAGAIN        1 /**< Resource unavailable, try again. */
	#define EBUSY         2 /**< Device or resource busy.         */
	#define ECHILD        3 /**< No child processes.              */
	#define EINTR         4 /**< Interrupted function.            */
	#define EINVAL        5 /**< Invalid argument.                */
	#define ENOMEM        6 /**< Not enough space.                */
	#define ENOSYS        7 /**< Function not supported.          */
	#define EPERM         8 /**< Operation not permitted.         */
	#define ESRCH         9 /**< No such process.                 */
	#define ENOTSUP      10 /**< Not supported.                   */
	#define ENAMETOOLONG 11 /**< Filename too long.               */
	#define ENOTDIR      12 /**< Not a directory.                 */
	#define ENOENT       13 /**< No such file or directory.       */
	#define EACCES       14 /**< Permission denied.               */
	#define EMFILE       15 /**< Too many open files.             */
	#define ENFILE       16 /**< Too many files open in system.   */
	#define EEXIST       17 /**< File exists.                     */
	#define ENOSPC       18 /**< No space left on device.         */
	#define EISDIR       19 /**< Is a directory.                  */
	#define EBADF        20 /**< Bad file descriptor.             */
	#define EFBIG        21 /**< File too large.                  */
	#define ENOEXEC      22 /**< Executable file format error.    */
	#define E2BIG        23 /**< Argument list too long.          */
	#define EFAULT       24 /**< Bad address.                     */
	#define ESPIPE       25 /**< Invalid seek.                    */
	#define EPIPE        26 /**< Broken pipe.                     */
	#define ENODEV       27 /**< No such device.                  */
	#define ERANGE       28 /**< Result too large.                */
	#define EMLINK       29 /**< Too many links.                  */
	#define EIO          30 /**< I/O error.                       */

#ifndef _ASM_FILE_

	/* Forward definitions. */
	extern int errno;

#endif /* _ASM_FILE_ */

#endif /* ERRNO_H_ */

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

/**
 * @file
 *
 * @brief System Error Numbers
 */

#ifndef ERRNO_H_
#define ERRNO_H_

	/**
	 * @defgroup errnolib Errno Library
	 *
	 * @brief System error numbers.
	 */
	/**@{*/

	/**
	 * @name Error Codes
	 */
	/**@{*/
	#define EAGAIN           1 /**< Resource unavailable, try again.                */
	#define EBUSY            2 /**< Device or resource busy.                        */
	#define ECHILD           3 /**< No child processes.                             */
	#define EINTR            4 /**< Interrupted function.                           */
	#define EINVAL           5 /**< Invalid argument.                               */
	#define ENOMEM           6 /**< Not enough space.                               */
	#define ENOSYS           7 /**< Function not supported.                         */
	#define EPERM            8 /**< Operation not permitted.                        */
	#define ESRCH            9 /**< No such process.                                */
	#define ENOTSUP         10 /**< Not supported.                                  */
	#define ENAMETOOLONG    11 /**< Filename too long.                              */
	#define ENOTDIR         12 /**< Not a directory.                                */
	#define ENOENT          13 /**< No such file or directory.                      */
	#define EACCES          14 /**< Permission denied.                              */
	#define EMFILE          15 /**< Too many open files.                            */
	#define ENFILE          16 /**< Too many files open in system.                  */
	#define EEXIST          17 /**< File exists.                                    */
	#define ENOSPC          18 /**< No space left on device.                        */
	#define EISDIR          19 /**< Is a directory.                                 */
	#define EBADF           20 /**< Bad file descriptor.                            */
	#define EFBIG           21 /**< File too large.                                 */
	#define ENOEXEC         22 /**< Executable file format error.                   */
	#define E2BIG           23 /**< Argument list too long.                         */
	#define EFAULT          24 /**< Bad address.                                    */
	#define ESPIPE          25 /**< Invalid seek.                                   */
	#define EPIPE           26 /**< Broken pipe.                                    */
	#define ENODEV          27 /**< No such device.                                 */
	#define ERANGE          28 /**< Result too large.                               */
	#define EMLINK          29 /**< Too many links.                                 */
	#define EIO             30 /**< I/O error.                                      */
	#define ENXIO           31 /**< No such device or address.                      */
	#define EXDEV           32 /**< Cross-device link.                              */
	#define ENOTTY          33 /**< Inappropriate I/O control operation.            */
	#define ETXTBSY         34 /**< Text file busy.                                 */
	#define EROFS           35 /**< Read-only file system.                          */
	#define EDOM            36 /**< Mathematics argument out of domain of function. */
	#define ENOMSG          37 /**< No message of the desired type.                 */
	#define EIDRM           38 /**< Identifier removed.                             */
	#define EDEADLK         39 /**< Resource deadlock would occur.                  */
	#define ENOLCK          40 /**< No locks available.                             */
	#define ENOLINK         41 /**< Virtual circuit is gone.                        */
	#define EPROTO          42 /**< Protocol error.                                 */
	#define EMULTIHOP       43 /**< Multihop attempted.                             */
	#define EBADMSG         44 /**< Bad message.                                    */
	#define EADDRINUSE      45 /**< Address in use.                                 */
	#define EADDRNOTAVAIL   46 /**< Address not available.                          */
	#define EAFNOSUPPORT    47 /**< Address family not supported.                   */
	#define EALREADY        48 /**< Connection already in progress.                 */
	#define ECANCELED       49 /**< Operation canceled.                             */
	#define ECONNABORTED    50 /**< Connection aborted.                             */
	#define ECONNREFUSED    51 /**< Connection refused.                             */
	#define ECONNRESET      52 /**< Connection reset.                               */
	#define EDESTADDRREQ    53 /**< Destination address required.                   */
	#define EDQUOT          54 /**< Quota exceeded.                                 */
	#define EHOSTUNREACH    55 /**< Host is unreachable.                            */
	#define EILSEQ          56 /**< Illegal byte sequence.                          */
	#define EINPROGRESS     57 /**< Operation in progress.                          */
	#define EISCONN         58 /**< Socket is connected.                            */
	#define ELOOP           59 /**< Too many links of symbolic links.               */
	#define EMSGSIZE        60 /**< Message too large.                              */
	#define ENETDOWN        61 /**< Network is down.                                */
	#define ENETRESET       62 /**< Connection aborted by network.                  */
	#define ENETUNREACH     63 /**< Network unreachable.                            */
	#define ENOBUFS         64 /**< No buffer space is available.                   */
	#define ENOPROTOOPT     65 /**< Protocol not available.                         */
	#define ENOTCONN        66 /**< The socket is not connected.                    */
	#define ENOTEMPTY       67 /**< Directory not empty.                            */
	#define ENOTSOCK        69 /**< Not a socket.                                   */
	#define EOPNOTSUPP      70 /**< Operation not supported on socket.              */
	#define EOVERFLOW       71 /**< value too large to be stored in data type.      */
	#define EPROTONOSUPPORT 72 /**< Protocol not supported.                         */
	#define EPROTOTYPE      73 /**< Protocol wrong type for socket.                 */
	#define ETIMEDOUT       74 /**< Connection timed out.                           */
	#define EWOULDBLOCK     75 /**< Operation would block.                          */
	#define ENOTRECOVERABLE 76 /**< State not recoverable.                          */
	#define EOWNERDEAD      77 /**< Previous owner died.                            */
	#define ESTALE          78 /**< Stale NFS file handle.                          */

#if defined(_XOPEN_SOURCE)
	#define ENODATA 79 /**< No message is available on the stream head read queue. */
	#define ENOSR   80 /**< No stream resources.                                   */
	#define ENOSTR  81 /**< Not a stream.                                          */
	#define ETIME   82 /**< Stream ioctl() timeout.                                */
#endif
	/**@}*/

	/**@}*/

#ifndef _ASM_FILE_

	/**
	 * @addtogroup errnolib
	 */
	/**@{*/

	/* Forward definitions. */
	extern int errno;

	/**@}*/

#endif /* _ASM_FILE_ */

#endif /* ERRNO_H_ */

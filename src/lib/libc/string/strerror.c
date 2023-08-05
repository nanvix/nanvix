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

/*
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/**
 * @file
 *
 * @brief strerror() implementation.
 */

#include <errno.h>

/**
 * @brief Gets error message string.
 *
 * @returns A pointer to the generated message string.
 *
 * @todo Use collating information.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
char *strerror(int errnum)
{
	char *error;

	switch (errnum)
	{
		case 0:
			error = "No error";
			break;
		case EPERM:
			error = "Operation not permitted";
			break;
		case ENOENT:
			error = "No such file or directory";
			break;
		case ESRCH:
			error = "No such process";
			break;
		case EINTR:
			error = "Interrupted function";
			break;
		case EIO:
			error = "I/O error";
			break;
		case ENXIO:
			error = "No such device or address";
			break;
		case E2BIG:
			error = "Argument list too long";
			break;
		case ENOEXEC:
			error = "Exec format error";
			break;
		case EBADF:
			error = "Bad file number";
			break;
		case ECHILD:
			error = "No children";
			break;
		case EAGAIN:
			error = "No more processes";
			break;
		case ENOMEM:
			error = "Not enough space";
			break;
		case EACCES:
			error = "Permission denied";
			break;
		case EFAULT:
			error = "Bad address";
			break;
		case EBUSY:
			error = "Device or resource busy";
			break;
		case EEXIST:
			error = "File exists";
			break;
		case EXDEV:
			error = "Cross-device link";
			break;
		case ENODEV:
			error = "No such device";
			break;
		case ENOTDIR:
			error = "Not a directory";
			break;
		case EISDIR:
			error = "Is a directory";
			break;
		case EINVAL:
			error = "Invalid argument";
			break;
		case ENFILE:
			error = "File table overflow";
			break;
		case EMFILE:
			error = "Too many open files";
			break;
		case ENOTTY:
			error = "Not a character device";
			break;
		case ETXTBSY:
			error = "Text file busy";
			break;
		case EFBIG:
			error = "File too large";
			break;
		case ENOSPC:
			error = "No space left on device";
			break;
		case ESPIPE:
			error = "Invalid seek";
			break;
		case EROFS:
			error = "Read-only file system";
			break;
		case EMLINK:
			error = "Too many links";
			break;
		case EPIPE:
			error = "Broken pipe";
			break;
		case EDOM:
			error = "Math argument";
			break;
		case ERANGE:
			error = "Result too large";
			break;
		case ENOMSG:
			error = "No message of desired type";
			break;
		case EIDRM:
			error = "Identifier removed";
			break;
		case EDEADLK:
			error = "Deadlock";
			break;
		case ENOLCK:
			error = "No lock";
			break;
		case ENOLINK:
			error = "Virtual circuit is gone";
			break;
		case EPROTO:
			error = "Protocol error";
			break;
		case EMULTIHOP:
			error = "Multihop attempted";
			break;
		case EBADMSG:
			error = "Bad message";
			break;
		case EADDRINUSE:
			error = "Address in use";
			break;
		case EADDRNOTAVAIL:
			error = "Address not available";
			break;
		case EAFNOSUPPORT:
			error = "Address family not supported";
			break;
		case EALREADY:
			error = "Connection already in progress";
			break;
		case ECANCELED:
			error = "Operation canceled";
			break;
		case ECONNABORTED:
			error = "Connection aborted";
			break;
		case ECONNREFUSED:
			error = "Connection refused";
			break;
		case ECONNRESET:
			error = "Connection reset";
			break;
		case EDESTADDRREQ:
			error = "Destination address required";
			break;
		case EDQUOT:
			error = "Quota exceeded";
			break;
		case EHOSTUNREACH:
			error = "Host is unreachable";
			break;
		case EILSEQ:
			error = "Illegal byte sequence";
			break;
		case EINPROGRESS:
			error = "Operation in progress";
			break;
		case EISCONN:
			error = "Socket is connected";
			break;
		case ELOOP:
			error = "Too many levels of symbolic links";
			break;
		case EMSGSIZE:
			error = "Message too large";
			break;
		case ENAMETOOLONG:
			error = "Filename too long";
			break;
		case ENETDOWN:
			error = "Network is down";
			break;
		case ENETRESET:
			error = "Connection aborted by network";
			break;
		case ENETUNREACH:
			error = "Network unreachable";
			break;
		case ENOBUFS:
			error = "No buffer space is available";
			break;
		case ENOPROTOOPT:
			error = "Protocol not available";
			break;
		case ENOSYS:
			error = "Function not supported";
			break;
		case ENOTCONN:
			error = "The socket is not connected";
			break;
		case ENOTEMPTY:
			error = "Directory not empty";
			break;
		case ENOTSOCK:
			error = "Not a socket";
			break;
		case ENOTSUP:
			error = "Not supported";
			break;
		case EOPNOTSUPP:
			error = "Operation not supported on socket";
			break;
		case EOVERFLOW:
			error = "Value too large to be stored in data type";
			break;
		case EPROTONOSUPPORT:
			error = "Protocol not supported";
			break;
		case EPROTOTYPE:
			error = "Protocol wrong type for socket";
			break;
		case ETIMEDOUT:
			error = "Connection timed out";
			break;
		case EWOULDBLOCK:
			error = "Operation would block";
			break;
		case ENOTRECOVERABLE:
			error = "State not recoverable";
			break;
		case EOWNERDEAD:
			error = "Previous owner died.";
			break;
		case ESTALE:
			error = "Stale NFS file handle";
			break;
#if defined(_XOPEN_SOURCE)
		case ENODATA:
			error = "No message is available on the stream head read queue";
			break;
		case ENOSR:
			error = "No stream resources";
			break;
		case ENOSTR:
			error = "Not a stream";
			break;
		case ETIME:
			error = "Stream ioctl() timeout";
			break;
#endif
		default:
			errno = EINVAL;
			error = "Not a valid error number";
			break;
	}

	return error;
}

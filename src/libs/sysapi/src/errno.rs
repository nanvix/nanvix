// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//===================================================================================================
// Imports
//===================================================================================================

use crate::ffi::c_int;

//====================================================================================================
// Constants
//====================================================================================================

/// Operation not permitted.
pub const EPERM: c_int = 1;
/// No such file or directory.
pub const ENOENT: c_int = 2;
/// No such process.
pub const ESRCH: c_int = 3;
/// Interrupted system call.
pub const EINTR: c_int = 4;
/// I/O error.
pub const EIO: c_int = 5;
/// No such device or address.
pub const ENXIO: c_int = 6;
/// Argument list too long.
pub const E2BIG: c_int = 7;
/// Exec format error.
pub const ENOEXEC: c_int = 8;
/// Bad file number.
pub const EBADF: c_int = 9;
/// No child processes.
pub const ECHILD: c_int = 10;
/// Try again.
pub const EAGAIN: c_int = 11;
/// Out of memory.
pub const ENOMEM: c_int = 12;
/// Permission denied.
pub const EACCES: c_int = 13;
/// Bad address.
pub const EFAULT: c_int = 14;
/// Block device required.
pub const ENOTBLK: c_int = 15;
/// Device or resource busy.
pub const EBUSY: c_int = 16;
/// File exists.
pub const EEXIST: c_int = 17;
/// Cross-device link.
pub const EXDEV: c_int = 18;
/// No such device.
pub const ENODEV: c_int = 19;
/// Not a directory.
pub const ENOTDIR: c_int = 20;
/// Is a directory.
pub const EISDIR: c_int = 21;
/// Invalid argument.
pub const EINVAL: c_int = 22;
/// File table overflow.
pub const ENFILE: c_int = 23;
/// Too many open files.
pub const EMFILE: c_int = 24;
/// Not a typewriter.
pub const ENOTTY: c_int = 25;
/// Text file busy.
pub const ETXTBSY: c_int = 26;
/// File too large.
pub const EFBIG: c_int = 27;
/// No space left on device.
pub const ENOSPC: c_int = 28;
/// Illegal seek.
pub const ESPIPE: c_int = 29;
/// Read-only file system.
pub const EROFS: c_int = 30;
/// Too many links.
pub const EMLINK: c_int = 31;
/// Broken pipe.
pub const EPIPE: c_int = 32;
/// Math argument out of domain of function.
pub const EDOM: c_int = 33;
/// Math result not representable.
pub const ERANGE: c_int = 34;
/// No message of desired type.
pub const ENOMSG: c_int = 35;
/// Identifier removed.
pub const EIDRM: c_int = 36;
/// Channel number out of range.
pub const ECHRNG: c_int = 37;
/// Level 2 not synchronized.
pub const EL2NSYNC: c_int = 38;
/// Level 3 halted.
pub const EL3HLT: c_int = 39;
/// Level 3 reset.
pub const EL3RST: c_int = 40;
/// Link number out of range.
pub const ELNRNG: c_int = 41;
/// Protocol driver not attached.
pub const EUNATCH: c_int = 42;
/// No CSI structure available.
pub const ENOCSI: c_int = 43;
/// Level 2 halted.
pub const EL2HLT: c_int = 44;
/// Resource deadlock would occur.
pub const EDEADLK: c_int = 45;
/// No record locks available.
pub const ENOLCK: c_int = 46;
/// Invalid exchange.
pub const EBADE: c_int = 50;
/// Invalid request descriptor.
pub const EBADR: c_int = 51;
/// Exchange full.
pub const EXFULL: c_int = 52;
/// No anode.
pub const ENOANO: c_int = 53;
/// Invalid request code.
pub const EBADRQC: c_int = 54;
/// Invalid slot.
pub const EBADSLT: c_int = 55;
/// File locking deadlock error.
pub const EDEADLOCK: c_int = 56;
/// Bad font file format.
pub const EBFONT: c_int = 57;
/// Device not a stream.
pub const ENOSTR: c_int = 60;
/// No data available.
pub const ENODATA: c_int = 61;
/// Timer expired.
pub const ETIME: c_int = 62;
/// Out of streams resources.
pub const ENOSR: c_int = 63;
/// Machine is not on the network.
pub const ENONET: c_int = 64;
/// Package not installed.
pub const ENOPKG: c_int = 65;
/// Object is remote.
pub const EREMOTE: c_int = 66;
/// Link has been severed.
pub const ENOLINK: c_int = 67;
/// Advertise error.
pub const EADV: c_int = 68;
/// Srmount error.
pub const ESRMNT: c_int = 69;
/// Communication error on send.
pub const ECOMM: c_int = 70;
/// Protocol error.
pub const EPROTO: c_int = 71;
/// Multihop attempted.
pub const EMULTIHOP: c_int = 74;
/// Remote inode.
pub const ELBIN: c_int = 75;
/// RFS specific error.
pub const EDOTDOT: c_int = 76;
/// Not a data message.
pub const EBADMSG: c_int = 77;
/// Inappropriate file type or format.
pub const EFTYPE: c_int = 79;
/// Name not unique on network.
pub const ENOTUNIQ: c_int = 80;
/// File descriptor in bad state.
pub const EBADFD: c_int = 81;
/// Remote address changed.
pub const EREMCHG: c_int = 82;
/// Can not access a needed shared library.
pub const ELIBACC: c_int = 83;
/// Accessing a corrupted shared library.
pub const ELIBBAD: c_int = 84;
/// .lib section in a.out corrupted.
pub const ELIBSCN: c_int = 85;
/// Attempting to link in too many shared libraries.
pub const ELIBMAX: c_int = 86;
/// Cannot exec a shared library directly.
pub const ELIBEXEC: c_int = 87;
/// Function not implemented.
pub const ENOSYS: c_int = 88;
/// Directory not empty.
pub const ENOTEMPTY: c_int = 90;
/// File name too long.
pub const ENAMETOOLONG: c_int = 91;
/// Too many symbolic links encountered.
pub const ELOOP: c_int = 92;
/// Operation not supported on socket.
pub const EOPNOTSUPP: c_int = 95;
/// Protocol family not supported.
pub const EPFNOSUPPORT: c_int = 96;
/// Connection reset by peer.
pub const ECONNRESET: c_int = 104;
/// No buffer space available.
pub const ENOBUFS: c_int = 105;
/// Address family not supported by protocol.
pub const EAFNOSUPPORT: c_int = 106;
/// Protocol wrong type for socket.
pub const EPROTOTYPE: c_int = 107;
/// Socket operation on non-socket.
pub const ENOTSOCK: c_int = 108;
/// Protocol not available.
pub const ENOPROTOOPT: c_int = 109;
/// Cannot send after transport endpoint shutdown.
pub const ESHUTDOWN: c_int = 110;
/// Connection refused.
pub const ECONNREFUSED: c_int = 111;
/// Address already in use.
pub const EADDRINUSE: c_int = 112;
/// Software caused connection abort.
pub const ECONNABORTED: c_int = 113;
/// Network is unreachable.
pub const ENETUNREACH: c_int = 114;
/// Network is down.
pub const ENETDOWN: c_int = 115;
/// Connection timed out.
pub const ETIMEDOUT: c_int = 116;
/// Host is down.
pub const EHOSTDOWN: c_int = 117;
/// No route to host.
pub const EHOSTUNREACH: c_int = 118;
/// Operation now in progress.
pub const EINPROGRESS: c_int = 119;
/// Operation already in progress.
pub const EALREADY: c_int = 120;
/// Destination address required.
pub const EDESTADDRREQ: c_int = 121;
/// Message too long.
pub const EMSGSIZE: c_int = 122;
/// Protocol not supported.
pub const EPROTONOSUPPORT: c_int = 123;
/// Socket type not supported.
pub const ESOCKTNOSUPPORT: c_int = 124;
/// Cannot assign requested address.
pub const EADDRNOTAVAIL: c_int = 125;
/// Network dropped connection on reset.
pub const ENETRESET: c_int = 126;
/// Transport endpoint is already connected.
pub const EISCONN: c_int = 127;
/// Transport endpoint is not connected.
pub const ENOTCONN: c_int = 128;
/// Too many references: cannot splice.
pub const ETOOMANYREFS: c_int = 129;
/// Too many users.
pub const EUSERS: c_int = 131;
/// Disk quota exceeded.
pub const EDQUOT: c_int = 132;
/// Stale file handle.
pub const ESTALE: c_int = 133;
/// Operation not supported.
pub const ENOTSUP: c_int = 134;
/// No medium found.
pub const ENOMEDIUM: c_int = 135;
/// Illegal byte sequence.
pub const EILSEQ: c_int = 138;
/// Value too large for defined data type.
pub const EOVERFLOW: c_int = 139;
/// Operation canceled.
pub const ECANCELED: c_int = 140;
/// State not recoverable.
pub const ENOTRECOVERABLE: c_int = 141;
/// Owner died.
pub const EOWNERDEAD: c_int = 142;
/// Streams pipe error.
pub const ESTRPIPE: c_int = 143;

unsafe extern "C" {
    pub unsafe fn __errno_location() -> *mut c_int;
}

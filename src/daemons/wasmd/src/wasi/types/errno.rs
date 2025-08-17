// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::num_enum::TryFromPrimitive;

//==================================================================================================
// Enumerations
//==================================================================================================

#[allow(dead_code)]
#[derive(Debug, TryFromPrimitive)]
#[repr(u16)]
pub enum Errno {
    Success = 0,
    TooBig = 1,
    AccessDenied = 2,
    Addrinuse = 3,
    Addrnotavail = 4,
    Afnosupport = 5,
    Again = 6,
    Already = 7,
    Badf = 8,
    Badmsg = 9,
    Busy = 10,
    Canceled = 11,
    Child = 12,
    Connaborted = 13,
    Connrefused = 14,
    Connreset = 15,
    Deadlk = 16,
    Destaddrreq = 17,
    Dom = 18,
    Dquot = 19,
    Exist = 20,
    Fault = 21,
    Fbig = 22,
    Hostunreach = 23,
    Idrm = 24,
    Ilseq = 25,
    Inprogress = 26,
    Intr = 27,
    Inval = 28,
    Io = 29,
    Isconn = 30,
    Isdir = 31,
    Loop = 32,
    Mfile = 33,
    Mlink = 34,
    Msgsize = 35,
    Multihop = 36,
    Nametoolong = 37,
    Netdown = 38,
    Netreset = 39,
    Netunreach = 40,
    Nfile = 41,
    Nobufs = 42,
    Nodev = 43,
    Noent = 44,
    Noexec = 45,
    Nolck = 46,
    Nolink = 47,
    Nomem = 48,
    Nomsg = 49,
    Noprotoopt = 50,
    Nospc = 51,
    Nosys = 52,
    Notconn = 53,
    Notdir = 54,
    Notempty = 55,
    Notrecoverable = 56,
    Notsock = 57,
    Notsup = 58,
    Notty = 59,
    Nxio = 60,
    Overflow = 61,
    Ownerdead = 62,
    Perm = 63,
    Pipe = 64,
    Proto = 65,
    Protonosupport = 66,
    Prototype = 67,
    Range = 68,
    Rofs = 69,
    Spipe = 70,
    Srch = 71,
    Stale = 72,
    Timedout = 73,
    Txtbsy = 74,
    Xdev = 75,
    Notcapable = 76,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl From<Errno> for i32 {
    fn from(errno: Errno) -> i32 {
        errno as i32
    }
}

impl From<i32> for Errno {
    fn from(errno: i32) -> Errno {
        match Errno::try_from(errno as u16) {
            Ok(errno) => errno,
            Err(_) => Errno::Inval,
        }
    }
}

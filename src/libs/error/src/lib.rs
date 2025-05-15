// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::num_enum::TryFromPrimitive;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Error code for various adverse conditions.
///
/// # Notes
///
/// The values in this enumeration intentionally match the error codes defined in the Linux kernel.
///
#[derive(Copy, Clone, PartialEq, Eq, Debug, TryFromPrimitive)]
#[num_enum(error_type(name = Error, constructor = invalid_error_code))]
#[repr(i32)]
pub enum ErrorCode {
    /// Operation not permitted (EPERM).
    OperationNotPermitted = 1,
    /// No such file or directory (ENOENT).
    NoSuchEntry = 2,
    /// No such process (ESRCH).
    NoSuchProcess = 3,
    /// Interrupted system call (EINTR).
    Interrupted = 4,
    /// I/O error (EIO).
    IoErr = 5,
    /// No such device or address (ENXIO).
    NoSuchDeviceOrAddress = 6,
    /// Argument list too long (E2BIG).
    TooBig = 7,
    /// Executable format error (ENOEXEC).
    InvalidExecutableFormat = 8,
    /// Bad file number (EBADF).
    BadFile = 9,
    /// No child processes (ECHILD).
    NoChildProcess = 10,
    /// Try again (EAGAIN).
    TryAgain = 11,
    /// Out of memory (ENOMEM).
    OutOfMemory = 12,
    /// Permission denied (EACCES).
    PermissionDenied = 13,
    /// Bad address (EFAULT).
    BadAddress = 14,
    /// Not a device required (ENOTBLK).
    NotBlockDevice = 15,
    /// Device or resource busy (EBUSY).
    ResourceBusy = 16,
    /// Entry Exists (EEXIST).
    EntryExists = 17,
    /// Cross-device link (EXDEV).
    CrossDeviceLink = 18,
    /// No such device (ENODEV).
    NoSuchDevice = 19,
    /// Not a directory (ENOTDIR).
    InvalidDirectory = 20,
    /// Is a directory (EISDIR).
    IsDirectory = 21,
    /// Invalid argument (EINVAL).
    InvalidArgument = 22,
    /// File table overflow (ENFILE).
    FileTableOVerflow = 23,
    /// Too many open files (EMFILE).
    TooManyOpenFiles = 24,
    /// Not a typewriter (ENOTTY).
    InvalidTerminalOperation = 25,
    /// Text file busy (ETXTBSY).
    TextFileBusy = 26,
    /// File too large (EFBIG).
    FileTooLarge = 27,
    /// No space left on device (ENOSPC).
    NoSpaceOnDevice = 28,
    /// Illegal seek (ESPIPE).
    IllegalSeek = 29,
    /// Read-only file system (EROFS).
    ReadOnlyFileSystem = 30,
    /// Too many links (EMLINK).
    TooManyLinks = 31,
    /// Broken pipe (EPIPE).
    BrokenPipe = 32,
    /// Math argument out of domain of function (EDOM).
    MathArgDomainErr = 33,
    /// Math result not representable (ERANGE).
    ValueOutOfRange = 34,
    /// No message of desired type (ENOMSG).
    NoMessageAvailable = 35,
    /// Identifier removed (EIDRM).
    IdentifierRemoved = 36,
    /// Channel number out of range (ECHRNG).
    OutOfRangeChannel = 37,
    /// Level 2 not synchronized (EL2NSYNC).
    Level2NotSynchronized = 38,
    /// Level 3 halted (EL3HLT).
    Level3Halted = 39,
    /// Level 3 reset (EL3RST).
    Level3Reset = 40,
    /// Link number out of range (ELNRNG).
    InvalidLinkNumber = 41,
    /// Protocol driver not attached (EUNATCH).
    InvalidProtocolDriver = 42,
    /// No CSI structure available (ENOCSI).
    NoStructAvailable = 43,
    /// Level 2 halted (EL2HLT).
    Level2Halted = 44,
    /// Resource deadlock would occur (EDEADLK).
    Deadlock = 45,
    /// No record locks available (ENOLCK).
    LockNotAvailable = 46,
    /// Invalid exchange (EBADE).
    InvalidExchange = 50,
    /// Invalid request descriptor (EBADR).
    InvalidRequestDescriptor = 51,
    /// Exchange full (EXFULL).
    ExchangeFull = 52,
    /// No anode (ENOANO).
    InvalidAnode = 53,
    /// Invalid request code (EBADRQC).
    InvalidRequestCode = 54,
    /// Invalid slot (EBADSLT).
    InvalidSlot = 55,
    /// Resource deadlock would occur (EDEADLOCK).
    DeadlockWouldOccur = 56,
    /// Bad font file format (EBFONT).
    BadFontFormat = 57,
    /// Device not a stream (ENOSTR).
    NoStreamDeviceAvailable = 60,
    /// No data available (ENODATA).
    NoDataAvailable = 61,
    /// Timer expired (ETIME).
    TimerExpired = 62,
    /// Out of streams resources (ENOSR).
    NoStreamResources = 63,
    /// Machine is not on the network (ENONET).
    NoNetwork = 64,
    /// Package not installed (ENOPKG).
    MissingPackage = 65,
    /// Object is remote (EREMOTE).
    RemoteObject = 66,
    /// Link has been severed (ENOLINK).
    NoLink = 67,
    /// Advertise error (EADV).
    AdvertiseErr = 68,
    /// Remote file share system mount error (ESRMNT).
    MountErr = 69,
    /// Communication error on send (ECOMM).
    CommunicationErr = 70,
    /// Protocol error (EPROTO).
    ProtocolErr = 71,
    /// Multi-hop attempted (EMULTIHOP).
    MultipleHopAttemped = 74,
    /// Inode is remote (ELBIN)
    InodeRemote = 75,
    /// RFS specific error (EDOTDOT).
    RfsErr = 76,
    /// Not a data message (EBADMSG).
    InvalidMessage = 77,
    /// Innapropriate file type or format (EFTYPE).
    InvalidFileType = 79,
    /// Name not unique on network (ENOTUNIQ).
    NonUniqueName = 80,
    /// File descriptor in bad state (EBADFD).
    InvalidFileDescriptor = 81,
    /// Remote address changed (EREMCHG).
    RemoteAddressChanged = 82,
    /// Can not access a needed shared library (ELIBACC).
    LibraryAccessErr = 83,
    /// Accessing a corrupted shared library (ELIBBAD).
    InvalidLibraryAccess = 84,
    /// .lib section in a.out corrupted (ELIBSCN).
    CorruptedLibSection = 85,
    /// Attempting to link in too many shared libraries (ELIBMAX).
    ExcessiveLibraryLinkCount = 86,
    /// Cannot exec a shared library directly (ELIBEXEC).
    InvalidExecSharedLibrary = 87,
    /// Invalid system call number (ENOSYS).
    InvalidSysCall = 88,
    /// Directory not empty (ENOTEMPTY).
    DirectoryNotEmpty = 90,
    /// File name too long (ENAMETOOLONG).
    NameTooLong = 91,
    /// Too many symbolic links encountered (ELOOP).
    SymbolicLinkLoop = 92,
    /// Operation not supported on transport endpoint (EOPNOTSUPP).
    OperationNotSupportedOnSocket = 95,
    /// Protocol family not supported (EPFNOSUPPORT).
    ProtocolFamilyNotSupported = 96,
    /// Connection reset by peer (ECONNRESET).
    ConnectionReset = 104,
    /// No buffer space available (ENOBUFS).
    NoBufferSpace = 105,
    /// Address family not supported by protocol (EAFNOSUPPORT).
    AddressFamilyNotSupported = 106,
    /// Protocol wrong type for socket (EPROTOTYPE).
    BadProtocolType = 107,
    /// Socket operation on non-socket (ENOTSOCK).
    NotSocketFile = 108,
    /// Protocol not available (ENOPROTOOPT).
    ProtocolOptionNotAvailable = 109,
    /// Cannot send after transport endpoint shutdown (ESHUTDOWN).
    TransportEndpointShutdown = 110,
    /// Connection refused (ECONNREFUSED).
    ConnectionRefused = 111,
    /// Address already in use (EADDRINUSE).
    AddressInUse = 112,
    /// Software caused connection abort (ECONNABORTED).
    ConnectionAborted = 113,
    /// Network is unreachable (ENETUNREACH).
    NetworkUnreachable = 114,
    /// Network is down (ENETDOWN).
    NetworkDown = 115,
    /// Connection timed out (ETIMEDOUT).
    OperationTimedOut = 116,
    /// Host is down (EHOSTDOWN).
    HostDown = 117,
    /// No route to host (EHOSTUNREACH).
    HostUnreachable = 118,
    /// Operation now in progress (EINPROGRESS).
    OperationInProgress = 119,
    /// Operation already in progress (EALREADY).
    OperationAlreadyInProgress = 120,
    /// Destination address required (EDESTADDRREQ).
    DestinationAddressRequired = 121,
    /// Message too long (EMSGSIZE).
    MessageTooLong = 122,
    /// Protocol not supported (EPROTONOSUPPORT).
    ProtocolNotSupported = 123,
    /// Socket type not supported (ESOCKTNOSUPPORT).
    SocketTypeNotSupported = 124,
    /// Cannot assign requested address (EADDRNOTAVAIL).
    AddressNotAvailable = 125,
    /// Network dropped connection because of reset (ENETRESET).
    NetworkReset = 126,
    /// Transport endpoint is already connected (EISCONN).
    TransportEndpointConnected = 127,
    /// Transport endpoint is not connected (ENOTCONN).
    TransportEndpointNotConnected = 128,
    /// Too many references: cannot splice (ETOOMANYREFS).
    TooManyReferences = 129,
    /// Too many users (EUSERS).
    TooManyUsers = 131,
    /// Quota exceeded (EDQUOT).
    QuotaExceeded = 132,
    /// Stale file handle (ESTALE).
    StaleHandle = 133,
    /// Operation not supported (ENOTSUP).
    OperationNotSupported = 134,
    /// No medium found (ENOMEDIUM).
    MediumNotFound = 135,
    /// Illegal byte sequence (EILSEQ).
    IllegalByteSequence = 138,
    /// Value too large for defined data type (EOVERFLOW).
    ValueOverflow = 139,
    /// Operation Canceled (ECANCELED).
    OperationCanceled = 140,
    /// State not recoverable (ENOTRECOVERABLE).
    UnrecoverableState = 141,
    /// Owner died (EOWNERDEAD).
    DeadOwner = 142,
    /// Streams pipe error (ESTRPIPE).
    StreamPipeErr = 143,
}

impl ErrorCode {
    #[allow(non_upper_case_globals)]
    pub const OperationWouldBlock: ErrorCode = ErrorCode::TryAgain;

    ///
    /// # Description
    ///
    /// Returns the error code as an `i32`.
    ///
    pub fn get(&self) -> i32 {
        *self as i32
    }
}

///
/// # Description
///
/// Constructs a default error.
///
/// # Parameters
///
/// - `value`: Error code (unused).
///
/// # Returns
///
/// Default error.
///
fn invalid_error_code(_value: i32) -> Error {
    Error {
        code: ErrorCode::InvalidArgument,
        reason: "invalid error code",
    }
}

#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub reason: &'static str,
}

impl Error {
    pub fn new(code: ErrorCode, reason: &'static str) -> Self {
        Self { code, reason }
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

impl core::error::Error for ErrorCode {}

impl core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "error={:?}", self)
    }
}

impl From<ErrorCode> for u32 {
    fn from(errno: ErrorCode) -> Self {
        errno as u32
    }
}

impl TryFrom<i64> for ErrorCode {
    type Error = Error;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match ErrorCode::try_from_primitive(value as i32) {
            Ok(code) => Ok(code),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid error code")),
        }
    }
}

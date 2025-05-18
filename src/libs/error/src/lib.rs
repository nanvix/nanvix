// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

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
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ErrorCode {
    /// Operation not permitted.
    OperationNotPermitted = ErrorCode::EPERM,
    /// No such file or directory.
    NoSuchEntry = ErrorCode::ENOENT,
    /// No such process.
    NoSuchProcess = ErrorCode::ESRCH,
    /// Interrupted system call.
    Interrupted = ErrorCode::EINTR,
    /// I/O error.
    IoErr = ErrorCode::EIO,
    /// No such device or address.
    NoSuchDeviceOrAddress = ErrorCode::ENXIO,
    /// Argument list too long.
    TooBig = ErrorCode::E2BIG,
    /// Exec format error.
    InvalidExecutableFormat = ErrorCode::ENOEXEC,
    /// Bad file number.
    BadFile = ErrorCode::EBADF,
    /// No child processes.
    NoChildProcess = ErrorCode::ECHILD,
    /// Try again.
    TryAgain = ErrorCode::EAGAIN,
    /// Out of memory.
    OutOfMemory = ErrorCode::ENOMEM,
    /// Permission denied.
    PermissionDenied = ErrorCode::EACCES,
    /// Bad address.
    BadAddress = ErrorCode::EFAULT,
    /// Block device required.
    NotBlockDevice = ErrorCode::ENOTBLK,
    /// Device or resource busy.
    ResourceBusy = ErrorCode::EBUSY,
    /// File exists.
    EntryExists = ErrorCode::EEXIST,
    /// Cross-device link.
    CrossDeviceLink = ErrorCode::EXDEV,
    /// No such device.
    NoSuchDevice = ErrorCode::ENODEV,
    /// Not a directory.
    InvalidDirectory = ErrorCode::ENOTDIR,
    /// Is a directory.
    IsDirectory = ErrorCode::EISDIR,
    /// Invalid argument.
    InvalidArgument = ErrorCode::EINVAL,
    /// File table overflow.
    FileTableOVerflow = ErrorCode::ENFILE,
    /// Too many open files.
    TooManyOpenFiles = ErrorCode::EMFILE,
    /// Not a typewriter.
    InvalidTerminalOperation = ErrorCode::ENOTTY,
    /// Text file busy.
    TextFileBusy = ErrorCode::ETXTBSY,
    /// File too large.
    FileTooLarge = ErrorCode::EFBIG,
    /// No space left on device.
    NoSpaceOnDevice = ErrorCode::ENOSPC,
    /// Illegal seek.
    IllegalSeek = ErrorCode::ESPIPE,
    /// Read-only file system.
    ReadOnlyFileSystem = ErrorCode::EROFS,
    /// Too many links.
    TooManyLinks = ErrorCode::EMLINK,
    /// Broken pipe.
    BrokenPipe = ErrorCode::EPIPE,
    /// Math argument out of domain of function.
    MathArgDomainErr = ErrorCode::EDOM,
    /// Math result not representable.
    ValueOutOfRange = ErrorCode::ERANGE,
    /// No message of desired type.
    NoMessageAvailable = ErrorCode::ENOMSG,
    /// Identifier removed.
    IdentifierRemoved = ErrorCode::EIDRM,
    /// Channel number out of range.
    OutOfRangeChannel = ErrorCode::ECHRNG,
    /// Level 2 not synchronized.
    Level2NotSynchronized = ErrorCode::EL2NSYNC,
    /// Level 3 halted.
    Level3Halted = ErrorCode::EL3HLT,
    /// Level 3 reset.
    Level3Reset = ErrorCode::EL3RST,
    /// Link number out of range.
    InvalidLinkNumber = ErrorCode::ELNRNG,
    /// Protocol driver not attached.
    InvalidProtocolDriver = ErrorCode::EUNATCH,
    /// No CSI structure available.
    NoStructAvailable = ErrorCode::ENOCSI,
    /// Level 2 halted.
    Level2Halted = ErrorCode::EL2HLT,
    /// Resource deadlock would occur.
    Deadlock = ErrorCode::EDEADLK,
    /// No record locks available.
    LockNotAvailable = ErrorCode::ENOLCK,
    /// Invalid exchange.
    InvalidExchange = ErrorCode::EBADE,
    /// Invalid request descriptor.
    InvalidRequestDescriptor = ErrorCode::EBADR,
    /// Exchange full.
    ExchangeFull = ErrorCode::EXFULL,
    /// No anode.
    InvalidAnode = ErrorCode::ENOANO,
    /// Invalid request code.
    InvalidRequestCode = ErrorCode::EBADRQC,
    /// Invalid slot.
    InvalidSlot = ErrorCode::EBADSLT,
    /// File locking deadlock error.
    DeadlockWouldOccur = ErrorCode::EDEADLOCK,
    /// Bad font file format.
    BadFontFormat = ErrorCode::EBFONT,
    /// Device not a stream.
    NoStreamDeviceAvailable = ErrorCode::ENOSTR,
    /// No data available.
    NoDataAvailable = ErrorCode::ENODATA,
    /// Timer expired.
    TimerExpired = ErrorCode::ETIME,
    /// Out of streams resources.
    NoStreamResources = ErrorCode::ENOSR,
    /// Machine is not on the network.
    NoNetwork = ErrorCode::ENONET,
    /// Package not installed.
    MissingPackage = ErrorCode::ENOPKG,
    /// Object is remote.
    RemoteObject = ErrorCode::EREMOTE,
    /// Link has been severed.
    NoLink = ErrorCode::ENOLINK,
    /// Advertise error.
    AdvertiseErr = ErrorCode::EADV,
    /// Srmount error.
    MountErr = ErrorCode::ESRMNT,
    /// Communication error on send.
    CommunicationErr = ErrorCode::ECOMM,
    /// Protocol error.
    ProtocolErr = ErrorCode::EPROTO,
    /// Multihop attempted.
    MultipleHopAttemped = ErrorCode::EMULTIHOP,
    /// Remote inode.
    InodeRemote = ErrorCode::ELBIN,
    /// RFS specific error.
    RfsErr = ErrorCode::EDOTDOT,
    /// Not a data message.
    InvalidMessage = ErrorCode::EBADMSG,
    /// Inappropriate file type or format.
    InvalidFileType = ErrorCode::EFTYPE,
    /// Name not unique on network.
    NonUniqueName = ErrorCode::ENOTUNIQ,
    /// File descriptor in bad state.
    InvalidFileDescriptor = ErrorCode::EBADFD,
    /// Remote address changed.
    RemoteAddressChanged = ErrorCode::EREMCHG,
    /// Can not access a needed shared library.
    LibraryAccessErr = ErrorCode::ELIBACC,
    /// Accessing a corrupted shared library.
    InvalidLibraryAccess = ErrorCode::ELIBBAD,
    /// .lib section in a.out corrupted.
    CorruptedLibSection = ErrorCode::ELIBSCN,
    /// Attempting to link in too many shared libraries.
    ExcessiveLibraryLinkCount = ErrorCode::ELIBMAX,
    /// Cannot exec a shared library directly.
    InvalidExecSharedLibrary = ErrorCode::ELIBEXEC,
    /// Function not implemented.
    InvalidSysCall = ErrorCode::ENOSYS,
    /// Directory not empty.
    DirectoryNotEmpty = ErrorCode::ENOTEMPTY,
    /// File name too long.
    NameTooLong = ErrorCode::ENAMETOOLONG,
    /// Too many symbolic links encountered.
    SymbolicLinkLoop = ErrorCode::ELOOP,
    /// Operation not supported on socket.
    OperationNotSupportedOnSocket = ErrorCode::EOPNOTSUPP,
    /// Protocol family not supported.
    ProtocolFamilyNotSupported = ErrorCode::EPFNOSUPPORT,
    /// Connection reset by peer.
    ConnectionReset = ErrorCode::ECONNRESET,
    /// No buffer space available.
    NoBufferSpace = ErrorCode::ENOBUFS,
    /// Address family not supported by protocol.
    AddressFamilyNotSupported = ErrorCode::EAFNOSUPPORT,
    /// Protocol wrong type for socket.
    BadProtocolType = ErrorCode::EPROTOTYPE,
    /// Socket operation on non-socket.
    NotSocketFile = ErrorCode::ENOTSOCK,
    /// Protocol not available.
    ProtocolOptionNotAvailable = ErrorCode::ENOPROTOOPT,
    /// Cannot send after transport endpoint shutdown.
    TransportEndpointShutdown = ErrorCode::ESHUTDOWN,
    /// Connection refused.
    ConnectionRefused = ErrorCode::ECONNREFUSED,
    /// Address already in use.
    AddressInUse = ErrorCode::EADDRINUSE,
    /// Software caused connection abort.
    ConnectionAborted = ErrorCode::ECONNABORTED,
    /// Network is unreachable.
    NetworkUnreachable = ErrorCode::ENETUNREACH,
    /// Network is down.
    NetworkDown = ErrorCode::ENETDOWN,
    /// Connection timed out.
    OperationTimedOut = ErrorCode::ETIMEDOUT,
    /// Host is down.
    HostDown = ErrorCode::EHOSTDOWN,
    /// No route to host.
    HostUnreachable = ErrorCode::EHOSTUNREACH,
    /// Operation now in progress.
    OperationInProgress = ErrorCode::EINPROGRESS,
    /// Operation already in progress.
    OperationAlreadyInProgress = ErrorCode::EALREADY,
    /// Destination address required.
    DestinationAddressRequired = ErrorCode::EDESTADDRREQ,
    /// Message too long.
    MessageTooLong = ErrorCode::EMSGSIZE,
    /// Protocol not supported.
    ProtocolNotSupported = ErrorCode::EPROTONOSUPPORT,
    /// Socket type not supported.
    SocketTypeNotSupported = ErrorCode::ESOCKTNOSUPPORT,
    /// Cannot assign requested address.
    AddressNotAvailable = ErrorCode::EADDRNOTAVAIL,
    /// Network dropped connection on reset.
    NetworkReset = ErrorCode::ENETRESET,
    /// Transport endpoint is already connected.
    TransportEndpointConnected = ErrorCode::EISCONN,
    /// Transport endpoint is not connected.
    TransportEndpointNotConnected = ErrorCode::ENOTCONN,
    /// Too many references: cannot splice.
    TooManyReferences = ErrorCode::ETOOMANYREFS,
    /// Too many users.
    TooManyUsers = ErrorCode::EUSERS,
    /// Disk quota exceeded.
    QuotaExceeded = ErrorCode::EDQUOT,
    /// Stale file handle.
    StaleHandle = ErrorCode::ESTALE,
    /// Operation not supported.
    OperationNotSupported = ErrorCode::ENOTSUP,
    /// No medium found.
    MediumNotFound = ErrorCode::ENOMEDIUM,
    /// Illegal byte sequence.
    IllegalByteSequence = ErrorCode::EILSEQ,
    /// Value too large for defined data type.
    ValueOverflow = ErrorCode::EOVERFLOW,
    /// Operation canceled.
    OperationCanceled = ErrorCode::ECANCELED,
    /// State not recoverable.
    UnrecoverableState = ErrorCode::ENOTRECOVERABLE,
    /// Owner died.
    DeadOwner = ErrorCode::EOWNERDEAD,
    /// Streams pipe error.
    StreamPipeErr = ErrorCode::ESTRPIPE,
}

impl ErrorCode {
    const EPERM: i32 = 1;
    const ENOENT: i32 = 2;
    const ESRCH: i32 = 3;
    const EINTR: i32 = 4;
    const EIO: i32 = 5;
    const ENXIO: i32 = 6;
    const E2BIG: i32 = 7;
    const ENOEXEC: i32 = 8;
    const EBADF: i32 = 9;
    const ECHILD: i32 = 10;
    const EAGAIN: i32 = 11;
    const ENOMEM: i32 = 12;
    const EACCES: i32 = 13;
    const EFAULT: i32 = 14;
    const ENOTBLK: i32 = 15;
    const EBUSY: i32 = 16;
    const EEXIST: i32 = 17;
    const EXDEV: i32 = 18;
    const ENODEV: i32 = 19;
    const ENOTDIR: i32 = 20;
    const EISDIR: i32 = 21;
    const EINVAL: i32 = 22;
    const ENFILE: i32 = 23;
    const EMFILE: i32 = 24;
    const ENOTTY: i32 = 25;
    const ETXTBSY: i32 = 26;
    const EFBIG: i32 = 27;
    const ENOSPC: i32 = 28;
    const ESPIPE: i32 = 29;
    const EROFS: i32 = 30;
    const EMLINK: i32 = 31;
    const EPIPE: i32 = 32;
    const EDOM: i32 = 33;
    const ERANGE: i32 = 34;
    const ENOMSG: i32 = 35;
    const EIDRM: i32 = 36;
    const ECHRNG: i32 = 37;
    const EL2NSYNC: i32 = 38;
    const EL3HLT: i32 = 39;
    const EL3RST: i32 = 40;
    const ELNRNG: i32 = 41;
    const EUNATCH: i32 = 42;
    const ENOCSI: i32 = 43;
    const EL2HLT: i32 = 44;
    const EDEADLK: i32 = 45;
    const ENOLCK: i32 = 46;
    const EBADE: i32 = 50;
    const EBADR: i32 = 51;
    const EXFULL: i32 = 52;
    const ENOANO: i32 = 53;
    const EBADRQC: i32 = 54;
    const EBADSLT: i32 = 55;
    const EDEADLOCK: i32 = 56;
    const EBFONT: i32 = 57;
    const ENOSTR: i32 = 60;
    const ENODATA: i32 = 61;
    const ETIME: i32 = 62;
    const ENOSR: i32 = 63;
    const ENONET: i32 = 64;
    const ENOPKG: i32 = 65;
    const EREMOTE: i32 = 66;
    const ENOLINK: i32 = 67;
    const EADV: i32 = 68;
    const ESRMNT: i32 = 69;
    const ECOMM: i32 = 70;
    const EPROTO: i32 = 71;
    const EMULTIHOP: i32 = 74;
    const ELBIN: i32 = 75;
    const EDOTDOT: i32 = 76;
    const EBADMSG: i32 = 77;
    const EFTYPE: i32 = 79;
    const ENOTUNIQ: i32 = 80;
    const EBADFD: i32 = 81;
    const EREMCHG: i32 = 82;
    const ELIBACC: i32 = 83;
    const ELIBBAD: i32 = 84;
    const ELIBSCN: i32 = 85;
    const ELIBMAX: i32 = 86;
    const ELIBEXEC: i32 = 87;
    const ENOSYS: i32 = 88;
    const ENOTEMPTY: i32 = 90;
    const ENAMETOOLONG: i32 = 91;
    const ELOOP: i32 = 92;
    const EOPNOTSUPP: i32 = 95;
    const EPFNOSUPPORT: i32 = 96;
    const ECONNRESET: i32 = 104;
    const ENOBUFS: i32 = 105;
    const EAFNOSUPPORT: i32 = 106;
    const EPROTOTYPE: i32 = 107;
    const ENOTSOCK: i32 = 108;
    const ENOPROTOOPT: i32 = 109;
    const ESHUTDOWN: i32 = 110;
    const ECONNREFUSED: i32 = 111;
    const EADDRINUSE: i32 = 112;
    const ECONNABORTED: i32 = 113;
    const ENETUNREACH: i32 = 114;
    const ENETDOWN: i32 = 115;
    const ETIMEDOUT: i32 = 116;
    const EHOSTDOWN: i32 = 117;
    const EHOSTUNREACH: i32 = 118;
    const EINPROGRESS: i32 = 119;
    const EALREADY: i32 = 120;
    const EDESTADDRREQ: i32 = 121;
    const EMSGSIZE: i32 = 122;
    const EPROTONOSUPPORT: i32 = 123;
    const ESOCKTNOSUPPORT: i32 = 124;
    const EADDRNOTAVAIL: i32 = 125;
    const ENETRESET: i32 = 126;
    const EISCONN: i32 = 127;
    const ENOTCONN: i32 = 128;
    const ETOOMANYREFS: i32 = 129;
    const EUSERS: i32 = 131;
    const EDQUOT: i32 = 132;
    const ESTALE: i32 = 133;
    const ENOTSUP: i32 = 134;
    const ENOMEDIUM: i32 = 135;
    const EILSEQ: i32 = 138;
    const EOVERFLOW: i32 = 139;
    const ECANCELED: i32 = 140;
    const ENOTRECOVERABLE: i32 = 141;
    const EOWNERDEAD: i32 = 142;
    const ESTRPIPE: i32 = 143;

    ///
    /// # Description
    ///
    /// Returns the error code as an `i32`.
    ///
    pub fn get(&self) -> i32 {
        *self as i32
    }
}

// Manual conversion from i32 to ErrorCode using constants
impl TryFrom<i32> for ErrorCode {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            Self::EPERM => Ok(ErrorCode::OperationNotPermitted),
            Self::ENOENT => Ok(ErrorCode::NoSuchEntry),
            Self::ESRCH => Ok(ErrorCode::NoSuchProcess),
            Self::EINTR => Ok(ErrorCode::Interrupted),
            Self::EIO => Ok(ErrorCode::IoErr),
            Self::ENXIO => Ok(ErrorCode::NoSuchDeviceOrAddress),
            Self::E2BIG => Ok(ErrorCode::TooBig),
            Self::ENOEXEC => Ok(ErrorCode::InvalidExecutableFormat),
            Self::EBADF => Ok(ErrorCode::BadFile),
            Self::ECHILD => Ok(ErrorCode::NoChildProcess),
            Self::EAGAIN => Ok(ErrorCode::TryAgain),
            Self::ENOMEM => Ok(ErrorCode::OutOfMemory),
            Self::EACCES => Ok(ErrorCode::PermissionDenied),
            Self::EFAULT => Ok(ErrorCode::BadAddress),
            Self::ENOTBLK => Ok(ErrorCode::NotBlockDevice),
            Self::EBUSY => Ok(ErrorCode::ResourceBusy),
            Self::EEXIST => Ok(ErrorCode::EntryExists),
            Self::EXDEV => Ok(ErrorCode::CrossDeviceLink),
            Self::ENODEV => Ok(ErrorCode::NoSuchDevice),
            Self::ENOTDIR => Ok(ErrorCode::InvalidDirectory),
            Self::EISDIR => Ok(ErrorCode::IsDirectory),
            Self::EINVAL => Ok(ErrorCode::InvalidArgument),
            Self::ENFILE => Ok(ErrorCode::FileTableOVerflow),
            Self::EMFILE => Ok(ErrorCode::TooManyOpenFiles),
            Self::ENOTTY => Ok(ErrorCode::InvalidTerminalOperation),
            Self::ETXTBSY => Ok(ErrorCode::TextFileBusy),
            Self::EFBIG => Ok(ErrorCode::FileTooLarge),
            Self::ENOSPC => Ok(ErrorCode::NoSpaceOnDevice),
            Self::ESPIPE => Ok(ErrorCode::IllegalSeek),
            Self::EROFS => Ok(ErrorCode::ReadOnlyFileSystem),
            Self::EMLINK => Ok(ErrorCode::TooManyLinks),
            Self::EPIPE => Ok(ErrorCode::BrokenPipe),
            Self::EDOM => Ok(ErrorCode::MathArgDomainErr),
            Self::ERANGE => Ok(ErrorCode::ValueOutOfRange),
            Self::ENOMSG => Ok(ErrorCode::NoMessageAvailable),
            Self::EIDRM => Ok(ErrorCode::IdentifierRemoved),
            Self::ECHRNG => Ok(ErrorCode::OutOfRangeChannel),
            Self::EL2NSYNC => Ok(ErrorCode::Level2NotSynchronized),
            Self::EL3HLT => Ok(ErrorCode::Level3Halted),
            Self::EL3RST => Ok(ErrorCode::Level3Reset),
            Self::ELNRNG => Ok(ErrorCode::InvalidLinkNumber),
            Self::EUNATCH => Ok(ErrorCode::InvalidProtocolDriver),
            Self::ENOCSI => Ok(ErrorCode::NoStructAvailable),
            Self::EL2HLT => Ok(ErrorCode::Level2Halted),
            Self::EDEADLK => Ok(ErrorCode::Deadlock),
            Self::ENOLCK => Ok(ErrorCode::LockNotAvailable),
            Self::EBADE => Ok(ErrorCode::InvalidExchange),
            Self::EBADR => Ok(ErrorCode::InvalidRequestDescriptor),
            Self::EXFULL => Ok(ErrorCode::ExchangeFull),
            Self::ENOANO => Ok(ErrorCode::InvalidAnode),
            Self::EBADRQC => Ok(ErrorCode::InvalidRequestCode),
            Self::EBADSLT => Ok(ErrorCode::InvalidSlot),
            Self::EDEADLOCK => Ok(ErrorCode::DeadlockWouldOccur),
            Self::EBFONT => Ok(ErrorCode::BadFontFormat),
            Self::ENOSTR => Ok(ErrorCode::NoStreamDeviceAvailable),
            Self::ENODATA => Ok(ErrorCode::NoDataAvailable),
            Self::ETIME => Ok(ErrorCode::TimerExpired),
            Self::ENOSR => Ok(ErrorCode::NoStreamResources),
            Self::ENONET => Ok(ErrorCode::NoNetwork),
            Self::ENOPKG => Ok(ErrorCode::MissingPackage),
            Self::EREMOTE => Ok(ErrorCode::RemoteObject),
            Self::ENOLINK => Ok(ErrorCode::NoLink),
            Self::EADV => Ok(ErrorCode::AdvertiseErr),
            Self::ESRMNT => Ok(ErrorCode::MountErr),
            Self::ECOMM => Ok(ErrorCode::CommunicationErr),
            Self::EPROTO => Ok(ErrorCode::ProtocolErr),
            Self::EMULTIHOP => Ok(ErrorCode::MultipleHopAttemped),
            Self::ELBIN => Ok(ErrorCode::InodeRemote),
            Self::EDOTDOT => Ok(ErrorCode::RfsErr),
            Self::EBADMSG => Ok(ErrorCode::InvalidMessage),
            Self::EFTYPE => Ok(ErrorCode::InvalidFileType),
            Self::ENOTUNIQ => Ok(ErrorCode::NonUniqueName),
            Self::EBADFD => Ok(ErrorCode::InvalidFileDescriptor),
            Self::EREMCHG => Ok(ErrorCode::RemoteAddressChanged),
            Self::ELIBACC => Ok(ErrorCode::LibraryAccessErr),
            Self::ELIBBAD => Ok(ErrorCode::InvalidLibraryAccess),
            Self::ELIBSCN => Ok(ErrorCode::CorruptedLibSection),
            Self::ELIBMAX => Ok(ErrorCode::ExcessiveLibraryLinkCount),
            Self::ELIBEXEC => Ok(ErrorCode::InvalidExecSharedLibrary),
            Self::ENOSYS => Ok(ErrorCode::InvalidSysCall),
            Self::ENOTEMPTY => Ok(ErrorCode::DirectoryNotEmpty),
            Self::ENAMETOOLONG => Ok(ErrorCode::NameTooLong),
            Self::ELOOP => Ok(ErrorCode::SymbolicLinkLoop),
            Self::EOPNOTSUPP => Ok(ErrorCode::OperationNotSupportedOnSocket),
            Self::EPFNOSUPPORT => Ok(ErrorCode::ProtocolFamilyNotSupported),
            Self::ECONNRESET => Ok(ErrorCode::ConnectionReset),
            Self::ENOBUFS => Ok(ErrorCode::NoBufferSpace),
            Self::EAFNOSUPPORT => Ok(ErrorCode::AddressFamilyNotSupported),
            Self::EPROTOTYPE => Ok(ErrorCode::BadProtocolType),
            Self::ENOTSOCK => Ok(ErrorCode::NotSocketFile),
            Self::ENOPROTOOPT => Ok(ErrorCode::ProtocolOptionNotAvailable),
            Self::ESHUTDOWN => Ok(ErrorCode::TransportEndpointShutdown),
            Self::ECONNREFUSED => Ok(ErrorCode::ConnectionRefused),
            Self::EADDRINUSE => Ok(ErrorCode::AddressInUse),
            Self::ECONNABORTED => Ok(ErrorCode::ConnectionAborted),
            Self::ENETUNREACH => Ok(ErrorCode::NetworkUnreachable),
            Self::ENETDOWN => Ok(ErrorCode::NetworkDown),
            Self::ETIMEDOUT => Ok(ErrorCode::OperationTimedOut),
            Self::EHOSTDOWN => Ok(ErrorCode::HostDown),
            Self::EHOSTUNREACH => Ok(ErrorCode::HostUnreachable),
            Self::EINPROGRESS => Ok(ErrorCode::OperationInProgress),
            Self::EALREADY => Ok(ErrorCode::OperationAlreadyInProgress),
            Self::EDESTADDRREQ => Ok(ErrorCode::DestinationAddressRequired),
            Self::EMSGSIZE => Ok(ErrorCode::MessageTooLong),
            Self::EPROTONOSUPPORT => Ok(ErrorCode::ProtocolNotSupported),
            Self::ESOCKTNOSUPPORT => Ok(ErrorCode::SocketTypeNotSupported),
            Self::EADDRNOTAVAIL => Ok(ErrorCode::AddressNotAvailable),
            Self::ENETRESET => Ok(ErrorCode::NetworkReset),
            Self::EISCONN => Ok(ErrorCode::TransportEndpointConnected),
            Self::ENOTCONN => Ok(ErrorCode::TransportEndpointNotConnected),
            Self::ETOOMANYREFS => Ok(ErrorCode::TooManyReferences),
            Self::EUSERS => Ok(ErrorCode::TooManyUsers),
            Self::EDQUOT => Ok(ErrorCode::QuotaExceeded),
            Self::ESTALE => Ok(ErrorCode::StaleHandle),
            Self::ENOTSUP => Ok(ErrorCode::OperationNotSupported),
            Self::ENOMEDIUM => Ok(ErrorCode::MediumNotFound),
            Self::EILSEQ => Ok(ErrorCode::IllegalByteSequence),
            Self::EOVERFLOW => Ok(ErrorCode::ValueOverflow),
            Self::ECANCELED => Ok(ErrorCode::OperationCanceled),
            Self::ENOTRECOVERABLE => Ok(ErrorCode::UnrecoverableState),
            Self::EOWNERDEAD => Ok(ErrorCode::DeadOwner),
            Self::ESTRPIPE => Ok(ErrorCode::StreamPipeErr),
            _ => Err(invalid_error_code(value)),
        }
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "error={self:?}")
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
        // Attempt to convert i64 to i32.
        let value: i32 = value
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid error code"))?;

        // Attempt to convert i32 to ErrorCode.
        ErrorCode::try_from(value)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid error code"))
    }
}

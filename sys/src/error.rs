// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    /// Executable format error (ENOEXE).
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
    /// Resource deadlock would occur (EDEADLK).
    Deadlock = 35,
    /// File name too long (ENAMETOOLONG).
    NameTooLong = 36,
    /// No record locks available (ENOLCK).
    LockNotAvailable = 37,
    /// Invalid system call number (ENOSYS).
    InvalidSysCall = 38,
    /// Directory not empty (ENOTEMPTY).
    DirectoryNotEmpty = 39,
    /// Too many symbolic links encountered (ELOOP).
    SymbolicLinkLoop = 40,
    /// Operation would block (EWOULDBLOCK).
    OperationWouldBlock = 41,
    /// No message of desired type (ENOMSG).
    NoMessageAvailable = 42,
    /// Identifier removed (EIDRM).
    IdentifierRemoved = 43,
    /// Channel number out of range (ECHRNG).
    OutOfRangeChannel = 44,
    /// Level 2 not synchronized (EL2NSYNC).
    Level2NotSynchronized = 45,
    /// Level 3 halted (EL3HLT).
    Level3Halted = 46,
    /// Level 3 reset (EL3RST).
    Level3Reset = 47,
    /// Link number out of range (ELNRNG).
    InvalidLinkNumber = 48,
    /// Protocol driver not attached (EUNATCH).
    InvalidProtocolDriver = 49,
    /// No CSI structure available (ENOCSI).
    NoStructAvailable = 50,
    /// Level 2 halted (EL2HLT).
    Level2Halted = 51,
    /// Invalid exchange (EBADE).
    InvalidExchange = 52,
    /// Invalid request descriptor (EBADR).
    InvalidRequestDescriptor = 53,
    /// Exchange full (EXFULL).
    ExchangeFull = 54,
    /// No anode (ENOANO).
    InvalidAnode = 55,
    /// Invalid request code (EBADRQC).
    InvalidRequestCode = 56,
    /// Invalid slot (EBADSLT).
    InvalidSlot = 57,
    /// Resource deadlock would occur (EDEADLOCK).
    DeadlockWouldOccur = 58,
    /// Bad font file format (ENOSTR).
    BadFontFormat = 59,
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
    MultipleHopAttemped = 72,
    /// RFS specific error (EDOTDOT).
    RfsErr = 73,
    /// Not a data message (EBADMSG).
    InvalidMessage = 74,
    /// Value too large for defined data type (EOVERFLOW).
    ValueOverflow = 75,
    /// Name not unique on network (ENOTUNIQ).
    NonUniqueName = 76,
    /// File descriptor in bad state (EBADFD).
    InvalidFileDescriptor = 77,
    /// Remote address changed (EREMCHG).
    RemoteAddressChanged = 78,
    /// Can not access a needed shared library (ELIBACC).
    LibraryAccessErr = 79,
    /// Accessing a corrupted shared library (ELIBBAD).
    InvalidLibraryAccess = 80,
    /// .lib section in a.out corrupted (ELIBSCN).
    CorruptedLibSection = 81,
    /// Attempting to link in too many shared libraries (ELIBMAX).
    ExcessiveLibraryLinkCount = 82,
    /// Cannot exec a shared library directly (ELIBEXEC).
    InvalidExecSharedLibrary = 83,
    /// Illegal byte sequence (EILSEQ).
    IllegalByteSequence = 84,
    /// Interrupted system call should be restarted (ERESTART).
    RestartRequired = 85,
    /// Streams pipe error (ESTRPIPE).
    StreamPipeErr = 86,
    /// Too many users (EUSERS).
    TooManyUsers = 87,
    /// Socket operation on non-socket (ENOTSOCK).
    NotSocketFile = 88,
    /// Destination address required (EDESTADDRREQ).
    DestinationAddressRequired = 89,
    /// Message too long (EMSGSIZE).
    MessageTooLong = 90,
    /// Protocol wrong type for socket (EPROTOTYPE).
    BadProtocolType = 91,
    /// Protocol not available (ENOPROTOOPT).
    ProtocolOptionNotAvailable = 92,
    /// Protocol not supported (EPROTONOSUPPORT).
    ProtocolNotSupported = 93,
    /// Socket type not supported (ESOCKTNOSUPPORT).
    SocketTypeNotSupported = 94,
    /// Operation not supported on transport endpoint (EOPNOTSUPP).
    OperationNotSupported = 95,
    /// Protocol family not supported (EPFNOSUPPORT).
    ProtocolFamilyNotSupported = 96,
    /// Address family not supported by protocol (EAFNOSUPPORT).
    AddressFamilyNotSupported = 97,
    /// Address already in use (EADDRINUSE).
    AddressInUse = 98,
    /// Cannot assign requested address (EADDRNOTAVAIL).
    AddressNotAvailable = 99,
    /// Network is down (ENETDOWN).
    NetworkDown = 100,
    /// Network is unreachable (ENETUNREACH).
    NetworkUnreachable = 101,
    /// Network dropped connection because of reset (ENETRESET).
    NetworkReset = 102,
    /// Software caused connection abort (ECONNABORTED).
    ConnectionAborted = 103,
    /// Connection reset by peer (ECONNRESET).
    ConnectionReset = 104,
    /// No buffer space available (ENOBUFS).
    NoBufferSpace = 105,
    /// Transport endpoint is already connected (EISCONN).
    TransportEndpointConnected = 106,
    /// Transport endpoint is not connected (ENOTCONN).
    TransportEndpointNotConnected = 107,
    /// Cannot send after transport endpoint shutdown (ESHUTDOWN).
    TransportEndpointShutdown = 108,
    /// Too many references: cannot splice (ETOOMANYREFS).
    TooManyReferences = 109,
    /// Connection timed out (ETIMEDOUT).
    ConnectionTimeout = 110,
    /// Connection refused (ECONNREFUSED).
    ConnectionRefused = 111,
    /// Host is down (EHOSTDOWN).
    HostDown = 112,
    /// No route to host (EHOSTUNREACH).
    HostUnreachable = 113,
    /// Operation already in progress (EALREADY).
    OperationAlreadyInProgress = 114,
    /// Operation now in progress (EINPROGRESS).
    OperationInProgress = 115,
    /// Stale file handle (ESTALE).
    StaleHandle = 116,
    /// Structure needs cleaning (EUCLEAN).
    UncleanStructure = 117,
    /// Not a XENIX named type file (ENOTNAM).
    NoXenixtNamedTypeFile = 118,
    /// No XENIX semaphores available (ENAVAIL).
    NoXenixSemaphoresAvailable = 119,
    /// Is a named type file (EISNAM).
    IsNamedTypeFile = 120,
    /// Remote I/O error (EREMOTEIO).
    RemoteIOErr = 121,
    /// Quota exceeded (EDQUOT).
    QuotaExceeded = 122,
    /// No medium found (ENOMEDIUM).
    MediumNotFound = 123,
    /// Wrong medium type (EMEDIUMTYPE).
    InvalidMediumType = 124,
    /// Operation Canceled (ECANCELED).
    OperationCanceled = 125,
    /// Required key not available (ENOKEY).
    MissingKey = 126,
    /// Key has expired (EKEYEXPIRED).
    ExpiredKey = 127,
    /// Key has been revoked (EKEYREVOKED).
    KeyRevoked = 128,
    /// Key was rejected by service (EKEYREJECTED).
    KeyRejected = 129,
    /// Owner died (EOWNERDEAD).
    DeadOwner = 130,
    /// State not recoverable (ENOTRECOVERABLE).
    UnrecoverableState = 131,
    /// Operation not possible due to RF-kill (ERFKILL).
    RfKillSwitch = 132,
    /// Memory page has hardware error (EHWPOISON).
    HardwarePoison = 133,
}

impl ErrorCode {
    ///
    /// # Description
    ///
    /// Converts an [`ErrorCode`] into an `errno` value.
    ///
    pub fn into_errno(self) -> i32 {
        -(self as i32)
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

impl TryFrom<i32> for ErrorCode {
    type Error = Error;

    fn try_from(errno: i32) -> Result<Self, Self::Error> {
        match errno {
            1 => Ok(ErrorCode::OperationNotPermitted),
            2 => Ok(ErrorCode::NoSuchEntry),
            3 => Ok(ErrorCode::NoSuchProcess),
            4 => Ok(ErrorCode::Interrupted),
            5 => Ok(ErrorCode::IoErr),
            6 => Ok(ErrorCode::NoSuchDeviceOrAddress),
            7 => Ok(ErrorCode::TooBig),
            8 => Ok(ErrorCode::InvalidExecutableFormat),
            9 => Ok(ErrorCode::BadFile),
            10 => Ok(ErrorCode::NoChildProcess),
            11 => Ok(ErrorCode::TryAgain),
            12 => Ok(ErrorCode::OutOfMemory),
            13 => Ok(ErrorCode::PermissionDenied),
            14 => Ok(ErrorCode::BadAddress),
            15 => Ok(ErrorCode::NotBlockDevice),
            16 => Ok(ErrorCode::ResourceBusy),
            17 => Ok(ErrorCode::EntryExists),
            18 => Ok(ErrorCode::CrossDeviceLink),
            19 => Ok(ErrorCode::NoSuchDevice),
            20 => Ok(ErrorCode::InvalidDirectory),
            21 => Ok(ErrorCode::IsDirectory),
            22 => Ok(ErrorCode::InvalidArgument),
            23 => Ok(ErrorCode::FileTableOVerflow),
            24 => Ok(ErrorCode::TooManyOpenFiles),
            25 => Ok(ErrorCode::InvalidTerminalOperation),
            26 => Ok(ErrorCode::TextFileBusy),
            27 => Ok(ErrorCode::FileTooLarge),
            28 => Ok(ErrorCode::NoSpaceOnDevice),
            29 => Ok(ErrorCode::IllegalSeek),
            30 => Ok(ErrorCode::ReadOnlyFileSystem),
            31 => Ok(ErrorCode::TooManyLinks),
            32 => Ok(ErrorCode::BrokenPipe),
            33 => Ok(ErrorCode::MathArgDomainErr),
            34 => Ok(ErrorCode::ValueOutOfRange),
            35 => Ok(ErrorCode::Deadlock),
            36 => Ok(ErrorCode::NameTooLong),
            37 => Ok(ErrorCode::LockNotAvailable),
            38 => Ok(ErrorCode::InvalidSysCall),
            39 => Ok(ErrorCode::DirectoryNotEmpty),
            40 => Ok(ErrorCode::SymbolicLinkLoop),
            41 => Ok(ErrorCode::OperationWouldBlock),
            42 => Ok(ErrorCode::NoMessageAvailable),
            43 => Ok(ErrorCode::IdentifierRemoved),
            44 => Ok(ErrorCode::OutOfRangeChannel),
            45 => Ok(ErrorCode::Level2NotSynchronized),
            46 => Ok(ErrorCode::Level3Halted),
            47 => Ok(ErrorCode::Level3Reset),
            48 => Ok(ErrorCode::InvalidLinkNumber),
            49 => Ok(ErrorCode::InvalidProtocolDriver),
            50 => Ok(ErrorCode::NoStructAvailable),
            51 => Ok(ErrorCode::Level2Halted),
            52 => Ok(ErrorCode::InvalidExchange),
            53 => Ok(ErrorCode::InvalidRequestDescriptor),
            54 => Ok(ErrorCode::ExchangeFull),
            55 => Ok(ErrorCode::InvalidAnode),
            56 => Ok(ErrorCode::InvalidRequestCode),
            57 => Ok(ErrorCode::InvalidSlot),
            58 => Ok(ErrorCode::DeadlockWouldOccur),
            59 => Ok(ErrorCode::BadFontFormat),
            60 => Ok(ErrorCode::NoStreamDeviceAvailable),
            61 => Ok(ErrorCode::NoDataAvailable),
            62 => Ok(ErrorCode::TimerExpired),
            63 => Ok(ErrorCode::NoStreamResources),
            64 => Ok(ErrorCode::NoNetwork),
            65 => Ok(ErrorCode::MissingPackage),
            66 => Ok(ErrorCode::RemoteObject),
            67 => Ok(ErrorCode::NoLink),
            68 => Ok(ErrorCode::AdvertiseErr),
            69 => Ok(ErrorCode::MountErr),
            70 => Ok(ErrorCode::CommunicationErr),
            71 => Ok(ErrorCode::ProtocolErr),
            72 => Ok(ErrorCode::MultipleHopAttemped),
            73 => Ok(ErrorCode::RfsErr),
            74 => Ok(ErrorCode::InvalidMessage),
            75 => Ok(ErrorCode::ValueOverflow),
            76 => Ok(ErrorCode::NonUniqueName),
            77 => Ok(ErrorCode::InvalidFileDescriptor),
            78 => Ok(ErrorCode::RemoteAddressChanged),
            79 => Ok(ErrorCode::LibraryAccessErr),
            80 => Ok(ErrorCode::InvalidLibraryAccess),
            81 => Ok(ErrorCode::CorruptedLibSection),
            82 => Ok(ErrorCode::ExcessiveLibraryLinkCount),
            83 => Ok(ErrorCode::InvalidExecSharedLibrary),
            84 => Ok(ErrorCode::IllegalByteSequence),
            85 => Ok(ErrorCode::RestartRequired),
            86 => Ok(ErrorCode::StreamPipeErr),
            87 => Ok(ErrorCode::TooManyUsers),
            88 => Ok(ErrorCode::NotSocketFile),
            89 => Ok(ErrorCode::DestinationAddressRequired),
            90 => Ok(ErrorCode::MessageTooLong),
            91 => Ok(ErrorCode::BadProtocolType),
            92 => Ok(ErrorCode::ProtocolOptionNotAvailable),
            93 => Ok(ErrorCode::ProtocolNotSupported),
            94 => Ok(ErrorCode::SocketTypeNotSupported),
            95 => Ok(ErrorCode::OperationNotSupported),
            96 => Ok(ErrorCode::ProtocolFamilyNotSupported),
            97 => Ok(ErrorCode::AddressFamilyNotSupported),
            98 => Ok(ErrorCode::AddressInUse),
            99 => Ok(ErrorCode::AddressNotAvailable),
            100 => Ok(ErrorCode::NetworkDown),
            101 => Ok(ErrorCode::NetworkUnreachable),
            102 => Ok(ErrorCode::NetworkReset),
            103 => Ok(ErrorCode::ConnectionAborted),
            104 => Ok(ErrorCode::ConnectionReset),
            105 => Ok(ErrorCode::NoBufferSpace),
            106 => Ok(ErrorCode::TransportEndpointConnected),
            107 => Ok(ErrorCode::TransportEndpointNotConnected),
            108 => Ok(ErrorCode::TransportEndpointShutdown),
            109 => Ok(ErrorCode::TooManyReferences),
            110 => Ok(ErrorCode::ConnectionTimeout),
            111 => Ok(ErrorCode::ConnectionRefused),
            112 => Ok(ErrorCode::HostDown),
            113 => Ok(ErrorCode::HostUnreachable),
            114 => Ok(ErrorCode::OperationAlreadyInProgress),
            115 => Ok(ErrorCode::OperationInProgress),
            116 => Ok(ErrorCode::StaleHandle),
            117 => Ok(ErrorCode::UncleanStructure),
            118 => Ok(ErrorCode::NoXenixtNamedTypeFile),
            119 => Ok(ErrorCode::NoXenixSemaphoresAvailable),
            120 => Ok(ErrorCode::IsNamedTypeFile),
            121 => Ok(ErrorCode::RemoteIOErr),
            122 => Ok(ErrorCode::QuotaExceeded),
            123 => Ok(ErrorCode::MediumNotFound),
            124 => Ok(ErrorCode::InvalidMediumType),
            125 => Ok(ErrorCode::OperationCanceled),
            126 => Ok(ErrorCode::MissingKey),
            127 => Ok(ErrorCode::ExpiredKey),
            128 => Ok(ErrorCode::KeyRevoked),
            129 => Ok(ErrorCode::KeyRejected),
            130 => Ok(ErrorCode::DeadOwner),
            131 => Ok(ErrorCode::UnrecoverableState),
            132 => Ok(ErrorCode::RfKillSwitch),
            133 => Ok(ErrorCode::HardwarePoison),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid error code")),
        }
    }
}

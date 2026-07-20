// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::errno::*;

#[cfg(target_os = "windows")]
use ::windows::Win32::Networking::WinSock as winsock_errno;

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
/// The numeric discriminants in this enumeration are the Nanvix errno values exported by
/// `sysapi::errno`, not host OS errno values.
///
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ErrorCode {
    /// Operation not permitted.
    OperationNotPermitted = EPERM,
    /// No such file or directory.
    NoSuchEntry = ENOENT,
    /// No such process.
    NoSuchProcess = ESRCH,
    /// Interrupted system call.
    Interrupted = EINTR,
    /// I/O error.
    IoErr = EIO,
    /// No such device or address.
    NoSuchDeviceOrAddress = ENXIO,
    /// Argument list too long.
    TooBig = E2BIG,
    /// Exec format error.
    InvalidExecutableFormat = ENOEXEC,
    /// Bad file number.
    BadFile = EBADF,
    /// No child processes.
    NoChildProcess = ECHILD,
    /// Try again.
    TryAgain = EAGAIN,
    /// Out of memory.
    OutOfMemory = ENOMEM,
    /// Permission denied.
    PermissionDenied = EACCES,
    /// Bad address.
    BadAddress = EFAULT,
    /// Block device required.
    NotBlockDevice = ENOTBLK,
    /// Device or resource busy.
    ResourceBusy = EBUSY,
    /// File exists.
    EntryExists = EEXIST,
    /// Cross-device link.
    CrossDeviceLink = EXDEV,
    /// No such device.
    NoSuchDevice = ENODEV,
    /// Not a directory.
    InvalidDirectory = ENOTDIR,
    /// Is a directory.
    IsDirectory = EISDIR,
    /// Invalid argument.
    InvalidArgument = EINVAL,
    /// File table overflow.
    FileTableOVerflow = ENFILE,
    /// Too many open files.
    TooManyOpenFiles = EMFILE,
    /// Not a typewriter.
    NotTerminal = ENOTTY,
    /// Text file busy.
    TextFileBusy = ETXTBSY,
    /// File too large.
    FileTooLarge = EFBIG,
    /// No space left on device.
    NoSpaceOnDevice = ENOSPC,
    /// Illegal seek.
    IllegalSeek = ESPIPE,
    /// Read-only file system.
    ReadOnlyFileSystem = EROFS,
    /// Too many links.
    TooManyLinks = EMLINK,
    /// Broken pipe.
    BrokenPipe = EPIPE,
    /// Math argument out of domain of function.
    MathArgDomainErr = EDOM,
    /// Math result not representable.
    ValueOutOfRange = ERANGE,
    /// No message of desired type.
    NoMessageAvailable = ENOMSG,
    /// Identifier removed.
    IdentifierRemoved = EIDRM,
    /// Channel number out of range.
    OutOfRangeChannel = ECHRNG,
    /// Level 2 not synchronized.
    Level2NotSynchronized = EL2NSYNC,
    /// Level 3 halted.
    Level3Halted = EL3HLT,
    /// Level 3 reset.
    Level3Reset = EL3RST,
    /// Link number out of range.
    InvalidLinkNumber = ELNRNG,
    /// Protocol driver not attached.
    InvalidProtocolDriver = EUNATCH,
    /// No CSI structure available.
    NoStructAvailable = ENOCSI,
    /// Level 2 halted.
    Level2Halted = EL2HLT,
    /// Resource deadlock would occur.
    Deadlock = EDEADLK,
    /// No record locks available.
    LockNotAvailable = ENOLCK,
    /// Invalid exchange.
    InvalidExchange = EBADE,
    /// Invalid request descriptor.
    InvalidRequestDescriptor = EBADR,
    /// Exchange full.
    ExchangeFull = EXFULL,
    /// No anode.
    InvalidAnode = ENOANO,
    /// Invalid request code.
    InvalidRequestCode = EBADRQC,
    /// Invalid slot.
    InvalidSlot = EBADSLT,
    /// File locking deadlock error.
    DeadlockWouldOccur = EDEADLOCK,
    /// Bad font file format.
    BadFontFormat = EBFONT,
    /// Device not a stream.
    NoStreamDeviceAvailable = ENOSTR,
    /// No data available.
    NoDataAvailable = ENODATA,
    /// Timer expired.
    TimerExpired = ETIME,
    /// Out of streams resources.
    NoStreamResources = ENOSR,
    /// Machine is not on the network.
    NoNetwork = ENONET,
    /// Package not installed.
    MissingPackage = ENOPKG,
    /// Object is remote.
    RemoteObject = EREMOTE,
    /// Link has been severed.
    NoLink = ENOLINK,
    /// Advertise error.
    AdvertiseErr = EADV,
    /// Srmount error.
    MountErr = ESRMNT,
    /// Communication error on send.
    CommunicationErr = ECOMM,
    /// Protocol error.
    ProtocolErr = EPROTO,
    /// Multihop attempted.
    MultipleHopAttemped = EMULTIHOP,
    /// Remote inode.
    InodeRemote = ELBIN,
    /// RFS specific error.
    RfsErr = EDOTDOT,
    /// Not a data message.
    InvalidMessage = EBADMSG,
    /// Inappropriate file type or format.
    InvalidFileType = EFTYPE,
    /// Name not unique on network.
    NonUniqueName = ENOTUNIQ,
    /// File descriptor in bad state.
    InvalidFileDescriptor = EBADFD,
    /// Remote address changed.
    RemoteAddressChanged = EREMCHG,
    /// Can not access a needed shared library.
    LibraryAccessErr = ELIBACC,
    /// Accessing a corrupted shared library.
    InvalidLibraryAccess = ELIBBAD,
    /// .lib section in a.out corrupted.
    CorruptedLibSection = ELIBSCN,
    /// Attempting to link in too many shared libraries.
    ExcessiveLibraryLinkCount = ELIBMAX,
    /// Cannot exec a shared library directly.
    InvalidExecSharedLibrary = ELIBEXEC,
    /// Function not implemented.
    InvalidSysCall = ENOSYS,
    /// Directory not empty.
    DirectoryNotEmpty = ENOTEMPTY,
    /// File name too long.
    NameTooLong = ENAMETOOLONG,
    /// Too many symbolic links encountered.
    SymbolicLinkLoop = ELOOP,
    /// Operation not supported on socket.
    OperationNotSupportedOnSocket = EOPNOTSUPP,
    /// Protocol family not supported.
    ProtocolFamilyNotSupported = EPFNOSUPPORT,
    /// Connection reset by peer.
    ConnectionReset = ECONNRESET,
    /// No buffer space available.
    NoBufferSpace = ENOBUFS,
    /// Address family not supported by protocol.
    AddressFamilyNotSupported = EAFNOSUPPORT,
    /// Protocol wrong type for socket.
    BadProtocolType = EPROTOTYPE,
    /// Socket operation on non-socket.
    NotSocketFile = ENOTSOCK,
    /// Protocol not available.
    ProtocolOptionNotAvailable = ENOPROTOOPT,
    /// Cannot send after transport endpoint shutdown.
    TransportEndpointShutdown = ESHUTDOWN,
    /// Connection refused.
    ConnectionRefused = ECONNREFUSED,
    /// Address already in use.
    AddressInUse = EADDRINUSE,
    /// Software caused connection abort.
    ConnectionAborted = ECONNABORTED,
    /// Network is unreachable.
    NetworkUnreachable = ENETUNREACH,
    /// Network is down.
    NetworkDown = ENETDOWN,
    /// Connection timed out.
    OperationTimedOut = ETIMEDOUT,
    /// Host is down.
    HostDown = EHOSTDOWN,
    /// No route to host.
    HostUnreachable = EHOSTUNREACH,
    /// Operation now in progress.
    OperationInProgress = EINPROGRESS,
    /// Operation already in progress.
    OperationAlreadyInProgress = EALREADY,
    /// Destination address required.
    DestinationAddressRequired = EDESTADDRREQ,
    /// Message too long.
    MessageTooLong = EMSGSIZE,
    /// Protocol not supported.
    ProtocolNotSupported = EPROTONOSUPPORT,
    /// Socket type not supported.
    SocketTypeNotSupported = ESOCKTNOSUPPORT,
    /// Cannot assign requested address.
    AddressNotAvailable = EADDRNOTAVAIL,
    /// Network dropped connection on reset.
    NetworkReset = ENETRESET,
    /// Transport endpoint is already connected.
    TransportEndpointConnected = EISCONN,
    /// Transport endpoint is not connected.
    TransportEndpointNotConnected = ENOTCONN,
    /// Too many references: cannot splice.
    TooManyReferences = ETOOMANYREFS,
    /// Too many users.
    TooManyUsers = EUSERS,
    /// Disk quota exceeded.
    QuotaExceeded = EDQUOT,
    /// Stale file handle.
    StaleHandle = ESTALE,
    /// Operation not supported.
    OperationNotSupported = ENOTSUP,
    /// No medium found.
    MediumNotFound = ENOMEDIUM,
    /// Illegal byte sequence.
    IllegalByteSequence = EILSEQ,
    /// Value too large for defined data type.
    ValueOverflow = EOVERFLOW,
    /// Operation canceled.
    OperationCanceled = ECANCELED,
    /// State not recoverable.
    UnrecoverableState = ENOTRECOVERABLE,
    /// Owner died.
    DeadOwner = EOWNERDEAD,
    /// Streams pipe error.
    StreamPipeErr = ESTRPIPE,
}

impl ErrorCode {
    ///
    /// # Description
    ///
    /// Returns the error code as an `i32`.
    ///
    pub fn get(&self) -> i32 {
        *self as i32
    }

    /// Converts a Linux host `errno` value into an [`ErrorCode`].
    ///
    /// Linux and Nanvix do not use the same numeric layout for all `errno`s, so callers that are
    /// translating host syscall failures should use this instead of [`ErrorCode::try_from`].
    #[cfg(target_os = "linux")]
    fn try_from_linux_errno(errno: i32) -> Result<Self, Error> {
        let errno: i32 = normalize_host_errno_value(errno);
        match errno {
            libc::EPERM => Ok(Self::OperationNotPermitted),
            libc::ENOENT => Ok(Self::NoSuchEntry),
            libc::ESRCH => Ok(Self::NoSuchProcess),
            libc::EINTR => Ok(Self::Interrupted),
            libc::EIO => Ok(Self::IoErr),
            libc::ENXIO => Ok(Self::NoSuchDeviceOrAddress),
            libc::E2BIG => Ok(Self::TooBig),
            libc::ENOEXEC => Ok(Self::InvalidExecutableFormat),
            libc::EBADF => Ok(Self::BadFile),
            libc::ECHILD => Ok(Self::NoChildProcess),
            libc::EAGAIN => Ok(Self::TryAgain),
            libc::ENOMEM => Ok(Self::OutOfMemory),
            libc::EACCES => Ok(Self::PermissionDenied),
            libc::EFAULT => Ok(Self::BadAddress),
            libc::ENOTBLK => Ok(Self::NotBlockDevice),
            libc::EBUSY => Ok(Self::ResourceBusy),
            libc::EEXIST => Ok(Self::EntryExists),
            libc::EXDEV => Ok(Self::CrossDeviceLink),
            libc::ENODEV => Ok(Self::NoSuchDevice),
            libc::ENOTDIR => Ok(Self::InvalidDirectory),
            libc::EISDIR => Ok(Self::IsDirectory),
            libc::EINVAL => Ok(Self::InvalidArgument),
            libc::ENFILE => Ok(Self::FileTableOVerflow),
            libc::EMFILE => Ok(Self::TooManyOpenFiles),
            libc::ENOTTY => Ok(Self::NotTerminal),
            libc::ETXTBSY => Ok(Self::TextFileBusy),
            libc::EFBIG => Ok(Self::FileTooLarge),
            libc::ENOSPC => Ok(Self::NoSpaceOnDevice),
            libc::ESPIPE => Ok(Self::IllegalSeek),
            libc::EROFS => Ok(Self::ReadOnlyFileSystem),
            libc::EMLINK => Ok(Self::TooManyLinks),
            libc::EPIPE => Ok(Self::BrokenPipe),
            libc::EDOM => Ok(Self::MathArgDomainErr),
            libc::ERANGE => Ok(Self::ValueOutOfRange),
            libc::EDEADLK => Ok(Self::Deadlock),
            libc::ENAMETOOLONG => Ok(Self::NameTooLong),
            libc::ENOLCK => Ok(Self::LockNotAvailable),
            libc::ENOSYS => Ok(Self::InvalidSysCall),
            libc::ENOTEMPTY => Ok(Self::DirectoryNotEmpty),
            libc::ELOOP => Ok(Self::SymbolicLinkLoop),
            libc::ENOMSG => Ok(Self::NoMessageAvailable),
            libc::EIDRM => Ok(Self::IdentifierRemoved),
            libc::ECHRNG => Ok(Self::OutOfRangeChannel),
            libc::EL2NSYNC => Ok(Self::Level2NotSynchronized),
            libc::EL3HLT => Ok(Self::Level3Halted),
            libc::EL3RST => Ok(Self::Level3Reset),
            libc::ELNRNG => Ok(Self::InvalidLinkNumber),
            libc::EUNATCH => Ok(Self::InvalidProtocolDriver),
            libc::ENOCSI => Ok(Self::NoStructAvailable),
            libc::EL2HLT => Ok(Self::Level2Halted),
            libc::EBADE => Ok(Self::InvalidExchange),
            libc::EBADR => Ok(Self::InvalidRequestDescriptor),
            libc::EXFULL => Ok(Self::ExchangeFull),
            libc::ENOANO => Ok(Self::InvalidAnode),
            libc::EBADRQC => Ok(Self::InvalidRequestCode),
            libc::EBADSLT => Ok(Self::InvalidSlot),
            libc::EBFONT => Ok(Self::BadFontFormat),
            libc::ENOSTR => Ok(Self::NoStreamDeviceAvailable),
            libc::ENODATA => Ok(Self::NoDataAvailable),
            libc::ETIME => Ok(Self::TimerExpired),
            libc::ENOSR => Ok(Self::NoStreamResources),
            libc::ENONET => Ok(Self::NoNetwork),
            libc::ENOPKG => Ok(Self::MissingPackage),
            libc::EREMOTE => Ok(Self::RemoteObject),
            libc::ENOLINK => Ok(Self::NoLink),
            libc::EADV => Ok(Self::AdvertiseErr),
            libc::ESRMNT => Ok(Self::MountErr),
            libc::ECOMM => Ok(Self::CommunicationErr),
            libc::EPROTO => Ok(Self::ProtocolErr),
            libc::EMULTIHOP => Ok(Self::MultipleHopAttemped),
            libc::EDOTDOT => Ok(Self::RfsErr),
            libc::EBADMSG => Ok(Self::InvalidMessage),
            libc::EOVERFLOW => Ok(Self::ValueOverflow),
            libc::ENOTUNIQ => Ok(Self::NonUniqueName),
            libc::EBADFD => Ok(Self::InvalidFileDescriptor),
            libc::EREMCHG => Ok(Self::RemoteAddressChanged),
            libc::ELIBACC => Ok(Self::LibraryAccessErr),
            libc::ELIBBAD => Ok(Self::InvalidLibraryAccess),
            libc::ELIBSCN => Ok(Self::CorruptedLibSection),
            libc::ELIBMAX => Ok(Self::ExcessiveLibraryLinkCount),
            libc::ELIBEXEC => Ok(Self::InvalidExecSharedLibrary),
            libc::EILSEQ => Ok(Self::IllegalByteSequence),
            libc::ESTRPIPE => Ok(Self::StreamPipeErr),
            libc::EUSERS => Ok(Self::TooManyUsers),
            libc::ENOTSOCK => Ok(Self::NotSocketFile),
            libc::EDESTADDRREQ => Ok(Self::DestinationAddressRequired),
            libc::EMSGSIZE => Ok(Self::MessageTooLong),
            libc::EPROTOTYPE => Ok(Self::BadProtocolType),
            libc::ENOPROTOOPT => Ok(Self::ProtocolOptionNotAvailable),
            libc::EPROTONOSUPPORT => Ok(Self::ProtocolNotSupported),
            libc::ESOCKTNOSUPPORT => Ok(Self::SocketTypeNotSupported),
            libc::EOPNOTSUPP => Ok(Self::OperationNotSupportedOnSocket),
            libc::EPFNOSUPPORT => Ok(Self::ProtocolFamilyNotSupported),
            libc::EAFNOSUPPORT => Ok(Self::AddressFamilyNotSupported),
            libc::EADDRINUSE => Ok(Self::AddressInUse),
            libc::EADDRNOTAVAIL => Ok(Self::AddressNotAvailable),
            libc::ENETDOWN => Ok(Self::NetworkDown),
            libc::ENETUNREACH => Ok(Self::NetworkUnreachable),
            libc::ENETRESET => Ok(Self::NetworkReset),
            libc::ECONNABORTED => Ok(Self::ConnectionAborted),
            libc::ECONNRESET => Ok(Self::ConnectionReset),
            libc::ENOBUFS => Ok(Self::NoBufferSpace),
            libc::EISCONN => Ok(Self::TransportEndpointConnected),
            libc::ENOTCONN => Ok(Self::TransportEndpointNotConnected),
            libc::ESHUTDOWN => Ok(Self::TransportEndpointShutdown),
            libc::ETOOMANYREFS => Ok(Self::TooManyReferences),
            libc::ETIMEDOUT => Ok(Self::OperationTimedOut),
            libc::ECONNREFUSED => Ok(Self::ConnectionRefused),
            libc::EHOSTDOWN => Ok(Self::HostDown),
            libc::EHOSTUNREACH => Ok(Self::HostUnreachable),
            libc::EALREADY => Ok(Self::OperationAlreadyInProgress),
            libc::EINPROGRESS => Ok(Self::OperationInProgress),
            libc::ESTALE => Ok(Self::StaleHandle),
            libc::EDQUOT => Ok(Self::QuotaExceeded),
            libc::ENOMEDIUM => Ok(Self::MediumNotFound),
            libc::ECANCELED => Ok(Self::OperationCanceled),
            libc::EOWNERDEAD => Ok(Self::DeadOwner),
            libc::ENOTRECOVERABLE => Ok(Self::UnrecoverableState),
            _ => Err(invalid_error_code(errno)),
        }
    }

    /// Converts a Winsock error value into an [`ErrorCode`].
    #[cfg(target_os = "windows")]
    fn try_from_winsock_errno(errno: i32) -> Result<Self, Error> {
        let errno: i32 = normalize_host_errno_value(errno);
        match winsock_errno::WSA_ERROR(errno) {
            winsock_errno::WSAEINTR => Ok(Self::Interrupted),
            winsock_errno::WSAEBADF => Ok(Self::BadFile),
            winsock_errno::WSAEACCES => Ok(Self::PermissionDenied),
            winsock_errno::WSAEFAULT => Ok(Self::BadAddress),
            winsock_errno::WSAEINVAL => Ok(Self::InvalidArgument),
            winsock_errno::WSAEMFILE => Ok(Self::TooManyOpenFiles),
            winsock_errno::WSAEWOULDBLOCK => Ok(Self::TryAgain),
            winsock_errno::WSAEINPROGRESS => Ok(Self::OperationInProgress),
            winsock_errno::WSAEALREADY => Ok(Self::OperationAlreadyInProgress),
            winsock_errno::WSAENOTSOCK => Ok(Self::NotSocketFile),
            winsock_errno::WSAEDESTADDRREQ => Ok(Self::DestinationAddressRequired),
            winsock_errno::WSAEMSGSIZE => Ok(Self::MessageTooLong),
            winsock_errno::WSAEPROTOTYPE => Ok(Self::BadProtocolType),
            winsock_errno::WSAENOPROTOOPT => Ok(Self::ProtocolOptionNotAvailable),
            winsock_errno::WSAEPROTONOSUPPORT => Ok(Self::ProtocolNotSupported),
            winsock_errno::WSAESOCKTNOSUPPORT => Ok(Self::SocketTypeNotSupported),
            winsock_errno::WSAEOPNOTSUPP => Ok(Self::OperationNotSupportedOnSocket),
            winsock_errno::WSAEPFNOSUPPORT => Ok(Self::ProtocolFamilyNotSupported),
            winsock_errno::WSAEAFNOSUPPORT => Ok(Self::AddressFamilyNotSupported),
            winsock_errno::WSAEADDRINUSE => Ok(Self::AddressInUse),
            winsock_errno::WSAEADDRNOTAVAIL => Ok(Self::AddressNotAvailable),
            winsock_errno::WSAENETDOWN => Ok(Self::NetworkDown),
            winsock_errno::WSAENETUNREACH => Ok(Self::NetworkUnreachable),
            winsock_errno::WSAENETRESET => Ok(Self::NetworkReset),
            winsock_errno::WSAECONNABORTED => Ok(Self::ConnectionAborted),
            winsock_errno::WSAECONNRESET => Ok(Self::ConnectionReset),
            winsock_errno::WSAENOBUFS => Ok(Self::NoBufferSpace),
            winsock_errno::WSAEISCONN => Ok(Self::TransportEndpointConnected),
            winsock_errno::WSAENOTCONN => Ok(Self::TransportEndpointNotConnected),
            winsock_errno::WSAESHUTDOWN => Ok(Self::TransportEndpointShutdown),
            winsock_errno::WSAETOOMANYREFS => Ok(Self::TooManyReferences),
            winsock_errno::WSAETIMEDOUT => Ok(Self::OperationTimedOut),
            winsock_errno::WSAECONNREFUSED => Ok(Self::ConnectionRefused),
            winsock_errno::WSAELOOP => Ok(Self::SymbolicLinkLoop),
            winsock_errno::WSAENAMETOOLONG => Ok(Self::NameTooLong),
            winsock_errno::WSAEHOSTDOWN => Ok(Self::HostDown),
            winsock_errno::WSAEHOSTUNREACH => Ok(Self::HostUnreachable),
            winsock_errno::WSAENOTEMPTY => Ok(Self::DirectoryNotEmpty),
            winsock_errno::WSAEUSERS => Ok(Self::TooManyUsers),
            winsock_errno::WSAEDQUOT => Ok(Self::QuotaExceeded),
            winsock_errno::WSAESTALE => Ok(Self::StaleHandle),
            winsock_errno::WSAEREMOTE => Ok(Self::RemoteObject),
            winsock_errno::WSANOTINITIALISED => Ok(Self::InvalidFileDescriptor),
            _ => Err(invalid_error_code(errno)),
        }
    }

    /// Converts a Winsock `connect()` error value into an [`ErrorCode`].
    ///
    /// Winsock reports a newly pending non-blocking `connect()` as `WSAEWOULDBLOCK`, while regular
    /// non-blocking I/O uses the same code for would-block.
    #[cfg(target_os = "windows")]
    fn try_from_winsock_connect_errno(errno: i32) -> Result<Self, Error> {
        let errno: i32 = normalize_host_errno_value(errno);
        match winsock_errno::WSA_ERROR(errno) {
            winsock_errno::WSAEWOULDBLOCK => Ok(Self::OperationInProgress),
            _ => Self::try_from_winsock_errno(errno),
        }
    }
}

/// Converts a host errno value into an [`ErrorCode`] for the current target OS.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn errno_to_error_code(errno: i32) -> ErrorCode {
    #[cfg(target_os = "linux")]
    {
        ErrorCode::try_from_linux_errno(errno).unwrap_or(ErrorCode::ValueOutOfRange)
    }

    #[cfg(target_os = "windows")]
    {
        ErrorCode::try_from_winsock_errno(errno).unwrap_or(ErrorCode::ValueOutOfRange)
    }
}

/// Converts a host `connect()` errno value into an [`ErrorCode`] for the current target OS.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn connect_errno_to_error_code(errno: i32) -> ErrorCode {
    #[cfg(target_os = "linux")]
    {
        errno_to_error_code(errno)
    }

    #[cfg(target_os = "windows")]
    {
        ErrorCode::try_from_winsock_connect_errno(errno).unwrap_or(ErrorCode::ValueOutOfRange)
    }
}

#[allow(dead_code)]
fn normalize_host_errno_value(value: i32) -> i32 {
    if value < 0 {
        match value.checked_abs() {
            Some(abs) => abs,
            None => value,
        }
    } else {
        value
    }
}

// Manual conversion from i32 to ErrorCode using constants.
// Accepts both positive and negative errno values (the Linux kernel call convention negates errno
// on the kernel side, so user-space may receive either form).
impl TryFrom<i32> for ErrorCode {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Error> {
        // Normalize to a positive errno value when possible, avoiding overflow on i32::MIN.
        let value: i32 = if value < 0 {
            match value.checked_abs() {
                Some(abs) => abs,
                None => value,
            }
        } else {
            value
        };
        match value {
            EPERM => Ok(ErrorCode::OperationNotPermitted),
            ENOENT => Ok(ErrorCode::NoSuchEntry),
            ESRCH => Ok(ErrorCode::NoSuchProcess),
            EINTR => Ok(ErrorCode::Interrupted),
            EIO => Ok(ErrorCode::IoErr),
            ENXIO => Ok(ErrorCode::NoSuchDeviceOrAddress),
            E2BIG => Ok(ErrorCode::TooBig),
            ENOEXEC => Ok(ErrorCode::InvalidExecutableFormat),
            EBADF => Ok(ErrorCode::BadFile),
            ECHILD => Ok(ErrorCode::NoChildProcess),
            EAGAIN => Ok(ErrorCode::TryAgain),
            ENOMEM => Ok(ErrorCode::OutOfMemory),
            EACCES => Ok(ErrorCode::PermissionDenied),
            EFAULT => Ok(ErrorCode::BadAddress),
            ENOTBLK => Ok(ErrorCode::NotBlockDevice),
            EBUSY => Ok(ErrorCode::ResourceBusy),
            EEXIST => Ok(ErrorCode::EntryExists),
            EXDEV => Ok(ErrorCode::CrossDeviceLink),
            ENODEV => Ok(ErrorCode::NoSuchDevice),
            ENOTDIR => Ok(ErrorCode::InvalidDirectory),
            EISDIR => Ok(ErrorCode::IsDirectory),
            EINVAL => Ok(ErrorCode::InvalidArgument),
            ENFILE => Ok(ErrorCode::FileTableOVerflow),
            EMFILE => Ok(ErrorCode::TooManyOpenFiles),
            ENOTTY => Ok(ErrorCode::NotTerminal),
            ETXTBSY => Ok(ErrorCode::TextFileBusy),
            EFBIG => Ok(ErrorCode::FileTooLarge),
            ENOSPC => Ok(ErrorCode::NoSpaceOnDevice),
            ESPIPE => Ok(ErrorCode::IllegalSeek),
            EROFS => Ok(ErrorCode::ReadOnlyFileSystem),
            EMLINK => Ok(ErrorCode::TooManyLinks),
            EPIPE => Ok(ErrorCode::BrokenPipe),
            EDOM => Ok(ErrorCode::MathArgDomainErr),
            ERANGE => Ok(ErrorCode::ValueOutOfRange),
            ENOMSG => Ok(ErrorCode::NoMessageAvailable),
            EIDRM => Ok(ErrorCode::IdentifierRemoved),
            ECHRNG => Ok(ErrorCode::OutOfRangeChannel),
            EL2NSYNC => Ok(ErrorCode::Level2NotSynchronized),
            EL3HLT => Ok(ErrorCode::Level3Halted),
            EL3RST => Ok(ErrorCode::Level3Reset),
            ELNRNG => Ok(ErrorCode::InvalidLinkNumber),
            EUNATCH => Ok(ErrorCode::InvalidProtocolDriver),
            ENOCSI => Ok(ErrorCode::NoStructAvailable),
            EL2HLT => Ok(ErrorCode::Level2Halted),
            EDEADLK => Ok(ErrorCode::Deadlock),
            ENOLCK => Ok(ErrorCode::LockNotAvailable),
            EBADE => Ok(ErrorCode::InvalidExchange),
            EBADR => Ok(ErrorCode::InvalidRequestDescriptor),
            EXFULL => Ok(ErrorCode::ExchangeFull),
            ENOANO => Ok(ErrorCode::InvalidAnode),
            EBADRQC => Ok(ErrorCode::InvalidRequestCode),
            EBADSLT => Ok(ErrorCode::InvalidSlot),
            EDEADLOCK => Ok(ErrorCode::DeadlockWouldOccur),
            EBFONT => Ok(ErrorCode::BadFontFormat),
            ENOSTR => Ok(ErrorCode::NoStreamDeviceAvailable),
            ENODATA => Ok(ErrorCode::NoDataAvailable),
            ETIME => Ok(ErrorCode::TimerExpired),
            ENOSR => Ok(ErrorCode::NoStreamResources),
            ENONET => Ok(ErrorCode::NoNetwork),
            ENOPKG => Ok(ErrorCode::MissingPackage),
            EREMOTE => Ok(ErrorCode::RemoteObject),
            ENOLINK => Ok(ErrorCode::NoLink),
            EADV => Ok(ErrorCode::AdvertiseErr),
            ESRMNT => Ok(ErrorCode::MountErr),
            ECOMM => Ok(ErrorCode::CommunicationErr),
            EPROTO => Ok(ErrorCode::ProtocolErr),
            EMULTIHOP => Ok(ErrorCode::MultipleHopAttemped),
            ELBIN => Ok(ErrorCode::InodeRemote),
            EDOTDOT => Ok(ErrorCode::RfsErr),
            EBADMSG => Ok(ErrorCode::InvalidMessage),
            EFTYPE => Ok(ErrorCode::InvalidFileType),
            ENOTUNIQ => Ok(ErrorCode::NonUniqueName),
            EBADFD => Ok(ErrorCode::InvalidFileDescriptor),
            EREMCHG => Ok(ErrorCode::RemoteAddressChanged),
            ELIBACC => Ok(ErrorCode::LibraryAccessErr),
            ELIBBAD => Ok(ErrorCode::InvalidLibraryAccess),
            ELIBSCN => Ok(ErrorCode::CorruptedLibSection),
            ELIBMAX => Ok(ErrorCode::ExcessiveLibraryLinkCount),
            ELIBEXEC => Ok(ErrorCode::InvalidExecSharedLibrary),
            ENOSYS => Ok(ErrorCode::InvalidSysCall),
            ENOTEMPTY => Ok(ErrorCode::DirectoryNotEmpty),
            ENAMETOOLONG => Ok(ErrorCode::NameTooLong),
            ELOOP => Ok(ErrorCode::SymbolicLinkLoop),
            EOPNOTSUPP => Ok(ErrorCode::OperationNotSupportedOnSocket),
            EPFNOSUPPORT => Ok(ErrorCode::ProtocolFamilyNotSupported),
            ECONNRESET => Ok(ErrorCode::ConnectionReset),
            ENOBUFS => Ok(ErrorCode::NoBufferSpace),
            EAFNOSUPPORT => Ok(ErrorCode::AddressFamilyNotSupported),
            EPROTOTYPE => Ok(ErrorCode::BadProtocolType),
            ENOTSOCK => Ok(ErrorCode::NotSocketFile),
            ENOPROTOOPT => Ok(ErrorCode::ProtocolOptionNotAvailable),
            ESHUTDOWN => Ok(ErrorCode::TransportEndpointShutdown),
            ECONNREFUSED => Ok(ErrorCode::ConnectionRefused),
            EADDRINUSE => Ok(ErrorCode::AddressInUse),
            ECONNABORTED => Ok(ErrorCode::ConnectionAborted),
            ENETUNREACH => Ok(ErrorCode::NetworkUnreachable),
            ENETDOWN => Ok(ErrorCode::NetworkDown),
            ETIMEDOUT => Ok(ErrorCode::OperationTimedOut),
            EHOSTDOWN => Ok(ErrorCode::HostDown),
            EHOSTUNREACH => Ok(ErrorCode::HostUnreachable),
            EINPROGRESS => Ok(ErrorCode::OperationInProgress),
            EALREADY => Ok(ErrorCode::OperationAlreadyInProgress),
            EDESTADDRREQ => Ok(ErrorCode::DestinationAddressRequired),
            EMSGSIZE => Ok(ErrorCode::MessageTooLong),
            EPROTONOSUPPORT => Ok(ErrorCode::ProtocolNotSupported),
            ESOCKTNOSUPPORT => Ok(ErrorCode::SocketTypeNotSupported),
            EADDRNOTAVAIL => Ok(ErrorCode::AddressNotAvailable),
            ENETRESET => Ok(ErrorCode::NetworkReset),
            EISCONN => Ok(ErrorCode::TransportEndpointConnected),
            ENOTCONN => Ok(ErrorCode::TransportEndpointNotConnected),
            ETOOMANYREFS => Ok(ErrorCode::TooManyReferences),
            EUSERS => Ok(ErrorCode::TooManyUsers),
            EDQUOT => Ok(ErrorCode::QuotaExceeded),
            ESTALE => Ok(ErrorCode::StaleHandle),
            ENOTSUP => Ok(ErrorCode::OperationNotSupported),
            ENOMEDIUM => Ok(ErrorCode::MediumNotFound),
            EILSEQ => Ok(ErrorCode::IllegalByteSequence),
            EOVERFLOW => Ok(ErrorCode::ValueOverflow),
            ECANCELED => Ok(ErrorCode::OperationCanceled),
            ENOTRECOVERABLE => Ok(ErrorCode::UnrecoverableState),
            EOWNERDEAD => Ok(ErrorCode::DeadOwner),
            ESTRPIPE => Ok(ErrorCode::StreamPipeErr),
            _ => Err(invalid_error_code(value)),
        }
    }
}

///
/// # Description
///
/// Constructs the error returned when a value is not a valid Nanvix error code.
///
/// # Parameters
///
/// - `value`: Error code (unused).
///
/// # Returns
///
/// Default error.
///
pub fn invalid_error_code(_value: i32) -> Error {
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

impl From<ErrorCode> for i32 {
    fn from(errno: ErrorCode) -> Self {
        errno as i32
    }
}

impl From<ErrorCode> for i64 {
    fn from(errno: ErrorCode) -> Self {
        errno as i64
    }
}

impl From<ErrorCode> for i16 {
    fn from(errno: ErrorCode) -> Self {
        errno as i16
    }
}

impl From<ErrorCode> for u16 {
    fn from(errno: ErrorCode) -> Self {
        errno as u16
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that Linux errno values that collide with unrelated Nanvix errno values are translated
    /// by semantic name instead of by raw number.
    #[cfg(target_os = "linux")]
    #[test]
    fn try_from_linux_errno_translates_socket_family() {
        assert_eq!(
            ErrorCode::try_from_linux_errno(libc::EINPROGRESS).unwrap(),
            ErrorCode::OperationInProgress
        );
        assert_eq!(
            ErrorCode::try_from_linux_errno(libc::EALREADY).unwrap(),
            ErrorCode::OperationAlreadyInProgress
        );
        assert_eq!(
            ErrorCode::try_from_linux_errno(libc::EISCONN).unwrap(),
            ErrorCode::TransportEndpointConnected
        );
    }

    /// Tests that unknown Linux errno values do not fall through into Nanvix errno numbering.
    #[cfg(target_os = "linux")]
    #[test]
    fn try_from_linux_errno_rejects_unknown_values() {
        assert!(ErrorCode::try_from_linux_errno(41).is_err());
    }

    /// Tests that Winsock errno values are translated by semantic name.
    #[cfg(target_os = "windows")]
    #[test]
    fn try_from_winsock_errno_translates_socket_family() {
        let cases: [(i32, ErrorCode); 12] = [
            (winsock_errno::WSAEAFNOSUPPORT.0, ErrorCode::AddressFamilyNotSupported),
            (winsock_errno::WSAEADDRINUSE.0, ErrorCode::AddressInUse),
            (winsock_errno::WSAEDESTADDRREQ.0, ErrorCode::DestinationAddressRequired),
            (winsock_errno::WSAEHOSTUNREACH.0, ErrorCode::HostUnreachable),
            (winsock_errno::WSAEISCONN.0, ErrorCode::TransportEndpointConnected),
            (winsock_errno::WSAENETDOWN.0, ErrorCode::NetworkDown),
            (winsock_errno::WSAENETRESET.0, ErrorCode::NetworkReset),
            (winsock_errno::WSAEPROTONOSUPPORT.0, ErrorCode::ProtocolNotSupported),
            (winsock_errno::WSAEPROTOTYPE.0, ErrorCode::BadProtocolType),
            (winsock_errno::WSAESHUTDOWN.0, ErrorCode::TransportEndpointShutdown),
            (winsock_errno::WSAESOCKTNOSUPPORT.0, ErrorCode::SocketTypeNotSupported),
            (winsock_errno::WSAETIMEDOUT.0, ErrorCode::OperationTimedOut),
        ];

        for (winsock_error, expected) in cases {
            assert_eq!(ErrorCode::try_from_winsock_errno(winsock_error).unwrap(), expected);
        }
    }

    /// Tests that `WSAEWOULDBLOCK` is connect-specific: generic I/O sees `TryAgain`, while
    /// `connect()` sees `OperationInProgress`.
    #[cfg(target_os = "windows")]
    #[test]
    fn try_from_winsock_connect_errno_handles_wouldblock() {
        assert_eq!(
            ErrorCode::try_from_winsock_errno(winsock_errno::WSAEWOULDBLOCK.0).unwrap(),
            ErrorCode::TryAgain
        );
        assert_eq!(
            ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEWOULDBLOCK.0).unwrap(),
            ErrorCode::OperationInProgress
        );
        assert_eq!(
            ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEINVAL.0).unwrap(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEALREADY.0).unwrap(),
            ErrorCode::OperationAlreadyInProgress
        );
    }

    /// Tests that unknown Winsock errno values do not fall through into Nanvix errno numbering.
    #[cfg(target_os = "windows")]
    #[test]
    fn try_from_winsock_errno_rejects_unknown_values() {
        assert!(ErrorCode::try_from_winsock_errno(99999).is_err());
    }
}

//==================================================================================================
// External Function Specifications (Verus)
//==================================================================================================

#[cfg(verus_keep_ghost)]
use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

#[verifier::external_type_specification]
pub struct ExError(crate::Error);

#[verifier::external_type_specification]
pub struct ExErrorCode(crate::ErrorCode);

/// External specification for Error::new.
pub assume_specification[ Error::new ](code: ErrorCode, reason: &'static str) -> (result: Error)
    ensures
        result.code == code,
        result.reason == reason,
;

} // verus!

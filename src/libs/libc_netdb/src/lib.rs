// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    mem,
    ptr,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    netdb::addrinfo,
    netinet_in::{
        in_addr,
        sockaddr_in,
        sockopt_levels::{
            IPPROTO_TCP,
            IPPROTO_UDP,
        },
    },
    sys_socket::{
        sa_family_t,
        sockaddr,
        socket_address_family::{
            AF_INET,
            AF_UNSPEC,
        },
        socket_types::{
            SOCK_DGRAM,
            SOCK_RAW,
            SOCK_STREAM,
        },
        socklen_t,
    },
    sys_types::c_size_t,
};
use ::syslog::{
    trace_libcall,
    trace_syscall,
};

//==================================================================================================
// External Functions
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn inet_aton(cp: *const c_char, inp: *mut in_addr) -> c_int;
}

//==================================================================================================
// Constants
//==================================================================================================

// Address-information flags for `getaddrinfo()`. These mirror the values in `include/netdb.h`.

/// Returns socket addresses suitable for binding a passive (listening) socket.
const AI_PASSIVE: c_int = 0x0001;
/// Requests the canonical name of the host.
const AI_CANONNAME: c_int = 0x0002;
/// Interprets `node` only as a numeric address string (never a host name).
const AI_NUMERICHOST: c_int = 0x0004;
/// Interprets `service` only as a numeric port string (never a service name).
const AI_NUMERICSERV: c_int = 0x0008;
/// Returns IPv4-mapped IPv6 addresses when no IPv6 addresses are found.
const AI_V4MAPPED: c_int = 0x0800;
/// Returns both IPv4 and IPv6 addresses (used together with `AI_V4MAPPED`).
const AI_ALL: c_int = 0x0100;
/// Returns addresses only for configured address families.
const AI_ADDRCONFIG: c_int = 0x0400;
/// Set of all recognized `getaddrinfo()` flags.
const AI_MASK: c_int = AI_PASSIVE
    | AI_CANONNAME
    | AI_NUMERICHOST
    | AI_NUMERICSERV
    | AI_V4MAPPED
    | AI_ALL
    | AI_ADDRCONFIG;

// Error codes returned by `getaddrinfo()`. These mirror the values in `include/netdb.h`.

/// The `flags` field of the `hints` structure had an invalid value.
const EAI_BADFLAGS: c_int = -1;
/// The name does not resolve for the supplied parameters.
const EAI_NONAME: c_int = -2;
/// A non-recoverable error occurred.
const EAI_FAIL: c_int = -4;
/// The requested address family is not supported.
const EAI_FAMILY: c_int = -6;
/// The requested socket type is not supported.
const EAI_SOCKTYPE: c_int = -7;
/// The requested service is not available for the requested socket type.
const EAI_SERVICE: c_int = -8;
/// A memory-allocation failure occurred.
const EAI_MEMORY: c_int = -10;

//==================================================================================================
// Global State
//==================================================================================================

/// Error code reported by the legacy host-resolution functions (`gethostbyname`).
#[unsafe(no_mangle)]
pub static mut h_errno: c_int = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Frees the memory allocated for the linked list of addrinfo structures returned by `getaddrinfo()`.
///
/// # Parameters
///
/// - `res`: Pointer to the linked list of addrinfo structures to be freed.
///
/// # Returns
///
/// This function does not return a value.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `res` points to a valid linked list of addrinfo structures previously allocated by `getaddrinfo()`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn freeaddrinfo(res: *mut addrinfo) {
    // Each node returned by `getaddrinfo()` is a single allocation whose `ai_addr` and
    // `ai_canonname` buffers live inside the same block, so freeing the node pointer releases the
    // whole entry. Walk the list, capturing `ai_next` before releasing each node.
    let mut cursor: *mut addrinfo = res;
    while !cursor.is_null() {
        let next: *mut addrinfo = (*cursor).ai_next;
        free(cursor as *mut c_void);
        cursor = next;
    }
}

///
/// # Description
///
/// Translates the name of a service location (such as a host name) and/or a service name
/// into a set of socket addresses.
///
/// # Parameters
///
/// - `node`: Pointer to a null-terminated string containing a host name or address string.
/// - `service`: Pointer to a null-terminated string containing a service name or port number.
/// - `hints`: Pointer to an addrinfo structure that specifies criteria for selecting the socket address structures returned.
/// - `res`: Pointer to a pointer where the resulting list of addrinfo structures will be stored.
///
/// # Returns
///
/// The `getaddrinfo()` function returns `0` on success. On error, it returns a nonzero error code.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `node` and `service` are valid null-terminated strings (if not null).
/// - `hints` points to a valid addrinfo structure (if not null).
/// - `res` points to a valid pointer to store the result.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const addrinfo,
    res: *mut *mut addrinfo,
) -> c_int {
    // The caller must provide somewhere to store the resulting list.
    if res.is_null() {
        return EAI_FAIL;
    }
    // Start from an empty list so the caller never observes a stale pointer on error.
    *res = ptr::null_mut();

    // At least one of `node` and `service` must be supplied.
    if node.is_null() && service.is_null() {
        return EAI_NONAME;
    }

    // Read the criteria from `hints`, defaulting to an unconstrained lookup when it is null.
    let (flags, family, hint_socktype, hint_protocol): (c_int, c_int, c_int, c_int) =
        if hints.is_null() {
            (0, AF_UNSPEC, 0, 0)
        } else {
            ((*hints).ai_flags, (*hints).ai_family, (*hints).ai_socktype, (*hints).ai_protocol)
        };

    // Reject flags outside the recognized set.
    if flags & !AI_MASK != 0 {
        return EAI_BADFLAGS;
    }

    // Only IPv4 is supported, so anything other than an unspecified or IPv4 family is rejected.
    if family != AF_UNSPEC && family != AF_INET {
        return EAI_FAMILY;
    }

    // Only stream, datagram, and raw socket types are recognized.
    if hint_socktype != 0
        && hint_socktype != SOCK_STREAM
        && hint_socktype != SOCK_DGRAM
        && hint_socktype != SOCK_RAW
    {
        return EAI_SOCKTYPE;
    }

    // Resolve the host and service. No name resolver is available, so only numeric forms succeed.
    let addr: in_addr = match resolve_node(node, flags) {
        Ok(addr) => addr,
        Err(code) => return code,
    };
    let port: u16 = match resolve_service(service, flags) {
        Ok(port) => port,
        Err(code) => return code,
    };

    // Determine the (socket type, protocol) pairs to emit, one addrinfo node per pair.
    let mut pairs: [(c_int, c_int); 2] = [(0, 0); 2];
    let count: usize = match fill_socktype_pairs(hint_socktype, hint_protocol, &mut pairs) {
        Ok(count) => count,
        Err(code) => return code,
    };

    // The canonical name, when requested, is attached to the first node only.
    let canon: Option<&[u8]> = if flags & AI_CANONNAME != 0 {
        Some(if node.is_null() {
            if flags & AI_PASSIVE != 0 {
                b"0.0.0.0".as_slice()
            } else {
                b"127.0.0.1".as_slice()
            }
        } else {
            slice::from_raw_parts(node as *const u8, c_str_len(node))
        })
    } else {
        None
    };

    // Build the linked list, freeing any partial result on allocation failure.
    let mut head: *mut addrinfo = ptr::null_mut();
    let mut tail: *mut addrinfo = ptr::null_mut();
    for (index, &(socktype, protocol)) in pairs.iter().take(count).enumerate() {
        let node_canon: Option<&[u8]> = if index == 0 { canon } else { None };
        let entry: *mut addrinfo = alloc_node(socktype, protocol, port, addr, node_canon);
        if entry.is_null() {
            freeaddrinfo(head);
            return EAI_MEMORY;
        }
        if head.is_null() {
            head = entry;
        } else {
            (*tail).ai_next = entry;
        }
        tail = entry;
    }

    *res = head;
    0
}

///
/// # Description
///
/// Retrieves host information corresponding to a network address.
///
/// # Parameters
///
/// - `addr`: Pointer to the network address.
/// - `len`: Length of the address.
/// - `type_`: Address type.
///
/// # Returns
///
/// Returns a pointer to a hostent structure on success, or null on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `addr` points to a valid network address of length `len`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn gethostbyaddr(
    addr: *const c_void,
    len: c_size_t,
    type_: c_int,
) -> *mut c_void {
    ::syslog::debug!("gethostbyaddr(): not implemented");
    ::core::ptr::null_mut()
}

///
/// # Description
///
/// Retrieves host information corresponding to a host name.
///
/// # Parameters
///
/// - `name`: Pointer to a null-terminated string containing the host name.
///
/// # Returns
///
/// Returns a pointer to a hostent structure on success, or null on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `name` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn gethostbyname(name: *const c_char) -> *mut c_void {
    ::syslog::debug!("gethostbyname(): not implemented");
    ::core::ptr::null_mut()
}

///
/// # Description
///
/// Retrieves protocol information corresponding to a protocol name.
///
/// # Parameters
///
/// - `name`: Pointer to a null-terminated string containing the protocol name.
///
/// # Returns
///
/// Returns a pointer to a protoent structure on success, or null on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `name` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getprotobyname(name: *const c_char) -> *mut c_void {
    ::syslog::debug!("getprotobyname(): not implemented");
    ::core::ptr::null_mut()
}

///
/// # Description
///
/// Retrieves service information corresponding to a service name and protocol.
///
/// # Parameters
///
/// - `name`: Pointer to a null-terminated string containing the service name.
/// - `proto`: Pointer to a null-terminated string containing the protocol name.
///
/// # Returns
///
/// Returns a pointer to a servent structure on success, or null on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `name` and `proto` point to valid null-terminated strings.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getservbyname(name: *const c_char, proto: *const c_char) -> *mut c_void {
    ::syslog::debug!("getservbyname(): not implemented");
    ::core::ptr::null_mut()
}

///
/// # Description
///
/// Retrieves service information corresponding to a port and protocol.
///
/// # Parameters
///
/// - `port`: Port number.
/// - `proto`: Pointer to a null-terminated string containing the protocol name.
///
/// # Returns
///
/// Returns a pointer to a servent structure on success, or null on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `proto` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getservbyport(port: c_int, proto: *const c_char) -> *mut c_void {
    ::syslog::debug!("getservbyport(): not implemented");
    ::core::ptr::null_mut()
}

///
/// # Description
///
/// Converts a socket address to a corresponding host and service, in a protocol-independent manner.
///
/// # Parameters
///
/// - `sa`: Pointer to the socket address structure.
/// - `salen`: Length of the socket address structure.
/// - `host`: Pointer to a buffer to store the host name.
/// - `hostlen`: Length of the host buffer.
/// - `serv`: Pointer to a buffer to store the service name.
/// - `servlen`: Length of the service buffer.
/// - `flags`: Flags to modify function behavior.
///
/// # Returns
///
/// Returns `0` on success, or a nonzero error code on failure.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `sa` points to a valid socket address structure of length `salen`.
/// - `host` and `serv` point to valid buffers of length `hostlen` and `servlen`, respectively.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn getnameinfo(
    sa: *const c_void,
    salen: socklen_t,
    host: *mut c_char,
    hostlen: socklen_t,
    serv: *mut c_char,
    servlen: socklen_t,
    flags: c_int,
) -> c_int {
    ::syslog::debug!("getnameinfo(): not implemented");
    ErrorCode::InvalidSysCall.get()
}

///
/// # Description
///
/// Returns a string describing a network-related error code.
///
/// # Parameters
///
/// - `errcode`: The error code to describe.
///
/// # Returns
///
/// Returns a pointer to a null-terminated string describing the error.
///
/// # Safety
///
/// This function is unsafe because it may return a pointer to a static string.
///
/// It is safe to call this function with any valid error code.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn gai_strerror(errcode: c_int) -> *const c_char {
    ::syslog::debug!("gai_strerror(): not implemented");
    ::core::ptr::null()
}

///
/// # Description
///
/// Returns a string describing a host-resolution error code, as found in `h_errno`.
///
/// # Parameters
///
/// - `err`: The error code to describe (`HOST_NOT_FOUND`, `TRY_AGAIN`, `NO_RECOVERY`, `NO_DATA`,
///   or `0`).
///
/// # Returns
///
/// Returns a pointer to a static null-terminated string describing the error.
///
/// # Safety
///
/// This function is unsafe because it returns a pointer to a static string. The returned pointer
/// must not be freed and is valid for the lifetime of the program.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn hstrerror(err: c_int) -> *const c_char {
    let message: &::core::ffi::CStr = match err {
        0 => c"Resolver error 0 (no error)",
        1 => c"Unknown host",                    // HOST_NOT_FOUND
        2 => c"Host name lookup failure",        // TRY_AGAIN
        3 => c"Unknown server error",            // NO_RECOVERY
        4 => c"No address associated with name", // NO_DATA / NO_ADDRESS
        _ => c"Unknown resolver error",
    };
    message.as_ptr()
}

///
/// # Description
///
/// Returns a pointer to the location where the error code for network database operations is stored.
///
/// # Returns
///
/// Returns a pointer to an integer containing the error code.
///
/// # Safety
///
/// This function is unsafe because it returns a raw pointer.
///
/// It is safe to call this function if the caller expects a pointer to a thread-local or global error variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn __h_errno() -> *mut c_int {
    ::syslog::debug!("__h_errno(): not implemented");
    ::core::ptr::null_mut()
}

//==================================================================================================
// Private Helper Functions
//==================================================================================================

///
/// # Description
///
/// Resolves the `node` argument of `getaddrinfo()` into an IPv4 address. Because no name resolver
/// is available, only numeric address strings are accepted; a null `node` yields the loopback
/// address, or `INADDR_ANY` when `AI_PASSIVE` is set.
///
/// # Parameters
///
/// - `node`: Pointer to a null-terminated host string, or null.
/// - `flags`: The `ai_flags` value supplied through the `hints` structure.
///
/// # Returns
///
/// The resolved [`in_addr`] on success, or an `EAI_*` error code on failure.
///
/// # Safety
///
/// The caller must ensure `node` is either null or a valid null-terminated string.
///
unsafe fn resolve_node(node: *const c_char, flags: c_int) -> Result<in_addr, c_int> {
    if node.is_null() {
        // With `AI_PASSIVE` the result is meant for binding, so use the wildcard address;
        // otherwise use the loopback address.
        let s_addr: u32 = if flags & AI_PASSIVE != 0 {
            // INADDR_ANY (0.0.0.0).
            0
        } else {
            // INADDR_LOOPBACK (127.0.0.1), stored in network byte order.
            0x7f00_0001u32.to_be()
        };
        return Ok(in_addr { s_addr });
    }

    let mut addr: in_addr = in_addr { s_addr: 0 };
    if inet_aton(node, &raw mut addr) != 0 {
        Ok(addr)
    } else {
        // Not a numeric address, and there is no resolver to consult.
        Err(EAI_NONAME)
    }
}

///
/// # Description
///
/// Resolves the `service` argument of `getaddrinfo()` into a port number. Because no service
/// database is available, only numeric port strings are accepted; a null `service` yields port 0.
///
/// # Parameters
///
/// - `service`: Pointer to a null-terminated service string, or null.
/// - `flags`: The `ai_flags` value supplied through the `hints` structure.
///
/// # Returns
///
/// The resolved port in host byte order on success, or an `EAI_*` error code on failure.
///
/// # Safety
///
/// The caller must ensure `service` is either null or a valid null-terminated string.
///
unsafe fn resolve_service(service: *const c_char, flags: c_int) -> Result<u16, c_int> {
    if service.is_null() {
        return Ok(0);
    }

    match parse_port(service) {
        Some(port) => Ok(port),
        None => {
            if flags & AI_NUMERICSERV != 0 {
                // A numeric service was required but the string was not numeric.
                Err(EAI_NONAME)
            } else {
                // Named services cannot be resolved without a services database.
                Err(EAI_SERVICE)
            }
        },
    }
}

///
/// # Description
///
/// Parses a decimal port number from a null-terminated string.
///
/// # Parameters
///
/// - `service`: Pointer to a null-terminated string.
///
/// # Returns
///
/// The parsed port on success, or [`None`] if the string is empty, contains a non-digit, or the
/// value exceeds 65535.
///
/// # Safety
///
/// The caller must ensure `service` points to a valid null-terminated string.
///
unsafe fn parse_port(service: *const c_char) -> Option<u16> {
    let mut index: usize = 0;
    let mut value: u32 = 0;
    let mut digits: usize = 0;
    loop {
        let byte: u8 = *service.add(index) as u8;
        if byte == 0 {
            break;
        }
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(byte - b'0');
        if value > u32::from(u16::MAX) {
            return None;
        }
        digits += 1;
        index += 1;
    }

    if digits == 0 {
        return None;
    }

    Some(value as u16)
}

///
/// # Description
///
/// Determines the `(socket type, protocol)` pairs that `getaddrinfo()` should emit for the given
/// hints. When the socket type is unspecified, both stream (TCP) and datagram (UDP) entries are
/// produced unless a protocol pins a single one.
///
/// # Parameters
///
/// - `socktype`: The requested socket type, or 0 if unspecified.
/// - `protocol`: The requested protocol, or 0 if unspecified.
/// - `out`: Destination array that receives the pairs.
///
/// # Returns
///
/// The number of pairs written to `out`, or an `EAI_*` error code when the requested protocol
/// cannot be represented by the requested socket type.
///
fn fill_socktype_pairs(
    socktype: c_int,
    protocol: c_int,
    out: &mut [(c_int, c_int); 2],
) -> Result<usize, c_int> {
    if socktype == SOCK_STREAM && protocol != 0 && protocol != IPPROTO_TCP {
        return Err(EAI_SOCKTYPE);
    }
    if socktype == SOCK_DGRAM && protocol != 0 && protocol != IPPROTO_UDP {
        return Err(EAI_SOCKTYPE);
    }

    if socktype != 0 {
        out[0] = (socktype, resolve_protocol(socktype, protocol));
        Ok(1)
    } else if protocol == IPPROTO_TCP {
        out[0] = (SOCK_STREAM, IPPROTO_TCP);
        Ok(1)
    } else if protocol == IPPROTO_UDP {
        out[0] = (SOCK_DGRAM, IPPROTO_UDP);
        Ok(1)
    } else if protocol != 0 {
        Err(EAI_SERVICE)
    } else {
        out[0] = (SOCK_STREAM, resolve_protocol(SOCK_STREAM, protocol));
        out[1] = (SOCK_DGRAM, resolve_protocol(SOCK_DGRAM, protocol));
        Ok(2)
    }
}

///
/// # Description
///
/// Selects the protocol for an addrinfo entry, honoring an explicit protocol and otherwise
/// inferring it from the socket type.
///
/// # Parameters
///
/// - `socktype`: The socket type of the entry.
/// - `protocol`: The requested protocol, or 0 if unspecified.
///
/// # Returns
///
/// The protocol to record in the entry.
///
fn resolve_protocol(socktype: c_int, protocol: c_int) -> c_int {
    if protocol != 0 {
        protocol
    } else if socktype == SOCK_STREAM {
        IPPROTO_TCP
    } else if socktype == SOCK_DGRAM {
        IPPROTO_UDP
    } else {
        0
    }
}

///
/// # Description
///
/// Allocates and initializes a single addrinfo node. The node, its [`sockaddr_in`], and its
/// optional canonical name are packed into one allocation so that `freeaddrinfo()` can release the
/// whole entry with a single call to `free()`.
///
/// # Parameters
///
/// - `socktype`: Socket type to record in the node.
/// - `protocol`: Protocol to record in the node.
/// - `port`: Port number in host byte order.
/// - `addr`: IPv4 address in network byte order.
/// - `canon`: Optional canonical name bytes (without the terminating NUL).
///
/// # Returns
///
/// A pointer to the initialized node, or null if allocation fails.
///
/// # Safety
///
/// This function allocates memory with `malloc` and writes through raw pointers.
///
unsafe fn alloc_node(
    socktype: c_int,
    protocol: c_int,
    port: u16,
    addr: in_addr,
    canon: Option<&[u8]>,
) -> *mut addrinfo {
    let ai_size: usize = mem::size_of::<addrinfo>();
    let sa_size: usize = mem::size_of::<sockaddr_in>();
    let canon_size: usize = canon.map_or(0, |bytes| bytes.len() + 1);
    let total: usize = ai_size + sa_size + canon_size;

    let block: *mut u8 = malloc(total as c_size_t) as *mut u8;
    if block.is_null() {
        return ptr::null_mut();
    }

    // Carve the block into the addrinfo header, the socket address, and the canonical name.
    let sa_ptr: *mut sockaddr_in = block.add(ai_size) as *mut sockaddr_in;
    let canon_ptr: *const c_char = match canon {
        Some(bytes) => {
            let dst: *mut u8 = block.add(ai_size + sa_size);
            ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            *dst.add(bytes.len()) = 0;
            dst as *const c_char
        },
        None => ptr::null(),
    };

    // `sockaddr_in` is packed, so build the value and write it as a whole.
    let sin: sockaddr_in = sockaddr_in {
        sin_len: sa_size as u8,
        sin_family: AF_INET as sa_family_t,
        sin_port: port.to_be(),
        sin_addr: addr,
        sin_zero: [0; 8],
    };
    ptr::write(sa_ptr, sin);

    let info: addrinfo = addrinfo {
        ai_flags: 0,
        ai_family: AF_INET,
        ai_socktype: socktype,
        ai_protocol: protocol,
        ai_addrlen: sa_size as socklen_t,
        ai_canonname: canon_ptr,
        ai_addr: sa_ptr as *const sockaddr,
        ai_next: ptr::null_mut(),
    };
    ptr::write(block as *mut addrinfo, info);

    block as *mut addrinfo
}

///
/// # Description
///
/// Computes the length of a null-terminated string, excluding the terminating NUL.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated string.
///
/// # Returns
///
/// The number of bytes preceding the terminating NUL.
///
/// # Safety
///
/// The caller must ensure `s` points to a valid null-terminated string.
///
unsafe fn c_str_len(s: *const c_char) -> usize {
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

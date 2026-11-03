// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests conversion from `SocketAddrV4` to `sockaddr_in`.
#[test]
fn test_ipv4_socket_addr_conversion() {
    let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
    let test_addr: sockaddr_in = sockaddr_in::try_from(&expected_addr)
        .expect("conversion from socket address should succeed");
    assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
}

/// Tets conversion from `sockaddr_in` to `SocketAddrV4`.
#[test]
fn test_ipv4_sockaddr_conversion() {
    let test_addr: sockaddr_in = sockaddr_in {
        sin_len: mem::size_of::<sockaddr_in>() as u8,
        sin_family: socket_address_family::AF_INET
            .try_into()
            .expect("converting address family should succeed"),
        sin_port: 80u16.to_be(),
        sin_addr: in_addr {
            s_addr: u32::from_be_bytes([192, 168, 1, 1]).to_be(),
        },
        sin_zero: [0; 8],
    };
    let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
    assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
}

/// Tests conversion from `SocketAddrV4` to `sockaddr`.
#[test]
fn test_ipv4_socket_addr_into_sockaddr() {
    let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
    let test_addr: sockaddr = sockaddr::from(&expected_addr);
    assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
}

/// Tests conversion from `sockaddr` to `SocketAddrV4`.
#[test]
fn test_ipv4_sockaddr_into_socket_addr() {
    let test_addr: sockaddr = sockaddr {
        sa_len: mem::size_of::<sockaddr>() as u8,
        sa_family: socket_address_family::AF_INET
            .try_into()
            .expect("converting address family should succeed"),
        sa_data: [0, 80, 192, 168, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
    assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
}

/// Tests conversion from `SocketAddr` to `sockaddr`.
#[test]
fn test_socket_addr_into_sockaddr() {
    let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
    let test_addr: sockaddr = sockaddr::from(&SocketAddr::V4(expected_addr));
    assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
}

/// Tests conversion for `SockAddrUnix` to `sockaddr_un`.
#[test]
fn test_unix_socket_addr_conversion() {
    let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
    let test_addr: sockaddr_un = sockaddr_un::try_from(&expected_addr)
        .expect("conversion from socket address should succeed");
    assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
}

/// Tests conversion from `sockaddr_un` to `SocketAddrUnix`.
#[test]
fn test_unix_sockaddr_conversion() {
    let test_addr: sockaddr_un = sockaddr_un {
        sun_len: mem::size_of::<sockaddr_un>() as u8,
        sun_family: socket_address_family::AF_UNIX
            .try_into()
            .expect("converting address family should succeed"),
        sun_path: {
            let mut path: [u8; SUNPATHLEN] = [0; SUNPATHLEN];
            let bytes = "/tmp/socket".as_bytes();
            path[..bytes.len()].copy_from_slice(bytes);
            path.map(|b| b as i8)
        },
    };
    let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
    assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
}

/// Tests conversion from `SocketAddrUnix` to `sockaddr`.
#[test]
fn test_unix_socket_addr_into_sockaddr() {
    let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
    let test_addr: sockaddr = sockaddr::from(&expected_addr);
    assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
}

/// Tests conversion from `sockaddr` to `SocketAddrUnix`.
#[test]
fn test_unix_sockaddr_into_socket_addr() {
    let test_addr: sockaddr = sockaddr {
        sa_len: mem::size_of::<sockaddr>() as u8,
        sa_family: socket_address_family::AF_UNIX as u8,
        sa_data: {
            let mut data = [0; 14];
            let bytes = "/tmp/socket".as_bytes();
            data[..bytes.len()].copy_from_slice(bytes);
            data
        },
    };
    let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
    assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
}

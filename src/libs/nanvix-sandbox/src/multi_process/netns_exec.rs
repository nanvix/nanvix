// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-process sandbox implementation.
//!
//! This module provides sandboxing functionality where Linux Daemon and User VM instances
//! are spawned as separate processes. This is the default mode of operation for Nanvix Daemon.

//==================================================================================================
// Imports
//==================================================================================================

use crate::netns::NetnsInfo;
use ::std::{
    ffi::CString,
    fs::File,
    io,
    io::Write,
    net::{
        Ipv4Addr,
        SocketAddr,
        TcpStream,
    },
    os::unix::io::{
        AsRawFd,
        FromRawFd,
        RawFd,
    },
    process::{
        Command as StdCommand,
        Output,
    },
    time::Duration,
};
use ::log::error;
use ::tokio::{
    process::Command,
    task,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// TAP device used by the restored L2 system VM.
const TUN_DEVICE_PATH: &str = "/dev/net/tun";

///
/// # Description
///
/// Join a named network namespace by calling setns() on /var/run/netns/<name>.
///
/// This method is intended to be called inside the target process, but before calling `exec`. We
/// can achieve this behaviour using a `pre_exec` hook, as explained in detail below.
///
/// # Arguments
///
/// - `ns_name`: name of the network namespace to enter.
///
/// # Safety
///
/// This function is unsafe because it does some low-level handling of raw file descriptors. In
/// addition, it is inserted as a pre-exec hook in tokio's command, which is also an unsafe
/// operation.
///
unsafe fn setns_by_name(ns_name: &str) -> io::Result<()> {
    let ns_path: String = format!("/var/run/netns/{}", ns_name);

    // Open with O_CLOEXEC so it doesn't leak into the exec'd program.
    let c_path: CString = CString::new(ns_path.clone()).map_err(|_| {
        let reason: String = format!("invalid namespace path (path={ns_path})");
        error!("setns_by_name(): {reason}");
        io::Error::new(io::ErrorKind::InvalidInput, reason)
    })?;

    let fd: RawFd = libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
    if fd < 0 {
        let open_err: io::Error = io::Error::last_os_error();
        error!("setns_by_name(): error opening netns file (error={open_err:?})");
        return Err(open_err);
    }

    // Join the network namespace.
    // Note: setns() returns 0 on success, -1 on error (errno set).
    let rc: libc::c_int = libc::setns(fd, libc::CLONE_NEWNET);
    let saved_err: Option<io::Error> = if rc != 0 {
        Some(io::Error::last_os_error())
    } else {
        None
    };

    // Close fd regardless.
    let close_rc: libc::c_int = libc::close(fd);
    if close_rc != 0 {
        let close_err: io::Error = io::Error::last_os_error();
        error!("setns_by_name(): error closing netns file descriptor (error={close_err:?})");
    }

    if let Some(e) = saved_err {
        error!("setns_by_name(): error entering network namespace (name={ns_name}, error={e:?})");
        return Err(e);
    }

    Ok(())
}

fn new_ifreq(name: &str) -> io::Result<libc::ifreq> {
    if name.len() >= libc::IFNAMSIZ {
        let reason: String = format!(
            "network interface name is too long (name={name:?}, max={})",
            libc::IFNAMSIZ - 1
        );
        error!("new_ifreq(): {reason}");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
    }

    let mut ifreq: libc::ifreq = unsafe { ::std::mem::zeroed() };
    for (dst, src) in ifreq.ifr_name.iter_mut().zip(name.as_bytes().iter().copied()) {
        *dst = src as libc::c_char;
    }

    Ok(ifreq)
}

fn create_sockaddr(ip_addr: Ipv4Addr) -> libc::sockaddr {
    let addr_in: libc::sockaddr_in = libc::sockaddr_in {
        sin_family: libc::AF_INET as u16,
        sin_port: 0,
        sin_addr: unsafe { ::std::mem::transmute::<[u8; 4], libc::in_addr>(ip_addr.octets()) },
        sin_zero: [0; 8],
    };

    unsafe { ::std::mem::transmute(addr_in) }
}

fn create_socket(domain: libc::c_int) -> io::Result<File> {
    let fd: RawFd = unsafe { libc::socket(domain, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { File::from_raw_fd(fd) })
}

fn ioctl_with_ref<T>(fd: RawFd, request: libc::Ioctl, value: &T) -> io::Result<()> {
    let ret: libc::c_int = unsafe { libc::ioctl(fd, request, value) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

fn ioctl_with_mut_ref<T>(fd: RawFd, request: libc::Ioctl, value: &mut T) -> io::Result<()> {
    let ret: libc::c_int = unsafe { libc::ioctl(fd, request, value) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

fn set_interface_ipv4_address(if_name: &str, ip_addr: Ipv4Addr) -> io::Result<()> {
    let sock: File = create_socket(libc::AF_INET)?;
    let addr: libc::sockaddr = create_sockaddr(ip_addr);
    let mut ifreq: libc::ifreq = new_ifreq(if_name)?;
    ifreq.ifr_ifru.ifru_addr = addr;
    ioctl_with_ref(sock.as_raw_fd(), libc::SIOCSIFADDR as libc::Ioctl, &ifreq)
}

fn set_interface_netmask(if_name: &str, netmask: Ipv4Addr) -> io::Result<()> {
    let sock: File = create_socket(libc::AF_INET)?;
    let addr: libc::sockaddr = create_sockaddr(netmask);
    let mut ifreq: libc::ifreq = new_ifreq(if_name)?;
    ifreq.ifr_ifru.ifru_addr = addr;
    ioctl_with_ref(sock.as_raw_fd(), libc::SIOCSIFNETMASK as libc::Ioctl, &ifreq)
}

fn enable_interface(if_name: &str) -> io::Result<()> {
    let sock: File = create_socket(libc::AF_UNIX)?;
    let mut ifreq: libc::ifreq = new_ifreq(if_name)?;
    ioctl_with_mut_ref(sock.as_raw_fd(), libc::SIOCGIFFLAGS as libc::Ioctl, &mut ifreq)?;

    unsafe {
        if ifreq.ifr_ifru.ifru_flags & libc::IFF_UP as libc::c_short == libc::IFF_UP as libc::c_short
        {
            return Ok(());
        }

        ifreq.ifr_ifru.ifru_flags = libc::IFF_UP as libc::c_short;
    }

    ioctl_with_ref(sock.as_raw_fd(), libc::SIOCSIFFLAGS as libc::Ioctl, &ifreq)
}

fn create_persistent_tap(if_name: &str, host_ip: Ipv4Addr, netmask: Ipv4Addr) -> io::Result<()> {
    let tun_device: CString = CString::new(TUN_DEVICE_PATH).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid tun device path ({TUN_DEVICE_PATH})"),
        )
    })?;

    let fd: RawFd = unsafe { libc::open(tun_device.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    let tap_file: File = unsafe { File::from_raw_fd(fd) };
    let mut ifreq: libc::ifreq = new_ifreq(if_name)?;
    ifreq.ifr_ifru.ifru_flags =
        (libc::IFF_TAP | libc::IFF_NO_PI | libc::IFF_VNET_HDR) as libc::c_short;
    ioctl_with_mut_ref(tap_file.as_raw_fd(), libc::TUNSETIFF, &mut ifreq)?;

    let persist: libc::c_int = 1;
    ioctl_with_ref(tap_file.as_raw_fd(), libc::TUNSETPERSIST, &persist)?;
    set_interface_ipv4_address(if_name, host_ip)?;
    set_interface_netmask(if_name, netmask)?;
    enable_interface(if_name)?;

    Ok(())
}

///
/// # Description
///
/// Spawn a program inside a network namespace.
///
/// This function spawns the provided program inside the provided network namespace without
/// requiring `sudo ip netns exec`. This function relies on executing a hook inside the new process
/// but before calling `exec`. This can be done using a `pre_exec` hook as exposed by tokio's
/// `Command` [1].
///
/// Avoiding the call to `sudo` reduces the overhead of executing a program inside a network
/// namespace, but forces the caller to have `CAP_SYS_ADMIN` + `CAP_NET_ADMIN` privileges.
///
/// [1] https://docs.rs/tokio/latest/tokio/process/struct.Command.html#method.pre_exec
///
/// # Arguments
///
/// - `info`: information on the network namespace.
/// - `program`: binary to execute inside the namespace.
/// - `args`: arguments to pass to the program.
///
/// # Returns
///
/// A Command with the right hook that can be spawned.
///
pub fn command_in_netns(info: &NetnsInfo, program: &str, args: &[String]) -> Command {
    let ns_name: String = info.ns_name().to_string();

    let mut cmd: Command = Command::new(program);
    cmd.args(args);
    // Ensure the child process is killed if the Child handle is dropped without explicit cleanup.
    // This acts as a best-effort safety net during normal unwinding and shutdown paths where drop
    // handlers run, helping to prevent orphaned processes.
    cmd.kill_on_drop(true);

    // SAFETY: inside the `pre-exec` closure we only run the logic to open the network namespace
    // file descriptor and call `setns` on it. It does not allocate any memory, and it only calls
    // async-safe functions: open, setns, and close.
    unsafe {
        cmd.pre_exec(move || {
            setns_by_name(&ns_name)?;
            Ok(())
        });
    }

    cmd
}

///
/// # Description
///
/// Enters the target network namespace in a short-lived blocking worker thread, connects to the
/// provided TCP address, writes a payload, and then exits. This keeps the namespace switch scoped
/// to the helper thread instead of the async runtime threads that run nanvixd itself.
///
/// # Parameters
///
/// - `info`: Information on the target network namespace.
/// - `addr`: TCP address to connect to from inside the namespace.
/// - `payload`: Bytes to send once the connection succeeds.
///
/// # Returns
///
/// Returns `Ok(())` on success or an I/O error if entering the namespace, connecting, or writing
/// the payload fails.
///
pub async fn write_tcp_in_netns(info: &NetnsInfo, addr: &str, payload: &[u8]) -> io::Result<()> {
    let ns_name: String = info.ns_name().to_string();
    let addr: SocketAddr = addr.parse::<SocketAddr>().map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{error} (addr={addr})"))
    })?;
    let payload: Vec<u8> = payload.to_vec();

    task::spawn_blocking(move || {
        unsafe {
            setns_by_name(&ns_name)?;
        }

        let mut stream: TcpStream = TcpStream::connect_timeout(&addr, Duration::from_secs(1))?;
        stream.write_all(&payload)?;
        stream.flush()?;

        Ok(())
    })
    .await
    .map_err(|error| io::Error::other(format!("netns helper thread failed: {error}")))?
}

///
/// # Description
///
/// Enters the target network namespace in a short-lived blocking worker thread and invokes
/// `ch-remote restore` from inside that namespace.
///
/// # Parameters
///
/// - `info`: Information on the target network namespace.
/// - `ch_remote_path`: Path to the `ch-remote` binary.
/// - `api_socket_path`: Path to the Cloud Hypervisor API socket.
/// - `snapshot_path`: Path to the snapshot directory that should be restored.
/// - `tap_name`: Name of the TAP device stored in the snapshot config.
/// - `tap_host_ip`: Host-side IPv4 address for the TAP device.
/// - `tap_netmask`: IPv4 netmask for the TAP device.
///
/// # Returns
///
/// Returns the `ch-remote` process output. On failure, the returned I/O error indicates whether the
/// failure happened while entering the namespace or executing the restore command itself.
///
pub async fn restore_vm_in_netns(
    info: &NetnsInfo,
    ch_remote_path: &str,
    api_socket_path: &str,
    snapshot_path: &str,
    tap_name: &str,
    tap_host_ip: &str,
    tap_netmask: &str,
) -> io::Result<Output> {
    let ns_name: String = info.ns_name().to_string();
    let ch_remote_path: String = ch_remote_path.to_string();
    let api_socket_path: String = api_socket_path.to_string();
    let snapshot_path: String = snapshot_path.to_string();
    let tap_name: String = tap_name.to_string();
    let tap_host_ip: Ipv4Addr = tap_host_ip.parse::<Ipv4Addr>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{error} (tap_host_ip={tap_host_ip})"),
        )
    })?;
    let tap_netmask: Ipv4Addr = tap_netmask.parse::<Ipv4Addr>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{error} (tap_netmask={tap_netmask})"),
        )
    })?;

    task::spawn_blocking(move || {
        unsafe {
            setns_by_name(&ns_name)?;
        }

        create_persistent_tap(&tap_name, tap_host_ip, tap_netmask)?;
        let restore_config: String = format!("source_url=file://{snapshot_path}");

        StdCommand::new(&ch_remote_path)
            .arg("--api-socket")
            .arg(&api_socket_path)
            .arg("restore")
            .arg(&restore_config)
            .output()
    })
    .await
    .map_err(|error| io::Error::other(format!("netns helper thread failed: {error}")))?
}

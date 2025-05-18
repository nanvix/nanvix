// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::socket::Socket,
    wasi::{
        types::{
            Errno,
            Fd,
            FdFlags,
            RiFlags,
            Rights,
            RoFlags,
            SdFlags,
            SiFlags,
        },
        IoVec,
        Pointer,
        Size,
        Slice,
        WasiCtxInner,
    },
};
use ::syscall::sys::socket::Shutdown;

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    pub(crate) fn sock_accept(&mut self, sockfd: Fd, _fdflags: FdFlags) -> Result<Fd, Errno> {
        // Retrieve socket.
        match self.get_socket(sockfd) {
            Some(socket) => {
                // Ensure that we have the right to invoke this operation.
                if !socket.rights_base().sock_accept {
                    ::syslog::error!("sock_accept(): operation not permitted");
                    return Err(Errno::Acces);
                }

                // Accept connection on socket.
                let os_connfd: Socket = match socket.socket().accept() {
                    Ok(connfd) => connfd,
                    Err(errno) => return Err(errno.value().into()),
                };

                // TODO: save fdflags when they are supported.

                let base_rights: Rights = socket.rights_base().clone();
                let inherited_rights: Rights = socket.rights_inheriting().clone();

                Ok(self.insert_socket(os_connfd, &base_rights, &inherited_rights))
            },
            None => {
                ::syslog::error!("sock_accept(): invalid file descriptor");
                Err(Errno::Badf)
            },
        }
    }

    pub(crate) fn sock_recv(
        &self,
        memory: &mut [u8],
        connfd: Fd,
        iovecs: &[IoVec],
        _riflags: RiFlags,
    ) -> Result<(Size, RoFlags), Errno> {
        // Retrieve socket.
        match self.get_socket(connfd) {
            Some(socket) => {
                // Ensure that we have the right to invoke this operation (using alias to fd_read).
                if !socket.rights_base().fd_read {
                    ::syslog::error!("sock_recv(): operation not permitted");
                    return Err(Errno::Acces);
                }

                let mut roflags = RoFlags::from(0);
                let mut total_read: usize = 0;
                for iovec in iovecs {
                    let ptr: Pointer<u8> = iovec.buf();
                    let len: Size = iovec.buf_len();

                    let mut buf: Slice<'_, u8> = Slice::<u8>::for_raw_parts(memory, ptr, len);
                    let buffer: &mut [u8] = match buf.as_mut() {
                        Ok(slice) => slice,
                        Err(_) => {
                            ::syslog::error!("sock_accept(): failed to get slice from memory");
                            return Err(Errno::Inval);
                        },
                    };

                    let nrecv = match socket.socket().recv(buffer) {
                        Ok(nrecv) => nrecv,
                        Err(errno) => {
                            ::syslog::error!(
                                "sock_recv(): failed to receive data on socket: {:?}",
                                errno
                            );
                            return Err(errno.value().into());
                        },
                    };

                    total_read += nrecv;

                    if nrecv != len.value() as usize {
                        roflags.trunc = true;
                    }
                }

                Ok((total_read.into(), roflags))
            },
            None => {
                ::syslog::error!("sock_recv(): invalid file descriptor");
                Err(Errno::Badf)
            },
        }
    }

    pub(crate) fn sock_send(
        &self,
        memory: &[u8],
        connfd: Fd,
        iovecs: &[IoVec],
        _roflags: SiFlags,
    ) -> Result<Size, Errno> {
        // Retrieve socket.
        match self.get_socket(connfd) {
            Some(socket) => {
                // Ensure that we have the right to invoke this operation (using alias to fd_write).
                if !socket.rights_base().fd_write {
                    ::syslog::error!("sock_send(): operation not permitted");
                    return Err(Errno::Acces);
                }

                let mut total_sent: usize = 0;
                for iovec in iovecs {
                    let ptr: Pointer<u8> = iovec.buf();
                    let len: Size = iovec.buf_len();

                    let buf: Slice<'_, u8> = Slice::<u8>::for_raw_parts(memory, ptr, len);
                    let buffer: &[u8] = match buf.as_ref() {
                        Ok(slice) => slice,
                        Err(_) => {
                            ::syslog::error!("sock_send(): failed to get slice from memory");
                            return Err(Errno::Inval);
                        },
                    };

                    let nsent = match socket.socket().send(buffer) {
                        Ok(nsent) => nsent,
                        Err(errno) => {
                            ::syslog::error!(
                                "sock_send(): failed to send data on socket: {:?}",
                                errno
                            );
                            return Err(errno.value().into());
                        },
                    };

                    total_sent += nsent;
                }

                Ok(total_sent.into())
            },
            None => {
                ::syslog::error!("sock_send(): invalid file descriptor");
                Err(Errno::Badf)
            },
        }
    }

    pub(crate) fn sock_shutdown(&self, connfd: Fd, how: SdFlags) -> Result<(), Errno> {
        // Attempt to convert shutdown flags.
        let how: Shutdown = how.into();

        // Retrieve socket.
        match self.get_socket(connfd) {
            Some(socket) => {
                // Ensure that we have the right to invoke this operation.
                if !socket.rights_base().sock_shutdown {
                    ::syslog::error!("sock_shutdown(): operation not permitted");
                    return Err(Errno::Acces);
                }

                match socket.socket().shutdown(how) {
                    Ok(_) => Ok(()),
                    Err(errno) => {
                        ::syslog::error!("sock_shutdown(): failed to shutdown socket: {:?}", errno);
                        Err(errno.value().into())
                    },
                }
            },
            None => {
                ::syslog::error!("sock_shutdown(): invalid file descriptor");
                Err(Errno::Badf)
            },
        }
    }
}

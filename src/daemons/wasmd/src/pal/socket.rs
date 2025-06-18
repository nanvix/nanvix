// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pal::{
    Error,
    RawFd,
};
use ::sys::error::ErrorCode;
use ::syscall::{
    netinet::in_::Protocol,
    sys::socket::{
        syscall,
        AddressFamily,
        Shutdown,
        SocketAddr,
        SocketType,
    },
    unistd,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Socket(RawFd);

//==================================================================================================
// Implementations
//==================================================================================================

impl Socket {
    pub fn new() -> Result<Self, Error> {
        match syscall::socket(AddressFamily::Inet, SocketType::Stream, Protocol::Ip) {
            Ok(sockfd) => {
                ::syslog::info!("created socket with fd {}", sockfd);
                Ok(Self(sockfd))
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn bind(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        match syscall::bind(self.0, addr) {
            Ok(()) => {
                ::syslog::info!("bound socket with fd {} to address {:?}", self.0, addr);
                Ok(())
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn listen(&mut self, backlog: i32) -> Result<(), Error> {
        // Attempt to coerce `backlog`.
        let backlog: usize = match backlog.try_into() {
            Ok(value) => value,
            Err(_) => {
                ::syslog::error!("listen(): invalid backlog (backlog={backlog:?})");
                return Err(Error {
                    errno: ErrorCode::InvalidArgument.get(),
                });
            },
        };

        match syscall::listen(self.0, backlog) {
            Ok(()) => {
                ::syslog::info!("listening on socket with fd {}", self.0);
                Ok(())
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn accept(&self) -> Result<Socket, Error> {
        match syscall::accept(self.0) {
            Ok((connfd, _sockaddr)) => {
                ::syslog::info!("accepted connection on socket with fd {}", connfd);
                Ok(Self(connfd))
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn recv(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        match syscall::recv(self.0, buffer, 0) {
            Ok(len) => {
                ::syslog::info!("received {} bytes on socket with fd {}", len, self.0);
                Ok(len)
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn send(&self, buffer: &[u8]) -> Result<usize, Error> {
        match syscall::send(self.0, buffer, 0) {
            Ok(len) => {
                ::syslog::info!("sent {} bytes on socket with fd {}", len, self.0);
                Ok(len)
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        match syscall::shutdown(self.0, how) {
            Ok(()) => {
                ::syslog::info!("shutdown socket with fd {}", self.0);
                Ok(())
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        match unistd::close(self.0) {
            Ok(()) => {
                ::syslog::info!("closed socket with fd {}", self.0);
            },
            Err(error) => {
                panic!("failed to close socket with fd {}: {:?}", self.0, error);
            },
        }
    }
}

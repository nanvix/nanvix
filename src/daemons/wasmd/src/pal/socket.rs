// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pal::{
    Error,
    RawFd,
};
use ::posix::{
    netinet::in_::Protocol,
    sys::socket::{
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
        match posix::sys::socket::socket(AddressFamily::Inet, SocketType::Stream, Protocol::Ip) {
            Ok(sockfd) => {
                ::nvx::info!("created socket with fd {}", sockfd);
                Ok(Self(sockfd))
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn bind(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        match posix::sys::socket::bind(self.0, addr) {
            Ok(()) => {
                ::nvx::info!("bound socket with fd {} to address {:?}", self.0, addr);
                Ok(())
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn listen(&mut self, backlog: i32) -> Result<(), Error> {
        match posix::sys::socket::listen(self.0, backlog) {
            Ok(()) => {
                ::nvx::info!("listening on socket with fd {}", self.0);
                Ok(())
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn accept(&self) -> Result<Socket, Error> {
        match posix::sys::socket::accept(self.0, None) {
            Ok(connfd) => {
                ::nvx::info!("accepted connection on socket with fd {}", connfd);
                Ok(Self(connfd))
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn recv(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        match posix::sys::socket::recv(self.0, buffer, 0) {
            Ok(len) => {
                ::nvx::info!("received {} bytes on socket with fd {}", len, self.0);
                Ok(len as usize)
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn send(&self, buffer: &[u8]) -> Result<usize, Error> {
        match posix::sys::socket::send(self.0, buffer, 0) {
            Ok(len) => {
                ::nvx::info!("sent {} bytes on socket with fd {}", len, self.0);
                Ok(len as usize)
            },
            Err(e) => Err(Error {
                errno: e.code.get(),
            }),
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        match posix::sys::socket::shutdown(self.0, how) {
            Ok(()) => {
                ::nvx::info!("shutdown socket with fd {}", self.0);
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
                ::nvx::info!("closed socket with fd {}", self.0);
            },
            Err(error) => {
                panic!("failed to close socket with fd {}: {:?}", self.0, error);
            },
        }
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod config;
mod linuxd;
mod microvm;
mod tag;

//==================================================================================================
// Imports
//==================================================================================================

use self::{
    linuxd::LinuxDaemon,
    microvm::Microvm,
};
use ::anyhow::Result;
use ::std::fs;
use ::tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use config::SandboxConfig;
pub use tag::SandboxTag;

//==================================================================================================

pub struct Sandbox {
    _linuxd: LinuxDaemon,
    linuxd_socket: Option<UnixStream>,
    microvm: Option<Microvm>,
    config: SandboxConfig,
}

impl Sandbox {
    pub fn new(config: &SandboxConfig) -> Result<Self> {
        Ok(Self {
            _linuxd: LinuxDaemon::spawn(config.linuxd_sockaddr(), config.sandbox_sockaddr())?,
            linuxd_socket: None,
            microvm: None,
            config: config.clone(),
        })
    }

    pub async fn load(&mut self, program: &str) -> Result<()> {
        if self.linuxd_socket.is_none() {
            match self.config.linuxd_listener().accept().await {
                Ok((socket, _)) => {
                    debug!("accepted connection from linux daemon {:?}", socket.peer_addr());
                    self.linuxd_socket = Some(socket);
                },

                Err(_) => {
                    let reason: String = "failed to accept connection".to_string();
                    error!("{reason}");
                    anyhow::bail!(reason)
                },
            };
        }

        // Peek VM status.
        if let Some(mut microvm) = self.microvm.take() {
            if let Ok(None) = microvm.peek() {
                debug!("microvm is still running");
                self.microvm = Some(microvm);
                return Ok(());
            }
        }

        match Microvm::spawn(program, self.config.linuxd_sockaddr(), self.config.console_file()) {
            Ok(microvm) => {
                self.microvm = Some(microvm);
            },
            Err(_) => {
                let reason: String = "failed to execute process".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(())
    }

    pub fn unload(&mut self) -> Result<()> {
        self.microvm.take();
        self.linuxd_socket.take();
        Ok(())
    }

    pub fn socket(&mut self) -> Result<&mut UnixStream> {
        self.linuxd_socket
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("linuxd socket not connected"))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Some(mut linuxd_socket) = self.linuxd_socket.take() {
            let linuxd_sockaddr: String = self.config.linuxd_sockaddr().to_string();
            tokio::spawn(async move {
                if let Err(e) = linuxd_socket.shutdown().await {
                    error!("failed to shutdown socket ({e:?})");
                }

                debug!("removing socket file {linuxd_sockaddr:?}");
                if let Err(e) = fs::remove_file(linuxd_sockaddr) {
                    error!("failed to remove socket file ({e:?})");
                }
            });
        }
    }
}

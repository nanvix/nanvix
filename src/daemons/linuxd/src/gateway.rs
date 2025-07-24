// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This file implements the gateway poll thread, a background thread that polls the gateway socket
//! and monitors new connections to it. It will then, upon request from the main linuxd thread,
//! send the accepted connections, in the form of socket streams, to the main linuxd thread.
//!
//! Most importantly, this module does nothing else than accepting connections.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::debug;
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
    Waker,
};
use ::std::{
    collections::VecDeque,
    sync::mpsc::{
        channel,
        Receiver,
        Sender,
    },
    thread::{
        self,
        JoinHandle,
    },
};
use ::syscomm::{
    SocketListener,
    SocketStream,
};

//==================================================================================================
// Structures
//==================================================================================================

pub enum GatewayCommand {
    AcceptConn,
    Shutdown,
}

pub struct GatewayHandle {
    // Handle to send commands to the gateway thread.
    pub gw_cmd_tx: Sender<GatewayCommand>,
    // Handle to receive accepted connections from the gateway thread.
    pub gw_conn_rx: Receiver<SocketStream>,
    // Handle to force the gateway thread to wake-up and process pending connections.
    pub waker: Waker,
    // Handle to the underlying polling thread.
    pub gw_thread: JoinHandle<()>,
}

pub struct GatewayPollThread {
    listener: SocketListener,
    poll: Poll,
    events: Events,
    gw_cmd_rx: Receiver<GatewayCommand>,
    gw_conn_tx: Sender<SocketStream>,
    pending_conns: VecDeque<SocketStream>,
    pending_accepts: i32,
}

impl GatewayPollThread {
    pub fn spawn(
        mut listener: SocketListener,
        listener_token: Token,
        waker_token: Token,
    ) -> std::io::Result<GatewayHandle> {
        let poll = Poll::new()?;
        poll.registry().register(
            &mut listener,
            listener_token,
            Interest::READABLE | Interest::WRITABLE,
        )?;

        let waker = Waker::new(poll.registry(), waker_token)?;

        let (gw_cmd_tx, gw_cmd_rx) = channel::<GatewayCommand>();
        let (gw_conn_tx, gw_conn_rx) = channel::<SocketStream>();

        let gw_thread = thread::spawn(move || {
            let reactor = GatewayPollThread {
                listener,
                poll,
                events: Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS),
                gw_cmd_rx,
                gw_conn_tx,
                pending_conns: VecDeque::new(),
                pending_accepts: 0,
            };

            reactor
                .run(listener_token, waker_token)
                .expect("gateway failed");
        });

        Ok(GatewayHandle {
            gw_cmd_tx,
            gw_conn_rx,
            waker,
            gw_thread,
        })
    }

    /// The main linuxd thread accepts connections from user VMs, and the gateway thread
    /// accepts connections from clients.
    ///
    /// To match a client connection to a user VM connection, at the moment, we rely on
    /// both connections being initiatied roughly at the same time (in the future we will
    /// refine the protocol). In particular, we rely on no other user VM, or no other client
    /// trying to connect before this match has been established.
    ///
    /// Still, there is a race between the user VM and the client on who connects first.
    /// To address this race, we keep track of both the pending connections we have
    /// accepted but have not delivered to linuxd, and the connections that linuxd has
    /// requested but we have not delivered yet.
    ///
    pub fn run(mut self, listener_token: Token, waker_token: Token) -> std::io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, None)?;

            for event in self.events.iter() {
                match event.token() {
                    _token if _token == listener_token => {
                        loop {
                            match self.listener.accept() {
                                Ok(mut stream) => {
                                    // We need reads from the gateway threads to block.
                                    stream.set_blocking()?;

                                    // If linuxd is waiting for a connection, send it over the
                                    // channel. If not, push it back to the queue.
                                    if self.pending_accepts > 0 {
                                        let _ = self.gw_conn_tx.send(stream);
                                        self.pending_accepts -= 1;
                                    } else {
                                        self.pending_conns.push_back(stream);
                                    }
                                },
                                // No more connections to accept.
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                                Err(e) => return Err(e.into()),
                            }
                        }
                    },
                    _token if _token == waker_token => {
                        while let Ok(cmd) = self.gw_cmd_rx.try_recv() {
                            match cmd {
                                // Linuxd has requested a connection from the gateway.
                                GatewayCommand::AcceptConn => {
                                    if let Some(conn) = self.pending_conns.pop_front() {
                                        let _ = self.gw_conn_tx.send(conn);
                                    } else {
                                        self.pending_accepts += 1;
                                        // The calling thread is blocked on a `recv` command in the
                                        // reception queue. We will send the connection as soon as
                                        // we accept one.
                                    }
                                },
                                // Linuxd has requested us to shut down.
                                GatewayCommand::Shutdown => {
                                    debug!("gateway thread shutting down");
                                    return Ok(());
                                },
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use mio::{
    Events,
    Interest,
    Poll,
    Token,
    Waker,
};
use std::{
    collections::VecDeque,
    sync::mpsc::{
        Receiver,
        Sender,
        channel,
    },
    thread::{
        self,
        JoinHandle,
    }
};
use ::syscomm::{SocketListener, SocketStream};

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

pub struct GatewayReactor {
    listener: SocketListener,
    poll: Poll,
    events: Events,
    gw_cmd_rx: Receiver<GatewayCommand>,
    gw_conn_tx: Sender<SocketStream>,
    pending_conns: VecDeque<SocketStream>,
}

impl GatewayReactor {
    pub fn spawn(
        mut listener: SocketListener,
        listener_token: Token,
        waker_token: Token,
    ) -> std::io::Result<GatewayHandle> {
        let poll = Poll::new()?;
        poll
            .registry()
            .register(&mut listener, listener_token, Interest::READABLE | Interest::WRITABLE)?;

        let waker = Waker::new(poll.registry(), waker_token)?;

        let (gw_cmd_tx, gw_cmd_rx) = channel::<GatewayCommand>();
        let (gw_conn_tx, gw_conn_rx) = channel::<SocketStream>();

        let gw_thread = thread::spawn(move || {
            let reactor = GatewayReactor {
                listener,
                poll,
                events: Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS),
                gw_cmd_rx,
                gw_conn_tx,
                pending_conns: VecDeque::new(),
            };

            reactor.run(listener_token, waker_token).expect("gateway failed");
        });

        Ok(GatewayHandle { gw_cmd_tx, gw_conn_rx, waker, gw_thread })
    }

    pub fn run(mut self, listener_token: Token, waker_token: Token) -> std::io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, None)?;

            for event in self.events.iter() {
                match event.token() {
                    listener_token => {
                        loop {
                            match self.listener.accept() {
                                Ok(stream) => {
                                    self.pending_conns.push_back(stream);
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                                Err(e) => return Err(e),
                            }
                        }
                    }
                    waker_token => {
                        while let Ok(cmd) = self.gw_cmd_rx.try_recv() {
                            match cmd {
                                GatewayCommand::AcceptConn => {
                                    if let Some(conn) = self.pending_conns.pop_front() {
                                        let _ = self.gw_conn_tx.send(conn);
                                    } else {
                                        log::error!("Gateway thread instructed to accept connection, but none found");
                                        // optionally queue assignment
                                        // TODO: what happens if we are instructed to accept a
                                        // conneciton but none is there?
                                    }
                                }
                                GatewayCommand::Shutdown => return Ok(()),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    collections::VecDeque,
    sync::{
        Arc,
        Mutex,
    },
};
use ::syscomm::{
    BlockingSocketStream,
    SocketStream,
};

//==================================================================================================
// Structures
//==================================================================================================

/// State associated with a user VM connected to this linuxd instance.
#[derive(Clone)]
pub struct UserVmHandle {
    // FIXME: this field is relevant once we start managing more than one user VMs.
    #[allow(dead_code)]
    conn_id: usize,
    // We keep track of a buffer to handle partial reads from the user VM socket. This buffer will
    // never exceed the IPC message size.
    user_vm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>>,
    gw_stream: Option<Arc<Mutex<BlockingSocketStream>>>,
}

impl UserVmHandle {
    pub fn new(
        conn_id: usize,
        user_vm_stream: SocketStream,
        gw_stream: Option<SocketStream>,
    ) -> Self {
        let blocking_stream: Option<BlockingSocketStream> = if let Some(gw_stream) = gw_stream {
            match gw_stream.set_blocking() {
                Ok(stream) => Some(stream),
                // We don't panic if we can not set the stream to blocking.
                Err(e) => {
                    error!("error setting gateway stream as blocking (error={e:?})");
                    None
                },
            }
        } else {
            None
        };

        Self {
            conn_id,
            user_vm_stream: Arc::new(Mutex::new((user_vm_stream, VecDeque::new()))),
            gw_stream: blocking_stream.map(|stream| Arc::new(Mutex::new(stream))),
        }
    }

    pub fn get_conn_id(&self) -> usize {
        self.conn_id
    }

    pub fn get_user_vm_stream(&self) -> Arc<Mutex<(SocketStream, VecDeque<u8>)>> {
        self.user_vm_stream.clone()
    }

    pub fn get_gw_vm_stream(&self) -> Option<Arc<Mutex<BlockingSocketStream>>> {
        self.gw_stream.clone()
    }
}

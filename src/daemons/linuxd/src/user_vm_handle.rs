// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::sync::{
    Arc,
    Mutex,
};
use ::syscomm::SocketStream;
use anyhow::Result;

//==================================================================================================
// Structures
//==================================================================================================

/// State associated with a user VM connected to this linuxd instance.
#[derive(Clone)]
pub struct UserVmHandle {
    conn_id: usize,
    user_vm_stream: Arc<Mutex<SocketStream>>,
    gw_stream: Option<Arc<Mutex<SocketStream>>>,
}

impl UserVmHandle {
    pub fn new(
        conn_id: usize,
        user_vm_stream: SocketStream,
        gw_stream: Option<SocketStream>,
    ) -> Result<Self> {
        Ok(Self {
            conn_id,
            user_vm_stream: Arc::new(Mutex::new(user_vm_stream)),
            gw_stream: gw_stream.map(|stream| Arc::new(Mutex::new(stream))),
        })
    }

    pub fn get_conn_id(&self) -> usize {
        self.conn_id
    }

    pub fn get_user_vm_stream(&self) -> Arc<Mutex<SocketStream>> {
        self.user_vm_stream.clone()
    }

    pub fn get_gw_vm_stream(&self) -> Option<Arc<Mutex<SocketStream>>> {
        self.gw_stream.clone()
    }
}

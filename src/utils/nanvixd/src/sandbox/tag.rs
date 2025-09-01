// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::rand::Rng;
use ::std::{
    self,
    hash::Hash,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxTag {
    tenant_id: String,
    app_name: String,
    sandbox_id: UserVmIdentifier,
}

impl SandboxTag {
    pub fn new(tenant_id: &str, app_name: &str) -> Self {
        let mut rng: rand::rngs::ThreadRng = rand::rng();
        let sandbox_id: u32 = rng.random();

        Self {
            tenant_id: tenant_id.to_string(),
            app_name: app_name.to_string(),
            sandbox_id,
        }
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn sandbox_id(&self) -> u32 {
        self.sandbox_id
    }
}

impl Hash for SandboxTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tenant_id.hash(state);
        self.app_name.hash(state);
        self.sandbox_id.hash(state);
    }
}

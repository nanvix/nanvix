// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    self,
    hash::Hash,
};
use ::uuid::Uuid;

//==================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxTag {
    tenant_id: String,
    app_name: String,
    sandbox_id: String,
}

impl SandboxTag {
    pub fn new(tenant_id: &str, app_name: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            app_name: app_name.to_string(),
            sandbox_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }
}

impl Hash for SandboxTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tenant_id.hash(state);
        self.app_name.hash(state);
        self.sandbox_id.hash(state);
    }
}

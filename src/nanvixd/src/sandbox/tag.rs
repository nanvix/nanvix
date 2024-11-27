// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    self,
    hash::Hash,
};

//==================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxTag {
    clientid: usize,
    program: String,
}

impl SandboxTag {
    pub fn new(clientid: usize, program: &str) -> Self {
        Self {
            clientid,
            program: program.to_string(),
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }
}

impl Hash for SandboxTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.clientid.hash(state);
        self.program.hash(state);
    }
}

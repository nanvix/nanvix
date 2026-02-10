// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox tagging for identification and management.
//!
//! This module provides structures and functionality to uniquely identify and tag sandboxes
//! within the Nanvix Daemon. Tags include tenant information, program details, and a unique
//! sandbox identifier for tracking and managing individual VM instances.

//==================================================================================================
// Imports
//==================================================================================================

use ::rand::RngExt;
use ::std::hash::Hash;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A unique tag that identifies a sandbox instance.
///
/// This structure combines tenant, application, program, and argument information to create
/// a unique identifier for each sandbox. Each tag includes a randomly generated sandbox ID
/// to ensure uniqueness even for identical configurations.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxTag {
    /// Path to the program binary being executed.
    program: String,
    /// Optional command-line arguments for the program.
    program_args: Option<String>,
    /// Tenant identifier for resource isolation and multi-tenancy support.
    tenant_id: String,
    /// Application name for identification and organization.
    app_name: String,
    /// Unique sandbox identifier assigned at creation time.
    sandbox_id: UserVmIdentifier,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxTag {
    ///
    /// # Description
    ///
    /// Creates a new sandbox tag with a randomly generated identifier.
    ///
    /// # Parameters
    ///
    /// - `tenant_id`: Tenant identifier for resource isolation.
    /// - `program`: Path to the program binary to execute.
    /// - `app_name`: Application name for identification.
    /// - `program_args`: Optional command-line arguments for the program.
    ///
    /// # Returns
    ///
    /// A new sandbox tag with a unique randomly generated sandbox ID.
    ///
    pub fn new(
        tenant_id: &str,
        program: &str,
        app_name: &str,
        program_args: Option<String>,
    ) -> Self {
        let mut rng: rand::rngs::ThreadRng = rand::rng();
        let sandbox_id: UserVmIdentifier = UserVmIdentifier::new(rng.random());

        Self {
            program: program.to_string(),
            tenant_id: tenant_id.to_string(),
            app_name: app_name.to_string(),
            sandbox_id,
            program_args,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the program binary path.
    ///
    /// # Returns
    ///
    /// A string slice containing the program path.
    ///
    pub fn program(&self) -> &str {
        &self.program
    }

    ///
    /// # Description
    ///
    /// Returns the optional program arguments.
    ///
    /// # Returns
    ///
    /// An optional reference to the program arguments string.
    ///
    pub fn program_args(&self) -> Option<&String> {
        self.program_args.as_ref()
    }

    ///
    /// # Description
    ///
    /// Returns the tenant identifier.
    ///
    /// # Returns
    ///
    /// A string slice containing the tenant ID.
    ///
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    ///
    /// # Description
    ///
    /// Returns the application name.
    ///
    /// # Returns
    ///
    /// A string slice containing the application name.
    ///
    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    ///
    /// # Description
    ///
    /// Returns the unique sandbox identifier.
    ///
    /// # Returns
    ///
    /// The User VM identifier assigned to this sandbox.
    ///
    pub fn sandbox_id(&self) -> UserVmIdentifier {
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

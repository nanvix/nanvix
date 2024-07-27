// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        GroupIdentifier,
        UserIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents the identity of a process.
///
#[derive(Clone, Copy)]
pub struct ProcessIdentity {
    /// Real user identifier.
    uid: UserIdentifier,
    /// Effective user identifier.
    euid: UserIdentifier,
    /// Saved user identifier.
    suid: UserIdentifier,
    /// Real group identifier.
    gid: GroupIdentifier,
    /// Effective group identifier.
    egid: GroupIdentifier,
    /// Saved group identifier.
    sgid: GroupIdentifier,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessIdentity {
    ///
    /// # Description
    ///
    /// Instantiates a new process identity.
    ///
    /// # Parameters
    ///
    /// - `uid`: Real user identifier.
    /// - `gid`: Real group identifier.
    ///
    /// # Returns
    ///
    /// A new process identity.
    ///
    pub fn new(uid: UserIdentifier, gid: GroupIdentifier) -> Self {
        Self {
            uid,
            euid: uid,
            suid: uid,
            gid,
            egid: gid,
            sgid: gid,
        }
    }

    ///
    /// # Description
    ///
    /// Gets the user identifier in the target process identity.
    ///
    /// # Return Values
    ///
    /// The user identifier in the target process identity.
    ///
    pub fn get_uid(&self) -> UserIdentifier {
        self.uid
    }

    ///
    /// # Description
    ///
    /// Sets the user identifier in the target process identity.
    ///
    /// # Parameters
    ///
    /// - `uid`: The new user identifier.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, empty result is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn set_uid(&mut self, uid: UserIdentifier) -> Result<(), Error> {
        if self.is_root() {
            // Root user is changing its identity.
            self.uid = uid;
            self.euid = uid;
            self.suid = uid;
            Ok(())
        } else if self.uid == uid || self.suid == uid {
            // User is changing its own identity.
            self.euid = uid;
            Ok(())
        } else {
            let reason: &str = "user is change another user's identity";
            error!("set_uid(): {}", reason);
            Err(Error::new(ErrorCode::PermissionDenied, reason))
        }
    }

    ///
    ///
    /// # Description
    ///
    /// Gets the effective user identifier in the target process identity.
    ///
    /// # Return Values
    ///
    /// The effective user identifier in the target process identity.
    ///
    pub fn get_euid(&self) -> UserIdentifier {
        self.euid
    }

    ///
    /// # Description
    ///
    /// Sets the effective user identifier in the target process identity.
    ///
    /// # Parameters
    ///
    /// - `euid`: The new effective user identifier.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, empty result is returned. Upon failure, an error is returned
    /// instead.
    pub fn set_euid(&mut self, euid: UserIdentifier) -> Result<(), Error> {
        if self.is_root() {
            // Root user is changing its identity.
            self.euid = euid;
            Ok(())
        } else if self.uid == euid || self.suid == euid {
            // User is changing its own identity.
            self.euid = euid;
            Ok(())
        } else {
            let reason: &str = "user is changing another user's identity";
            error!("set_euid(): {}", reason);
            Err(Error::new(ErrorCode::PermissionDenied, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Gets the group identifier in the target process identity.
    ///
    /// # Return Values
    ///
    /// The group identifier in the target process identity.
    ///
    pub fn get_gid(&self) -> GroupIdentifier {
        self.gid
    }

    ///
    /// # Description
    ///
    /// Sets the group identifier in the target process identity.
    ///
    /// # Parameters
    ///
    /// - `gid`: The new group identifier.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, empty result is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn set_gid(&mut self, gid: GroupIdentifier) -> Result<(), Error> {
        if self.is_root() {
            // Root user is changing its identity.
            self.gid = gid;
            self.egid = gid;
            self.sgid = gid;
            Ok(())
        } else if self.gid == gid || self.sgid == gid {
            // User is changing its own identity.
            self.egid = gid;
            Ok(())
        } else {
            let reason: &str = "user is changing another user's identity";
            error!("set_gid(): {}", reason);
            Err(Error::new(ErrorCode::PermissionDenied, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Gets the effective group identifier in the target process identity.
    ///
    /// # Return Values
    ///
    /// The effective group identifier in the target process identity.
    ///
    pub fn get_egid(&self) -> GroupIdentifier {
        self.egid
    }

    ///
    /// # Description
    ///
    /// Sets the effective group identifier in the target process identity.
    ///
    /// # Parameters
    ///
    /// - `egid`: The new effective group identifier.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, empty result is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn set_egid(&mut self, egid: GroupIdentifier) -> Result<(), Error> {
        if self.is_root() {
            // Root user is changing its identity.
            self.egid = egid;
            Ok(())
        } else if self.gid == egid || self.sgid == egid {
            // User is changing its own identity.
            self.egid = egid;
            Ok(())
        } else {
            let reason: &str = "user is changing another user's identity";
            error!("set_egid(): {}", reason);
            Err(Error::new(ErrorCode::PermissionDenied, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Checks wether the target identity is the root identity.
    ///
    /// # Return Values
    ///
    /// If the target identity is the root identity, `true` is returned.  Otherwise, `false` is
    /// returned instead.
    ///
    pub fn is_root(&self) -> bool {
        self.uid == UserIdentifier::ROOT || self.suid == UserIdentifier::ROOT
    }
}

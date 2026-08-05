// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod getegid;
mod geteuid;
mod getgid;
mod getuid;
mod job_control;
mod kill;
mod lookup;
mod rpc;
mod signup;
mod wait;

//==================================================================================================
// Exports
//==================================================================================================

pub use getegid::getegid;
pub use geteuid::geteuid;
pub use getgid::getgid;
pub use getuid::getuid;
pub use job_control::{
    getpgid,
    getpgrp,
    getsid,
    setpgid,
    setsid,
    tcgetpgrp,
    tcsetpgrp,
};
pub use kill::kill;
pub use lookup::lookup;
pub use signup::signup;
pub use wait::{
    wait,
    WaitOutcome,
};

// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_types::pthread_attr_t;
use ::syscall::pthread::{
    pthread_attr_destroy,
    pthread_attr_init,
    pthread_getattr_np,
    pthread_self,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Exercises thread attribute initialization, querying, and destruction.
pub fn run() -> Result<(), Error> {
    test_attr_init_destroy()?;
    test_getattr_np()?;
    test_attr_getstack()?;
    Ok(())
}

//==================================================================================================
// attr_init / attr_destroy (ports attr_init_destroy.c)
//==================================================================================================

fn test_attr_init_destroy() -> Result<(), Error> {
    // SAFETY: zeroed memory is a valid uninitialized pthread_attr_t (is_initialized == 0).
    let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };

    pthread_attr_init(&mut attr)?;
    pthread_attr_destroy(&mut attr)?;

    Ok(())
}

//==================================================================================================
// pthread_getattr_np (ports getattr.c)
//==================================================================================================

fn test_getattr_np() -> Result<(), Error> {
    let self_id = pthread_self();

    // SAFETY: zeroed memory is a valid uninitialized pthread_attr_t.
    let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };

    pthread_getattr_np(self_id, &mut attr)?;

    // Verify that the returned attributes contain sensible values.
    // Access fields directly since pthread_attr_getstack is not re-exported from syscall::pthread.
    assert!(attr.stacksize > 0, "stack size must be positive");

    // TODO: test detachstate once pthread_attr_getdetachstate is available in the Rust API.

    pthread_attr_destroy(&mut attr)?;

    Ok(())
}

//==================================================================================================
// pthread_attr_getstack (ports getstack.c)
//==================================================================================================

fn test_attr_getstack() -> Result<(), Error> {
    // SAFETY: zeroed memory is a valid uninitialized pthread_attr_t.
    let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };

    pthread_attr_init(&mut attr)?;

    // Access fields directly since pthread_attr_getstack is not re-exported from syscall::pthread.
    assert!(attr.stacksize > 0, "stack size must be positive");
    assert!(!attr.stackaddr.is_null(), "stack address must not be null");

    pthread_attr_destroy(&mut attr)?;

    Ok(())
}

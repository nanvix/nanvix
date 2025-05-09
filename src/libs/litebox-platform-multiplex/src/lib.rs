//! This is a copy of the LiteBox platform multiplexer, specialized to work on `litebox-nanvix`.
//!
//! In the future, once platforms are more stabilized, this code might be removed, and a new feature
//! would be added to `litebox_platform_multiplexer`. For now, this exists purely to override the
//! existing implementation.

#![no_std]

extern crate alloc;

pub type Platform = litebox_nanvix::NanvixUserland;

static PLATFORM: once_cell::race::OnceBox<&'static Platform> = once_cell::race::OnceBox::new();

/// Initialize the shim by providing a [LiteBox platform](../litebox/platform/index.html).
///
/// **Must** be invoked prior to any of the other functionality provided by this crate; all other
/// functionality is prone to panics if this has not been invoked first.
///
/// # Panics
///
/// Panics if invoked more than once
#[expect(
    clippy::match_wild_err_arm,
    reason = "the platform itself is not Debug thus we cannot use `expect`"
)]
pub fn set_platform(platform: &'static Platform) {
    match PLATFORM.set(alloc::boxed::Box::new(platform)) {
        Ok(()) => {},
        Err(_) => panic!("set_platform should only be called once per crate"),
    }
}

/// Get the global platform, or panic if [`set_platform`] has not yet been invoked.
///
/// # Panics
///
/// Panics if [`set_platform`] has not been invoked before this
pub fn platform() -> &'static Platform {
    PLATFORM
        .get()
        .expect("set_platform should have already been called before this point")
}

//! # nanvix-oci
//!
//! OCI annotation constants and parsing utilities for Nanvix container images.
//!
//! This crate defines the `com.nanvix.*` annotation namespace used to identify
//! and configure Nanvix workloads in standard OCI images.

pub mod annotations;
pub mod config;

pub use config::NanvixImageConfig;

//! Core Remnant modules.
//!
//! The binary entrypoint in `main.rs` stays intentionally thin. Domain logic
//! lives under modules declared here so it can grow without turning the CLI
//! entrypoint into the application root.

pub mod archive;
pub mod commands;
pub mod output;
pub mod package_json;
pub mod policy;

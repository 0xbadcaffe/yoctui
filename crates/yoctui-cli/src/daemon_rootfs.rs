//! Bounded, connection-owned read-only image metadata. Never broadcast sources.
use std::time::Duration;

mod authority_validation;
mod client_query;
mod source_acquisition;
mod worker;

pub use authority_validation::validate_authority;
pub use client_query::{query_for_app, request_sources};
pub use worker::PendingQuery;

#[cfg(test)]
use client_query::request_sources_at;
#[cfg(test)]
use source_acquisition::acquire;

pub const QUERY_TIMEOUT: Duration = Duration::from_secs(120);

#[cfg(test)]
#[path = "tests/daemon_rootfs/mod.rs"]
mod tests;

#![cfg(unix)]

#[path = "daemon_persist/history_and_job_identity.rs"]
mod history_and_job_identity;
#[path = "daemon_persist/recovery_boundaries.rs"]
mod recovery_boundaries;
#[path = "daemon_persist/support.rs"]
mod support;

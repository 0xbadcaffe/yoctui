//! Workspace input.

include!("workspace_input/global_and_logs.rs");

include!("workspace_input/metadata_and_images.rs");

include!("workspace_input/sdk_and_testing.rs");

include!("workspace_input/security_and_qa.rs");

#[cfg(test)]
#[path = "tests/workspace_input/mod.rs"]
mod focus_flow_tests;

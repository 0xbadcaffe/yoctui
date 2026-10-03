//! Bridge backend.

include!("bridge_backend/types_and_stderr.rs");

#[cfg(test)]
#[path = "tests/bridge_local_api_scope.rs"]
mod local_api_scope_tests;

include!("bridge_backend/process_lifecycle.rs");

include!("bridge_backend/event_mapping.rs");

include!("bridge_backend/dependency_graph.rs");

include!("bridge_backend/backend_api.rs");

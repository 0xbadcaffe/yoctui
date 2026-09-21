include!("test_results/adapter_and_capability.rs");

include!("test_results/commands_and_identity.rs");

include!("test_results/job_runner.rs");

include!("test_results/result_discovery_and_parsing.rs");

include!("test_results/case_and_path_validation.rs");

#[cfg(test)]
#[path = "tests/test_results/mod.rs"]
mod tests;

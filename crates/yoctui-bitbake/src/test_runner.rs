include!("test_runner/adapter_and_capability.rs");

include!("test_runner/command_and_discovery.rs");

include!("test_runner/job_runner.rs");

#[cfg(test)]
#[path = "tests/test_runner/mod.rs"]
mod tests;

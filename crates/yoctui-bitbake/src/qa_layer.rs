include!("qa_layer/capability_and_validation.rs");

include!("qa_layer/command_spec.rs");

include!("qa_layer/job_runner.rs");

#[cfg(test)]
#[path = "tests/qa_layer/mod.rs"]
mod tests;

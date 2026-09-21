include!("compatibility_probe/context_and_runner.rs");

include!("compatibility_probe/backend_probes.rs");

include!("compatibility_probe/observation_validation.rs");

#[cfg(test)]
#[path = "tests/compatibility_probe/mod.rs"]
mod tests;

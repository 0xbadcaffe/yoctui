include!("security_report/types_and_adapter.rs");

include!("security_report/report_acquisition.rs");

include!("security_report/manifest_and_cyclonedx.rs");

include!("security_report/cve_parsing.rs");

include!("security_report/spdx_and_metadata.rs");

#[cfg(test)]
#[path = "tests/security_report/mod.rs"]
mod tests;

include!("compatibility_fixtures/types_and_resolution.rs");

include!("compatibility_fixtures/release_catalog.rs");

include!("compatibility_fixtures/fixture_helpers.rs");

#[cfg(test)]
#[path = "tests/compatibility_fixtures/mod.rs"]
mod tests;

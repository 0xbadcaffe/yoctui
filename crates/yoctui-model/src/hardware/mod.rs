use super::*;

mod project_controls;
mod projects;
mod reducer;
mod types;
mod viewer;

#[cfg(test)]
#[path = "../tests/hardware_text.rs"]
mod text_tests;

pub use projects::*;
pub(crate) use reducer::reduce_hardware;
pub use types::*;

#[cfg(test)]
#[path = "../tests/hardware_domain.rs"]
mod tests;

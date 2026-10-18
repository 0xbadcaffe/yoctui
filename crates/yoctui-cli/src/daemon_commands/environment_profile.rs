//! Exact selected source/build profiles and legacy ancestor discovery.
use std::path::Path;
use yoctui_model::BuildEnvironmentProfile;

pub(crate) fn inferred_build_environment_profile(
    working_directory: &Path,
) -> Option<BuildEnvironmentProfile> {
    yoctui_bitbake::discover_build_environment(working_directory, None)
}

pub(crate) fn daemon_build_environment_profile(build: &Path) -> Option<BuildEnvironmentProfile> {
    if let Some(source) = std::env::var_os("YOCTUI_SOURCE_DIR") {
        return selected_build_environment_profile(build, Path::new(&source));
    }
    inferred_build_environment_profile(build)
}

pub(crate) fn selected_build_environment_profile(
    working_directory: &Path,
    source_directory: &Path,
) -> Option<BuildEnvironmentProfile> {
    yoctui_bitbake::discover_build_environment(working_directory, Some(source_directory))
}

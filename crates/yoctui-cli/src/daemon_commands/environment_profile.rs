//! Exact selected source/build profiles and legacy ancestor discovery.
use std::path::Path;
use yoctui_model::BuildEnvironmentProfile;

pub(crate) fn inferred_build_environment_profile(
    working_directory: &Path,
) -> Option<BuildEnvironmentProfile> {
    let build_dir = working_directory.canonicalize().ok()?;
    if !build_dir.join("conf/local.conf").is_file()
        || !build_dir.join("conf/bblayers.conf").is_file()
    {
        return None;
    }
    let source_dir = build_dir
        .ancestors()
        .find(|candidate| candidate.join("oe-init-build-env").is_file())?;
    selected_build_environment_profile(&build_dir, source_dir)
}

pub(crate) fn selected_build_environment_profile(
    working_directory: &Path,
    source_directory: &Path,
) -> Option<BuildEnvironmentProfile> {
    let build_dir = working_directory.canonicalize().ok()?;
    if !build_dir.join("conf/local.conf").is_file()
        || !build_dir.join("conf/bblayers.conf").is_file()
    {
        return None;
    }
    let source_dir = source_directory.canonicalize().ok()?;
    let init_script = source_dir.join("oe-init-build-env").canonicalize().ok()?;
    Some(BuildEnvironmentProfile {
        source_dir,
        build_dir,
        init_script,
    })
}

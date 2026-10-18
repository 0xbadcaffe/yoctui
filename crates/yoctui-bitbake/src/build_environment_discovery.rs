use std::path::Path;
use yoctui_model::BuildEnvironmentProfile;

/// Import an existing build, or the parent setup created by bitbake-setup.
/// Only inspect paths here; initialization is a separate, reviewed operation.
pub fn discover_build_environment(
    selected: &Path,
    source: Option<&Path>,
) -> Option<BuildEnvironmentProfile> {
    let selected = selected.canonicalize().ok()?;
    let build = if selected.join("build/init-build-env").is_file() {
        selected.join("build").canonicalize().ok()?
    } else {
        selected
    };
    if !build.join("conf/local.conf").is_file() || !build.join("conf/bblayers.conf").is_file() {
        return None;
    }
    let wrapper = build.join("init-build-env");
    if wrapper.is_file() {
        let source = source
            .and_then(|p| p.canonicalize().ok())
            .filter(|p| p.is_dir())
            .or_else(|| {
                build
                    .parent()?
                    .join("layers/openembedded-core")
                    .canonicalize()
                    .ok()
            })
            .unwrap_or_else(|| build.clone());
        return Some(BuildEnvironmentProfile {
            source_dir: source,
            init_script: wrapper.canonicalize().ok()?,
            build_dir: build,
        });
    }
    let source = match source {
        Some(source) => source.canonicalize().ok()?,
        None => build
            .ancestors()
            .find(|path| path.join("oe-init-build-env").is_file())?
            .to_path_buf(),
    };
    Some(BuildEnvironmentProfile {
        init_script: source.join("oe-init-build-env").canonicalize().ok()?,
        source_dir: source,
        build_dir: build,
    })
}

#[cfg(test)]
#[path = "tests/build_environment_discovery.rs"]
mod tests;

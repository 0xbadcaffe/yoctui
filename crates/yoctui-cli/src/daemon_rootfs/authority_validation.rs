use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use yoctui_model::DaemonCompatibilitySnapshot;
use yoctui_protocol::{daemon::DaemonInstanceId, rootfs::RootfsSourcesRequestData};

pub fn validate_authority(
    query: &RootfsSourcesRequestData,
    instance: DaemonInstanceId,
    compatibility: &DaemonCompatibilitySnapshot,
) -> Result<PathBuf> {
    query.validate()?;
    ensure!(
        query.daemon_instance_id == instance,
        "rootfs query belongs to another daemon instance"
    );
    ensure!(
        query.compatibility_generation == compatibility.snapshot.generation,
        "rootfs query compatibility generation is stale"
    );
    let environment = &compatibility.snapshot.environment;
    let build = environment
        .build_directory
        .value()
        .context("rootfs query has no build authority")?;
    ensure!(
        environment.machine.value() == Some(&query.request.image.machine),
        "rootfs query machine does not match daemon authority"
    );
    let artifact = Path::new(&query.request.image.path);
    let metadata =
        std::fs::symlink_metadata(artifact).context("selected rootfs artifact is unavailable")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "selected rootfs artifact must be a regular file"
    );
    let canonical_build = build.canonicalize()?;
    ensure!(
        artifact.canonicalize()?.starts_with(&canonical_build),
        "selected rootfs artifact escapes daemon build directory"
    );
    Ok(build.clone())
}

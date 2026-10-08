// Read only an exact image's build-generated testdata sidecar. Never infer
// work-directory paths from recipe names or use another image's metadata.
pub fn rootfs_sources_from_deployed_metadata(
    build_directory: &Path,
    request: &RootfsCompositionRequest,
    manifest: &Path,
) -> Option<RootfsCompositionSources> {
    use std::io::Read;
    #[derive(serde::Deserialize)]
    struct Metadata {
        #[serde(rename = "PN")]
        recipe: String,
        #[serde(rename = "MACHINE")]
        machine: String,
        #[serde(rename = "IMAGE_NAME")]
        image_name: String,
        #[serde(rename = "IMAGE_MANIFEST")]
        manifest: PathBuf,
        #[serde(rename = "IMAGE_ROOTFS")]
        rootfs: PathBuf,
        #[serde(rename = "PKGDATA_DIR")]
        pkgdata: PathBuf,
    }
    request.validate().ok()?;
    let build = canonical_directory(build_directory, None).ok()?;
    let manifest = canonical_regular_file(manifest, &build).ok()?;
    let artifact = canonical_regular_file(&request.image.path, &build).ok()?;
    if artifact.parent() != manifest.parent() {
        return None;
    }
    let sidecar = manifest.with_extension("testdata.json");
    let sidecar = canonical_regular_file(&sidecar, &build).ok()?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(sidecar).ok()?;
    if !file.metadata().ok()?.is_file() || file.metadata().ok()?.len() > MAX_MANIFEST_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES { return None; }
    let data: Metadata = serde_json::from_slice(&bytes).ok()?;
    let manifest_stem = manifest.file_stem()?.to_str()?;
    if data.recipe != request.image.image || data.machine != request.image.machine
        || !valid_text(&data.image_name) || data.image_name.is_empty()
        || (manifest_stem != data.image_name && manifest_stem != format!("{}.rootfs", data.image_name))
        || !artifact.file_name()?.to_str()?.starts_with(&format!("{}.", data.image_name))
        || data.manifest.file_name() != manifest.file_name()
    { return None; }
    let resolve = |path: &Path| -> Option<PathBuf> {
        if path.components().any(|part| matches!(part,
            std::path::Component::ParentDir | std::path::Component::CurDir | std::path::Component::Prefix(_)))
        { return None; }
        let resolved = if path.is_absolute() { path.to_path_buf() } else { build.join(path) };
        (resolved != build && resolved.starts_with(&build)).then_some(resolved)
    };
    resolve(&data.manifest)?;
    let image_rootfs = canonical_directory(&resolve(&data.rootfs)?, Some(&build)).ok()?;
    let pkgdata_directory = canonical_directory(&resolve(&data.pkgdata)?, Some(&build)).ok()?;
    Some(RootfsCompositionSources {
        image: request.image.clone(), manifest: Some(manifest),
        pkgdata_directory: Some(pkgdata_directory), image_rootfs: Some(image_rootfs),
    })
}

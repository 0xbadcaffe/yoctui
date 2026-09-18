//! Project profile.
use super::*;

pub(crate) const MAX_PROJECT_PROFILE_BYTES: u64 = 1_048_576;

pub(crate) fn load_project_profile(root: &Path) -> Result<Option<yoctui_model::ProjectProfile>> {
    let root = root
        .canonicalize()
        .with_context(|| format!("could not resolve project root {}", root.display()))?;
    let directory = root.join(".yoctui");
    let path = directory.join("project.toml");
    if !path.exists() {
        return Ok(None);
    }
    let directory_metadata = fs::symlink_metadata(&directory)
        .with_context(|| format!("could not inspect {}", directory.display()))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        anyhow::bail!("project profile directory must be a regular directory");
    }
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("could not inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("project profile must be a regular non-symlink file");
    }
    if metadata.len() > MAX_PROJECT_PROFILE_BYTES {
        anyhow::bail!("project profile exceeds the 1 MiB limit");
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let profile: yoctui_model::ProjectProfile = toml::from_str(&text)
        .with_context(|| format!("invalid project profile {}", path.display()))?;
    profile.validate().map_err(anyhow::Error::msg)?;
    Ok(Some(profile))
}

pub fn generate_project_profile(
    root: &Path,
    profile: &yoctui_model::ProjectProfile,
    replace: bool,
) -> Result<()> {
    profile.validate().map_err(anyhow::Error::msg)?;
    let root = root
        .canonicalize()
        .with_context(|| format!("could not resolve project root {}", root.display()))?;
    let directory = root.join(".yoctui");
    if directory.exists() {
        let metadata = fs::symlink_metadata(&directory)
            .with_context(|| format!("could not inspect {}", directory.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("project profile directory must be a regular directory");
        }
    } else {
        fs::create_dir(&directory)
            .with_context(|| format!("could not create {}", directory.display()))?;
    }
    let destination = directory.join("project.toml");
    if destination.exists() {
        let metadata = fs::symlink_metadata(&destination)
            .with_context(|| format!("could not inspect {}", destination.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            anyhow::bail!("project profile destination must be a regular non-symlink file");
        }
        if !replace {
            anyhow::bail!("project profile already exists; replacement was not confirmed");
        }
    }
    let text = toml::to_string_pretty(profile).context("could not serialize project profile")?;
    if text.len() as u64 > MAX_PROJECT_PROFILE_BYTES {
        anyhow::bail!("generated project profile exceeds the 1 MiB limit");
    }
    let temporary = directory.join(format!("project.toml.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("could not create {}", temporary.display()))?;
    let result = (|| -> Result<()> {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if replace {
            fs::rename(&temporary, &destination)?;
        } else {
            fs::hard_link(&temporary, &destination)?;
            fs::remove_file(&temporary)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.with_context(|| format!("could not write {}", destination.display()))
}

pub(crate) fn project_profile_root(build_dir: &Path) -> Option<PathBuf> {
    env::var_os("OEROOT")
        .map(PathBuf::from)
        .or_else(|| build_dir.parent().map(Path::to_path_buf))
}

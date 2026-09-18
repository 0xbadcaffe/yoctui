//! Path validation.
use super::*;

pub(crate) fn revalidate_canonical_regular_file(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || fs::canonicalize(path).ok().as_deref() != Some(path)
    {
        return Err(format!("{label} is no longer a canonical regular file"));
    }
    Ok(())
}

pub(crate) fn revalidate_canonical_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || fs::canonicalize(path).ok().as_deref() != Some(path)
    {
        return Err(format!("{label} is no longer a canonical directory"));
    }
    Ok(())
}

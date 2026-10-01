//! Real project folders and private manifests; no daemon or shell authority.
use super::*;
use std::io::{Read, Write};
use yoctui_model::{
    HardwareProject, HardwareProjectEntry, HardwareProjectOperation, HardwareProjectRequest,
    HardwareProjectResult, MAX_HARDWARE_IMPORT_BYTES, MAX_HARDWARE_PROJECTS,
    validate_hardware_project_name,
};

const MANIFEST: &str = ".yoctui-project.toml";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024;

pub(super) fn run(request: HardwareProjectRequest) -> Action {
    let result = data_root().and_then(|root| execute(&root, request.operation));
    Action::Hardware(HardwareAction::Project(
        yoctui_model::HardwareProjectAction::Finished {
            generation: request.generation,
            result: result.map_err(|error| format!("{error:#}")),
        },
    ))
}

fn data_root() -> Result<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .context("No user data directory is configured")?;
    anyhow::ensure!(data.is_absolute(), "user data directory must be absolute");
    Ok(data.join("yoctui/hardware-projects"))
}

fn name_valid(name: &str) -> Result<()> {
    validate_hardware_project_name(name).map_err(anyhow::Error::msg)
}

pub(super) fn regular_file(path: &Path) -> Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "file must be regular and non-symlink"
    );
    Ok(file)
}

fn directory(path: &Path) -> Result<PathBuf> {
    anyhow::ensure!(path.is_absolute(), "directory must be absolute");
    // Refuse symlink components, not just the final entry, including project roots.
    let mut checked = PathBuf::new();
    for component in path.components() {
        anyhow::ensure!(
            !matches!(component, std::path::Component::ParentDir),
            "parent traversal is not allowed"
        );
        checked.push(component);
        let metadata = fs::symlink_metadata(&checked)?;
        anyhow::ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "directory must not contain symlinks: {}",
            checked.display()
        );
    }
    fs::canonicalize(path).context("could not resolve project directory")
}

pub(super) fn validate_preview(root: &Path, path: &Path) -> Result<()> {
    let store = directory(&data_root()?)?;
    validate_preview_in_store(&store, root, path)
}

fn validate_preview_in_store(store: &Path, root: &Path, path: &Path) -> Result<()> {
    let root = directory(root)?;
    anyhow::ensure!(
        root.parent() == Some(store),
        "preview root is not a registered Hardware project"
    );
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid project root")?;
    read_project(store, name)?;
    let parent = directory(path.parent().context("preview path has no directory")?)?;
    anyhow::ensure!(
        parent.starts_with(&root),
        "preview file escapes project root"
    );
    anyhow::ensure!(
        HardwareDocumentKind::project_kind(path).is_some(),
        "file is stored only; preview format is not supported"
    );
    regular_file(path)?;
    Ok(())
}

fn ensure_store(root: &Path) -> Result<PathBuf> {
    anyhow::ensure!(root.is_absolute(), "project store must be absolute");
    let mut current = PathBuf::new();
    for component in root.components() {
        anyhow::ensure!(
            !matches!(component, std::path::Component::ParentDir),
            "project store cannot contain parent traversal"
        );
        current.push(component);
        if !current.exists() {
            private_directory(&current)?;
        }
        directory(&current)?;
    }
    directory(root)
}

fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).with_context(|| {
        format!(
            "could not create folder {} (existing names are not overwritten)",
            path.display()
        )
    })
}

fn read_project(root: &Path, name: &str) -> Result<HardwareProject> {
    name_valid(name)?;
    let project_root = directory(&root.join(name))?;
    let file = regular_file(&project_root.join(MANIFEST))?;
    anyhow::ensure!(
        file.metadata()?.len() <= MAX_MANIFEST_BYTES,
        "project manifest exceeds 16 KiB"
    );
    let mut text = String::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_string(&mut text)?;
    anyhow::ensure!(
        text.len() as u64 <= MAX_MANIFEST_BYTES,
        "project manifest grew past 16 KiB"
    );
    let mut project: HardwareProject = toml::from_str(&text).context("invalid project manifest")?;
    project.validate().map_err(anyhow::Error::msg)?;
    anyhow::ensure!(project.name == name, "project name differs from its folder");
    // The folder being opened, never a stored path, determines filesystem authority.
    project.root = project_root;
    Ok(project)
}

fn write_manifest(project: &HardwareProject) -> Result<()> {
    project.validate().map_err(anyhow::Error::msg)?;
    let text = toml::to_string(project)?;
    let target = project.root.join(MANIFEST);
    if target.exists() {
        regular_file(&target)?;
    }
    let temporary = project.root.join(format!(
        ".yoctui-save-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, &target)?;
        fs::File::open(&project.root)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn contained(project: &HardwareProject, relative: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        relative
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_))),
        "project directory must be a contained relative path"
    );
    let target = directory(&project.root.join(relative))?;
    anyhow::ensure!(
        target.starts_with(&project.root),
        "directory escapes project root"
    );
    Ok(target)
}

fn entries(path: &Path, project: bool) -> Result<Vec<HardwareProjectEntry>> {
    let path = directory(path)?;
    let mut entries = Vec::new();
    for child in fs::read_dir(path)? {
        let child = child?;
        let metadata = fs::symlink_metadata(child.path())?;
        let name = child.file_name().to_string_lossy().into_owned();
        if metadata.file_type().is_symlink()
            || !(metadata.is_dir() || metadata.is_file())
            || (project && name.starts_with(".yoctui-"))
        {
            continue;
        }
        anyhow::ensure!(
            entries.len() < MAX_HARDWARE_BROWSER_ENTRIES,
            "directory exceeds the {}-entry browsing limit",
            MAX_HARDWARE_BROWSER_ENTRIES
        );
        entries.push(HardwareProjectEntry {
            name,
            path: child.path(),
            is_directory: metadata.is_dir(),
            size: metadata.len(),
            kind: HardwareDocumentKind::project_kind(&child.path()),
        });
    }
    entries.sort_by_key(|entry| (!entry.is_directory, entry.name.to_lowercase()));
    Ok(entries)
}

fn directory_result(project: HardwareProject, relative: PathBuf) -> Result<HardwareProjectResult> {
    let entries = entries(&contained(&project, &relative)?, true)?;
    Ok(HardwareProjectResult::Directory {
        project,
        relative,
        entries,
    })
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    directory(source.parent().context("source has no parent directory")?)?;
    let mut source = regular_file(source)?;
    anyhow::ensure!(
        source.metadata()?.len() <= MAX_HARDWARE_IMPORT_BYTES,
        "file exceeds 256 MiB import limit"
    );
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let temporary = destination
        .parent()
        .context("destination has no parent")?
        .join(format!(
            ".yoctui-import-{}-{}",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
    let mut output = options
        .open(&temporary)
        .context("cannot create private import staging file")?;
    let result = (|| -> Result<()> {
        let bytes = std::io::copy(
            &mut Read::by_ref(&mut source).take(MAX_HARDWARE_IMPORT_BYTES + 1),
            &mut output,
        )?;
        anyhow::ensure!(
            bytes <= MAX_HARDWARE_IMPORT_BYTES,
            "source grew beyond import limit"
        );
        output.sync_all()?;
        fs::hard_link(&temporary, destination)
            .context("destination already exists or cannot be created; no overwrite performed")?;
        Ok(())
    })();
    drop(output);
    let _ = fs::remove_file(&temporary);
    result
}

fn execute(root: &Path, operation: HardwareProjectOperation) -> Result<HardwareProjectResult> {
    use HardwareProjectOperation as Op;
    if let Op::ImportBrowse { directory: path } = operation {
        let path = directory(&path)?;
        return Ok(HardwareProjectResult::ImportBrowser {
            entries: entries(&path, false)?,
            directory: path,
        });
    }
    let root = ensure_store(root)?;
    match operation {
        Op::List => {
            let mut projects = Vec::new();
            for child in fs::read_dir(&root)? {
                let child = child?;
                let metadata = fs::symlink_metadata(child.path())?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    continue;
                }
                anyhow::ensure!(
                    projects.len() < MAX_HARDWARE_PROJECTS,
                    "project catalog exceeds {} projects",
                    MAX_HARDWARE_PROJECTS
                );
                let name = child.file_name().to_string_lossy().into_owned();
                projects.push(
                    read_project(&root, &name)
                        .with_context(|| format!("could not load project {name}"))?,
                );
            }
            projects.sort_by_key(|project| project.name.to_lowercase());
            Ok(HardwareProjectResult::Catalog(projects))
        }
        Op::Create { name } => {
            name_valid(&name)?;
            anyhow::ensure!(
                fs::read_dir(&root)?.count() < MAX_HARDWARE_PROJECTS,
                "project limit reached"
            );
            let path = root.join(&name);
            private_directory(&path)?;
            let project = HardwareProject {
                name,
                root: path.clone(),
                progress: [0; 6],
            };
            if let Err(error) = write_manifest(&project) {
                let _ = fs::remove_dir(&path); // Only the empty folder this operation created.
                return Err(error);
            }
            directory_result(project, PathBuf::new())
        }
        Op::Directory { name, relative } => directory_result(read_project(&root, &name)?, relative),
        Op::CreateFolder {
            name,
            relative,
            folder,
        } => {
            name_valid(&folder)?;
            let project = read_project(&root, &name)?;
            let target = contained(&project, &relative)?.join(folder);
            private_directory(&target)?;
            directory_result(project, relative)
        }
        Op::Import {
            name,
            relative,
            source,
        } => {
            let file_name = source
                .file_name()
                .and_then(|name| name.to_str())
                .context("invalid source filename")?;
            name_valid(file_name)?;
            let project = read_project(&root, &name)?;
            let destination = contained(&project, &relative)?.join(file_name);
            copy_file(&source, &destination)?;
            fs::File::open(destination.parent().context("import has no directory")?)?.sync_all()?;
            directory_result(project, relative)
        }
        Op::SaveProgress { name, progress } => {
            let mut project = read_project(&root, &name)?;
            project.progress = progress;
            write_manifest(&project)?;
            Ok(HardwareProjectResult::Progress(project))
        }
        Op::ImportBrowse { .. } => unreachable!("handled above"),
    }
}

#[cfg(test)]
#[path = "../tests/hardware_projects.rs"]
mod tests;

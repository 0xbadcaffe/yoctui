use super::*;

pub const MAX_HARDWARE_PROJECTS: usize = 128;
pub const MAX_HARDWARE_IMPORT_BYTES: u64 = 256 * 1024 * 1024;
pub const HARDWARE_BRINGUP_STAGES: [&str; 6] = [
    "Bootloader",
    "Kernel",
    "Device tree",
    "Drivers",
    "RootFS",
    "Packages",
];

pub fn validate_hardware_project_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 128
        || name.trim() != name
        || name.starts_with(".yoctui-")
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        || !matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [std::path::Component::Normal(_)]
        )
    {
        return Err("Use a nonempty folder name (up to 128 bytes), without paths or reserved .yoctui- names.".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProject {
    pub name: String,
    pub root: PathBuf,
    pub progress: [u8; 6],
}

impl HardwareProject {
    pub fn validate(&self) -> Result<(), String> {
        validate_hardware_project_name(&self.name)?;
        if !self.root.is_absolute() || self.progress.iter().any(|value| *value > 100) {
            return Err("Invalid project root or manual progress (expected 0–100).".into());
        }
        Ok(())
    }

    pub fn percent(&self) -> u16 {
        self.progress
            .iter()
            .map(|value| u16::from(*value))
            .sum::<u16>()
            / 6
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProjectEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: u64,
    pub kind: Option<HardwareDocumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareProjectOperation {
    List,
    Create {
        name: String,
    },
    Directory {
        name: String,
        relative: PathBuf,
    },
    CreateFolder {
        name: String,
        relative: PathBuf,
        folder: String,
    },
    Import {
        name: String,
        relative: PathBuf,
        source: PathBuf,
    },
    SaveProgress {
        name: String,
        progress: [u8; 6],
    },
    ImportBrowse {
        directory: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProjectRequest {
    pub generation: u64,
    pub operation: HardwareProjectOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareProjectResult {
    Catalog(Vec<HardwareProject>),
    Directory {
        project: HardwareProject,
        relative: PathBuf,
        entries: Vec<HardwareProjectEntry>,
    },
    Progress(HardwareProject),
    ImportBrowser {
        directory: PathBuf,
        entries: Vec<HardwareProjectEntry>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareProjectForm {
    Name {
        value: String,
    },
    Progress {
        values: [u8; 6],
        stage: usize,
        digits: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareProjectsState {
    pub visible: bool,
    pub catalog: Vec<HardwareProject>,
    pub project: Option<HardwareProject>,
    pub relative: PathBuf,
    pub entries: Vec<HardwareProjectEntry>,
    pub selection: usize,
    pub project_selection: usize,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
    pub form: Option<HardwareProjectForm>,
    pub import_browser: Option<(PathBuf, Vec<HardwareProjectEntry>, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareProjectAction {
    Request(HardwareProjectOperation),
    Finished {
        generation: u64,
        result: Result<HardwareProjectResult, String>,
    },
}

pub(crate) fn reduce_hardware_project(
    app: &mut App,
    action: HardwareProjectAction,
) -> Option<Effect> {
    let state = &mut app.hardware.projects;
    match action {
        HardwareProjectAction::Request(operation) => {
            if state.loading {
                return None;
            }
            state.generation = next_generation(&mut state.generation);
            state.loading = true;
            state.error = None;
            Some(Effect::Hardware(HardwareEffect::Project(
                HardwareProjectRequest {
                    generation: state.generation,
                    operation,
                },
            )))
        }
        HardwareProjectAction::Finished { generation, result } => {
            if generation != state.generation || !state.loading {
                return None;
            }
            state.loading = false;
            match result {
                Err(error) => state.error = Some(error),
                Ok(HardwareProjectResult::Catalog(mut catalog)) => {
                    catalog.truncate(MAX_HARDWARE_PROJECTS);
                    state.catalog = catalog;
                    state.project = None;
                    state.entries.clear();
                    state.selection = state
                        .project_selection
                        .min(state.catalog.len().saturating_sub(1));
                }
                Ok(HardwareProjectResult::Directory {
                    project,
                    relative,
                    mut entries,
                }) => {
                    entries.truncate(MAX_HARDWARE_BROWSER_ENTRIES);
                    state.project = Some(project);
                    state.relative = relative;
                    state.entries = entries;
                    state.selection = 0;
                    state.form = None;
                    state.import_browser = None;
                }
                Ok(HardwareProjectResult::Progress(project)) => {
                    state.project = Some(project);
                    state.form = None;
                }
                Ok(HardwareProjectResult::ImportBrowser {
                    directory,
                    mut entries,
                }) => {
                    entries.truncate(MAX_HARDWARE_BROWSER_ENTRIES);
                    state.import_browser = Some((directory, entries, 0));
                }
            }
            None
        }
    }
}

#[cfg(test)]
#[path = "../tests/hardware_projects.rs"]
mod tests;

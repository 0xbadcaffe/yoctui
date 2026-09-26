//! Recipe types.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub path: PathBuf,
    pub priority: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Recipe {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub preferred_version: Option<String>,
    #[serde(default)]
    pub file: Option<PathBuf>,
    #[serde(default)]
    pub append_count: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeWorkspaceStatus {
    Clean,
    Modified,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecipeIdentity {
    pub name: String,
    pub file: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolCapability {
    Available,
    MissingExecutable,
    Unavailable { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolWorkspace {
    NotMember,
    MissingDirectory {
        source_path: PathBuf,
    },
    Present {
        source_path: PathBuf,
        recipe_file: Option<PathBuf>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolGitState {
    NotApplicable,
    MissingExecutable,
    NotRepository,
    Available {
        repository_root: Option<PathBuf>,
        branch: Option<String>,
        upstream: Option<String>,
        ahead: usize,
        behind: usize,
        head: Option<String>,
        modified: usize,
        untracked: usize,
        conflicted: usize,
    },
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
    Malformed {
        message: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolStatusError {
    InvalidRecipeIdentity,
    DevtoolFailed {
        exit_code: Option<i32>,
        message: String,
    },
    MalformedOutput {
        line: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolStatus {
    pub identity: RecipeIdentity,
    pub capability: DevtoolCapability,
    pub workspace: DevtoolWorkspace,
    pub git: DevtoolGitState,
    pub error: Option<DevtoolStatusError>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevtoolAction {
    ModifyOrEdit,
    UpdateRecipe,
    Finish,
    Deploy,
    Undeploy,
    Reset,
    Upgrade,
}
impl DevtoolStatus {
    pub fn disabled_reason(&self, action: DevtoolAction) -> Option<String> {
        match &self.capability {
            DevtoolCapability::Available => {}
            DevtoolCapability::MissingExecutable => {
                return Some("Devtool executable is missing.".into());
            }
            DevtoolCapability::Unavailable { reason } => return Some(reason.clone()),
        }
        if let Some(error) = &self.error {
            return Some(format!("Devtool status is unavailable: {error:?}."));
        }
        match (&self.workspace, action) {
            (DevtoolWorkspace::NotMember, DevtoolAction::ModifyOrEdit) => None,
            (DevtoolWorkspace::NotMember, _) => {
                Some("Recipe is not in the Devtool workspace.".into())
            }
            (DevtoolWorkspace::MissingDirectory { .. }, DevtoolAction::Reset) => None,
            (DevtoolWorkspace::MissingDirectory { .. }, _) => {
                Some("Devtool workspace source directory is missing.".into())
            }
            (DevtoolWorkspace::Present { .. }, DevtoolAction::Finish) => match &self.git {
                DevtoolGitState::Available {
                    head: Some(_),
                    modified: 0,
                    untracked: 0,
                    conflicted: 0,
                    ..
                } => None,
                DevtoolGitState::Available { .. } => {
                    Some("Commit all workspace changes before Devtool finish.".into())
                }
                _ => Some("Authoritative Git status is unavailable.".into()),
            },
            (DevtoolWorkspace::Present { .. }, _) => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeBuildStatus {
    Idle,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeMetadata {
    pub recipe: String,
    pub workspace_status: Option<RecipeWorkspaceStatus>,
    pub build_status: Option<RecipeBuildStatus>,
    pub tasks: Option<Vec<String>>,
    pub sources: Option<Vec<PathBuf>>,
    pub patches: Option<Vec<String>>,
    pub packages: Option<Vec<String>>,
    pub history: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeEditor {
    pub recipe: String,
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub file_inventory_truncated: bool,
    pub selection: usize,
    pub focus: RecipeEditorFocus,
    pub language: SourceLanguage,
    pub document: TextAreaState,
    pub searching: bool,
    pub pending_search_position: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecipeEditorFocus {
    #[default]
    Files,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceLanguage {
    BitBake,
    C,
    Cpp,
    Rust,
    Python,
    Shell,
    JavaScript,
    TypeScript,
    Json,
    Toml,
    Yaml,
    Make,
    Markdown,
    DeviceTree,
    #[default]
    PlainText,
}

impl SourceLanguage {
    pub fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "bb" | "bbappend" | "bbclass" | "inc" | "conf" | "wks" => Self::BitBake,
            "c" | "h" => Self::C,
            "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" => Self::Cpp,
            "rs" => Self::Rust,
            "py" | "pyi" => Self::Python,
            "sh" | "bash" | "zsh" | "fish" => Self::Shell,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" | "mts" | "cts" => Self::TypeScript,
            "json" | "jsonc" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "md" | "markdown" => Self::Markdown,
            "dts" | "dtsi" => Self::DeviceTree,
            _ if name.ends_with(".wks.in") => Self::BitBake,
            _ if matches!(name.as_str(), "makefile" | "gnumakefile")
                || name.starts_with("makefile.") =>
            {
                Self::Make
            }
            _ => Self::PlainText,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::BitBake => "BitBake",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Shell => "Shell",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Json => "JSON",
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Make => "Make",
            Self::Markdown => "Markdown",
            Self::DeviceTree => "Device Tree",
            Self::PlainText => "Plain text",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskPicker {
    pub recipe: String,
    pub tasks: Vec<String>,
    pub selection: usize,
    pub force: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureTaskPicker {
    pub recipe: RecipeIdentity,
    pub tasks: Vec<String>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskLogChoice {
    pub task: String,
    pub state: TaskState,
    pub path: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskLogPicker {
    pub recipe: String,
    pub logs: Vec<RecipeTaskLogChoice>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePatchPicker {
    pub recipe: String,
    pub patches: Vec<PathBuf>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigSourceChoice {
    pub operation: String,
    pub path: PathBuf,
    pub line: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSourcePicker {
    pub identity: VariableIdentity,
    pub sources: Vec<ConfigSourceChoice>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigScopePicker {
    pub variable: String,
    pub scopes: Vec<Option<String>>,
    pub selection: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigComparisonOutcome {
    Equal,
    Different,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigComparisonField {
    pub global: Option<String>,
    pub recipe: Option<String>,
    pub outcome: ConfigComparisonOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigComparison {
    pub variable: String,
    pub recipe: String,
    pub effective: ConfigComparisonField,
    pub unexpanded: ConfigComparisonField,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEditRequest {
    pub identity: VariableIdentity,
    pub value: String,
    pub destination: PathBuf,
    pub assignment: String,
}

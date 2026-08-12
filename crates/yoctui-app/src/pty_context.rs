use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_PTY_ARGUMENT_BYTES, PtyCommandIdentity, PtySessionKind, PtyWorkspaceContext,
};

const MAX_CONTEXTS_PER_KIND: usize = 4_096;
const MAX_ENVIRONMENT_ENTRIES: usize = 4_096;
const MAX_ENVIRONMENT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyContextEntry {
    pub identity: String,
    pub directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPtyEnvironment {
    pub identity: String,
    pub shell: PathBuf,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalContextEntry {
    identity: String,
    directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalEnvironment {
    identity: String,
    shell: PathBuf,
    environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyContextAuthority {
    owner_identity: String,
    source_dir: PathBuf,
    build_dir: PathBuf,
    build_environment: CanonicalEnvironment,
    layers: Vec<CanonicalContextEntry>,
    recipes: Vec<CanonicalContextEntry>,
    devtool_workspaces: Vec<CanonicalContextEntry>,
    deploy_directories: Vec<CanonicalContextEntry>,
    sdk_environments: Vec<(CanonicalContextEntry, CanonicalEnvironment)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyContextAction {
    BuildDirectory,
    SourceTree,
    SelectedLayer { identity: String },
    SelectedRecipeSource { identity: String },
    DevtoolWorkspace { identity: String },
    SdkEnvironment { identity: String },
    DeployDirectory { identity: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyContextLaunch {
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub environment_identity: String,
    pub environment: BTreeMap<String, String>,
    pub workspace: PtyWorkspaceContext,
}

impl PtyContextAuthority {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner_identity: String,
        source_dir: PathBuf,
        build_dir: PathBuf,
        build_environment: VerifiedPtyEnvironment,
        layers: Vec<PtyContextEntry>,
        recipes: Vec<PtyContextEntry>,
        devtool_workspaces: Vec<PtyContextEntry>,
        deploy_directories: Vec<PtyContextEntry>,
        sdk_environments: Vec<(PtyContextEntry, VerifiedPtyEnvironment)>,
    ) -> Result<Self, PtyContextError> {
        validate_identity(&owner_identity)?;
        for (kind, count) in [
            ("layers", layers.len()),
            ("recipes", recipes.len()),
            ("Devtool workspaces", devtool_workspaces.len()),
            ("deploy directories", deploy_directories.len()),
            ("SDK environments", sdk_environments.len()),
        ] {
            if count > MAX_CONTEXTS_PER_KIND {
                return Err(PtyContextError::TooManyContexts { kind, count });
            }
        }
        let source_dir = canonical_directory(&source_dir)?;
        let build_dir = canonical_directory(&build_dir)?;
        let build_environment = canonical_environment(build_environment)?;
        let layers = canonical_entries(layers)?;
        let recipes = canonical_entries(recipes)?;
        let devtool_workspaces = canonical_entries(devtool_workspaces)?;
        let deploy_directories = canonical_entries(deploy_directories)?;
        let mut canonical_sdks = Vec::with_capacity(sdk_environments.len());
        let mut sdk_ids = BTreeSet::new();
        for (entry, environment) in sdk_environments {
            let entry = canonical_entry(entry)?;
            if !sdk_ids.insert(entry.identity.clone()) {
                return Err(PtyContextError::DuplicateIdentity(entry.identity));
            }
            canonical_sdks.push((entry, canonical_environment(environment)?));
        }
        Ok(Self {
            owner_identity,
            source_dir,
            build_dir,
            build_environment,
            layers,
            recipes,
            devtool_workspaces,
            deploy_directories,
            sdk_environments: canonical_sdks,
        })
    }

    pub fn resolve(&self, action: PtyContextAction) -> Result<PtyContextLaunch, PtyContextError> {
        let (name, kind, directory, environment) = match action {
            PtyContextAction::BuildDirectory => (
                "Build directory shell".into(),
                PtySessionKind::BuildShell,
                &self.build_dir,
                &self.build_environment,
            ),
            PtyContextAction::SourceTree => (
                "Source tree shell".into(),
                PtySessionKind::SourceShell,
                &self.source_dir,
                &self.build_environment,
            ),
            PtyContextAction::SelectedLayer { identity } => (
                format!("Layer {identity}"),
                PtySessionKind::LayerShell,
                lookup(&self.layers, "layer", &identity)?,
                &self.build_environment,
            ),
            PtyContextAction::SelectedRecipeSource { identity } => (
                format!("Recipe {identity}"),
                PtySessionKind::RecipeShell,
                lookup(&self.recipes, "recipe source", &identity)?,
                &self.build_environment,
            ),
            PtyContextAction::DevtoolWorkspace { identity } => (
                format!("Devtool {identity}"),
                PtySessionKind::DevtoolShell,
                lookup(&self.devtool_workspaces, "Devtool workspace", &identity)?,
                &self.build_environment,
            ),
            PtyContextAction::DeployDirectory { identity } => (
                format!("Deploy {identity}"),
                PtySessionKind::DeployShell,
                lookup(&self.deploy_directories, "deploy directory", &identity)?,
                &self.build_environment,
            ),
            PtyContextAction::SdkEnvironment { identity } => {
                let (entry, environment) = self
                    .sdk_environments
                    .iter()
                    .find(|(entry, _)| entry.identity == identity)
                    .ok_or_else(|| PtyContextError::StaleIdentity {
                        kind: "SDK environment",
                        identity: identity.clone(),
                    })?;
                (
                    format!("SDK {identity}"),
                    PtySessionKind::SdkShell,
                    &entry.directory,
                    environment,
                )
            }
        };
        revalidate_directory(directory)?;
        revalidate_executable(&environment.shell)?;
        Ok(PtyContextLaunch {
            name,
            kind,
            cwd: directory.clone(),
            command: PtyCommandIdentity {
                executable: environment.shell.clone(),
                arguments: vec!["-i".into()],
            },
            environment_identity: environment.identity.clone(),
            environment: environment.environment.clone(),
            workspace: PtyWorkspaceContext {
                source_dir: self.source_dir.clone(),
                build_dir: self.build_dir.clone(),
                authorized_context_roots: vec![directory.clone()],
                owner_identity: self.owner_identity.clone(),
            },
        })
    }
}

fn canonical_entries(
    entries: Vec<PtyContextEntry>,
) -> Result<Vec<CanonicalContextEntry>, PtyContextError> {
    let mut identities = BTreeSet::new();
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        let entry = canonical_entry(entry)?;
        if !identities.insert(entry.identity.clone()) {
            return Err(PtyContextError::DuplicateIdentity(entry.identity));
        }
        result.push(entry);
    }
    Ok(result)
}

fn canonical_entry(entry: PtyContextEntry) -> Result<CanonicalContextEntry, PtyContextError> {
    validate_identity(&entry.identity)?;
    Ok(CanonicalContextEntry {
        identity: entry.identity,
        directory: canonical_directory(&entry.directory)?,
    })
}

fn canonical_environment(
    environment: VerifiedPtyEnvironment,
) -> Result<CanonicalEnvironment, PtyContextError> {
    validate_identity(&environment.identity)?;
    let shell = canonical_file(&environment.shell)?;
    if environment.environment.len() > MAX_ENVIRONMENT_ENTRIES {
        return Err(PtyContextError::EnvironmentTooLarge);
    }
    let mut bytes = 0_usize;
    for (name, value) in &environment.environment {
        bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
        if name.is_empty() || name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(PtyContextError::InvalidEnvironment(name.clone()));
        }
    }
    if bytes > MAX_ENVIRONMENT_BYTES {
        return Err(PtyContextError::EnvironmentTooLarge);
    }
    Ok(CanonicalEnvironment {
        identity: environment.identity,
        shell,
        environment: environment.environment,
    })
}

fn lookup<'a>(
    entries: &'a [CanonicalContextEntry],
    kind: &'static str,
    identity: &str,
) -> Result<&'a PathBuf, PtyContextError> {
    entries
        .iter()
        .find(|entry| entry.identity == identity)
        .map(|entry| &entry.directory)
        .ok_or_else(|| PtyContextError::StaleIdentity {
            kind,
            identity: identity.into(),
        })
}

fn canonical_directory(path: &Path) -> Result<PathBuf, PtyContextError> {
    let canonical = fs::canonicalize(path).map_err(|error| PtyContextError::Path {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !canonical.is_dir() {
        return Err(PtyContextError::NotDirectory(canonical));
    }
    Ok(canonical)
}

fn canonical_file(path: &Path) -> Result<PathBuf, PtyContextError> {
    let canonical = fs::canonicalize(path).map_err(|error| PtyContextError::Path {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !canonical.is_file() {
        return Err(PtyContextError::NotExecutable(canonical));
    }
    revalidate_executable(&canonical)?;
    Ok(canonical)
}

fn revalidate_directory(path: &Path) -> Result<(), PtyContextError> {
    let current = canonical_directory(path)?;
    if current != path {
        return Err(PtyContextError::PathIdentityChanged(path.to_path_buf()));
    }
    Ok(())
}

fn revalidate_executable(path: &Path) -> Result<(), PtyContextError> {
    let metadata = fs::metadata(path).map_err(|error| PtyContextError::Path {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(PtyContextError::NotExecutable(path.to_path_buf()));
        }
    }
    #[cfg(not(unix))]
    if !metadata.is_file() {
        return Err(PtyContextError::NotExecutable(path.to_path_buf()));
    }
    Ok(())
}

fn validate_identity(identity: &str) -> Result<(), PtyContextError> {
    if identity.trim().is_empty()
        || identity.len() > MAX_PTY_ARGUMENT_BYTES
        || identity.chars().any(char::is_control)
    {
        return Err(PtyContextError::InvalidIdentity);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PtyContextError {
    #[error("invalid or oversized PTY context identity")]
    InvalidIdentity,
    #[error("duplicate PTY context identity: {0}")]
    DuplicateIdentity(String),
    #[error("too many PTY {kind}: {count}")]
    TooManyContexts { kind: &'static str, count: usize },
    #[error("PTY context path {path} is unavailable: {message}")]
    Path { path: PathBuf, message: String },
    #[error("PTY context is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("PTY shell is not an executable file: {0}")]
    NotExecutable(PathBuf),
    #[error("PTY context path identity changed: {0}")]
    PathIdentityChanged(PathBuf),
    #[error("stale {kind} identity: {identity}")]
    StaleIdentity {
        kind: &'static str,
        identity: String,
    },
    #[error("invalid captured PTY environment variable: {0}")]
    InvalidEnvironment(String),
    #[error("captured PTY environment exceeds configured bounds")]
    EnvironmentTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoctui_model::{PtyDimensions, PtySession, PtySessionId, PtySessionSpec};

    fn fixture() -> (PathBuf, PtyContextAuthority) {
        let nonce = std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("yoctui-pty-context-{}-{nonce}", std::process::id()));
        for directory in [
            "source", "build", "layer", "recipe", "devtool", "deploy", "sdk",
        ] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        let shell = fs::canonicalize("/bin/sh").unwrap();
        let build_environment = VerifiedPtyEnvironment {
            identity: "build-env-7".into(),
            shell: shell.clone(),
            environment: BTreeMap::from([(
                "BUILDDIR".into(),
                root.join("build").display().to_string(),
            )]),
        };
        let sdk_environment = VerifiedPtyEnvironment {
            identity: "sdk-env-3".into(),
            shell,
            environment: BTreeMap::from([(
                "SDKTARGETSYSROOT".into(),
                root.join("sdk/sysroot").display().to_string(),
            )]),
        };
        let entry = |identity: &str, directory: &str| PtyContextEntry {
            identity: identity.into(),
            directory: root.join(directory),
        };
        let authority = PtyContextAuthority::new(
            "workspace-1".into(),
            root.join("source"),
            root.join("build"),
            build_environment,
            vec![entry("meta-test", "layer")],
            vec![entry("busybox", "recipe")],
            vec![entry("devtool:busybox", "devtool")],
            vec![entry("qemux86-64", "deploy")],
            vec![(entry("sdk-x86_64", "sdk"), sdk_environment)],
        )
        .unwrap();
        (root, authority)
    }

    #[test]
    fn pty_context_resolves_all_authoritative_routes_without_shell_strings() {
        let (root, authority) = fixture();
        let cases = [
            (
                PtyContextAction::BuildDirectory,
                PtySessionKind::BuildShell,
                "build",
                "build-env-7",
            ),
            (
                PtyContextAction::SourceTree,
                PtySessionKind::SourceShell,
                "source",
                "build-env-7",
            ),
            (
                PtyContextAction::SelectedLayer {
                    identity: "meta-test".into(),
                },
                PtySessionKind::LayerShell,
                "layer",
                "build-env-7",
            ),
            (
                PtyContextAction::SelectedRecipeSource {
                    identity: "busybox".into(),
                },
                PtySessionKind::RecipeShell,
                "recipe",
                "build-env-7",
            ),
            (
                PtyContextAction::DevtoolWorkspace {
                    identity: "devtool:busybox".into(),
                },
                PtySessionKind::DevtoolShell,
                "devtool",
                "build-env-7",
            ),
            (
                PtyContextAction::DeployDirectory {
                    identity: "qemux86-64".into(),
                },
                PtySessionKind::DeployShell,
                "deploy",
                "build-env-7",
            ),
            (
                PtyContextAction::SdkEnvironment {
                    identity: "sdk-x86_64".into(),
                },
                PtySessionKind::SdkShell,
                "sdk",
                "sdk-env-3",
            ),
        ];
        for (action, kind, directory, environment) in cases {
            let launch = authority.resolve(action).unwrap();
            assert_eq!(launch.kind, kind);
            assert_eq!(launch.cwd, fs::canonicalize(root.join(directory)).unwrap());
            assert_eq!(launch.command.arguments, vec!["-i"]);
            assert_eq!(launch.environment_identity, environment);
            assert_eq!(launch.workspace.owner_identity, "workspace-1");
            PtySession::new(
                PtySessionSpec {
                    id: PtySessionId(1),
                    name: launch.name.clone(),
                    kind: launch.kind,
                    cwd: launch.cwd.clone(),
                    command: launch.command.clone(),
                    dimensions: PtyDimensions {
                        columns: 80,
                        rows: 24,
                    },
                    restartable: true,
                    workspace: launch.workspace.clone(),
                },
                10,
            )
            .unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_context_rejects_stale_identity_and_changed_path() {
        let (root, authority) = fixture();
        assert!(matches!(
            authority.resolve(PtyContextAction::SelectedLayer {
                identity: "removed".into()
            }),
            Err(PtyContextError::StaleIdentity { kind: "layer", .. })
        ));
        fs::remove_dir_all(root.join("layer")).unwrap();
        assert!(matches!(
            authority.resolve(PtyContextAction::SelectedLayer {
                identity: "meta-test".into()
            }),
            Err(PtyContextError::Path { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_context_rejects_duplicate_authority_and_untrusted_environment() {
        let (root, _) = fixture();
        let shell = fs::canonicalize("/bin/sh").unwrap();
        let environment = VerifiedPtyEnvironment {
            identity: "build".into(),
            shell,
            environment: BTreeMap::from([("BAD=NAME".into(), "value".into())]),
        };
        let entry = PtyContextEntry {
            identity: "same".into(),
            directory: root.join("layer"),
        };
        assert!(matches!(
            PtyContextAuthority::new(
                "workspace".into(),
                root.join("source"),
                root.join("build"),
                environment,
                vec![entry.clone(), entry],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new()
            ),
            Err(PtyContextError::InvalidEnvironment(_))
        ));
        let valid_environment = VerifiedPtyEnvironment {
            identity: "build".into(),
            shell: fs::canonicalize("/bin/sh").unwrap(),
            environment: BTreeMap::new(),
        };
        let entry = PtyContextEntry {
            identity: "same".into(),
            directory: root.join("layer"),
        };
        assert!(matches!(
            PtyContextAuthority::new(
                "workspace".into(),
                root.join("source"),
                root.join("build"),
                valid_environment,
                vec![entry.clone(), entry],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new()
            ),
            Err(PtyContextError::DuplicateIdentity(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }
}

use crate::{BackgroundJobId, BuildRequest};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

pub const MAX_SDK_ARTIFACTS: usize = 4_096;
pub const MAX_SDK_ASSOCIATIONS: usize = 256;
pub const MAX_SDK_LIMITATIONS: usize = 64;
pub const MAX_SDK_NATIVE_ARGUMENTS: usize = 128;
pub const MAX_SDK_NATIVE_ARGUMENT_BYTES: usize = 4_096;

fn token_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= 4_096
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkKind {
    Standard,
    Extensible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkBuildAction {
    Populate(SdkKind),
    Test(SdkKind),
}

impl SdkBuildAction {
    pub fn task(self) -> &'static str {
        match self {
            Self::Populate(SdkKind::Standard) => "populate_sdk",
            Self::Populate(SdkKind::Extensible) => "populate_sdk_ext",
            Self::Test(SdkKind::Standard) => "testsdk",
            Self::Test(SdkKind::Extensible) => "testsdkext",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkBuildPreview {
    pub machine: String,
    pub distro: String,
    pub image: String,
    pub action: SdkBuildAction,
    pub request: BuildRequest,
}

impl SdkBuildPreview {
    pub fn new(
        machine: String,
        distro: String,
        image: String,
        action: SdkBuildAction,
    ) -> Result<Self, &'static str> {
        if !token_is_valid(&machine) || !token_is_valid(&distro) || !token_is_valid(&image) {
            return Err("SDK build identity must use bounded BitBake tokens");
        }
        let request = BuildRequest {
            targets: vec![image.clone()],
            task: Some(action.task().into()),
            force: false,
        };
        request
            .validate()
            .map_err(|_| "SDK build request is invalid")?;
        Ok(Self {
            machine,
            distro,
            image,
            action,
            request,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkArtifactKind {
    Installer,
    Checksum,
    Manifest,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SdkArtifactIdentity {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

impl SdkArtifactIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !absolute_normal_path(&self.path) {
            return Err("SDK artifact identity requires a normalized absolute path");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkArtifact {
    pub identity: SdkArtifactIdentity,
    pub kind: SdkArtifactKind,
    pub sdk_kind: Option<SdkKind>,
    pub machine: Option<String>,
    pub host_tuple: Option<String>,
    pub target_tuple: Option<String>,
    pub checksums: Vec<PathBuf>,
    pub manifests: Vec<PathBuf>,
    pub published: Option<bool>,
}

impl SdkArtifact {
    pub fn validate(&self, root: &Path) -> Result<(), &'static str> {
        self.identity.validate()?;
        if !absolute_normal_path(root) || !self.identity.path.starts_with(root) {
            return Err("SDK artifact escapes its authoritative deploy root");
        }
        if self
            .machine
            .iter()
            .chain(self.host_tuple.iter())
            .chain(self.target_tuple.iter())
            .any(|value| !token_is_valid(value))
        {
            return Err("SDK artifact metadata contains an invalid token");
        }
        if self.checksums.len() > MAX_SDK_ASSOCIATIONS
            || self.manifests.len() > MAX_SDK_ASSOCIATIONS
            || self
                .checksums
                .iter()
                .chain(self.manifests.iter())
                .any(|path| !absolute_normal_path(path) || !path.starts_with(root))
        {
            return Err("SDK artifact associations are invalid or exceed their bound");
        }
        Ok(())
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.to_ascii_lowercase();
        query.is_empty()
            || self
                .identity
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&query)
            || self
                .machine
                .iter()
                .chain(self.host_tuple.iter())
                .chain(self.target_tuple.iter())
                .any(|value| value.to_ascii_lowercase().contains(&query))
            || format!("{:?}", self.kind)
                .to_ascii_lowercase()
                .contains(&query)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkArtifactInventoryRequest {
    pub generation: u64,
    pub root: PathBuf,
    pub machine: String,
}

impl SdkArtifactInventoryRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0
            || !absolute_normal_path(&self.root)
            || !token_is_valid(&self.machine)
        {
            return Err("SDK inventory request identity is invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SdkArtifactInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: SdkArtifactInventoryRequest,
    },
    AvailableEmpty {
        request: SdkArtifactInventoryRequest,
    },
    Available {
        request: SdkArtifactInventoryRequest,
        artifacts: Vec<SdkArtifact>,
    },
    Partial {
        request: SdkArtifactInventoryRequest,
        artifacts: Vec<SdkArtifact>,
        limitations: Vec<String>,
    },
    Failed {
        request: SdkArtifactInventoryRequest,
        message: String,
    },
}

impl SdkArtifactInventoryState {
    pub fn artifacts(&self) -> Option<&[SdkArtifact]> {
        match self {
            Self::Available { artifacts, .. } | Self::Partial { artifacts, .. } => Some(artifacts),
            Self::AvailableEmpty { .. } => Some(&[]),
            Self::NotLoaded | Self::Loading { .. } | Self::Failed { .. } => None,
        }
    }
}

pub fn normalize_sdk_artifacts(
    request: &SdkArtifactInventoryRequest,
    artifacts: Vec<SdkArtifact>,
) -> Result<Vec<SdkArtifact>, &'static str> {
    request.validate()?;
    if artifacts.len() > MAX_SDK_ARTIFACTS {
        return Err("SDK artifact inventory exceeds its record bound");
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for mut artifact in artifacts {
        artifact.validate(&request.root)?;
        artifact.checksums.sort();
        artifact.checksums.dedup();
        artifact.manifests.sort();
        artifact.manifests.dedup();
        if seen.insert(artifact.identity.clone()) {
            normalized.push(artifact);
        }
    }
    normalized.sort_by(|left, right| left.identity.cmp(&right.identity));
    Ok(normalized)
}

pub fn normalize_sdk_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|limitation| {
        !limitation.is_empty()
            && limitation.len() <= 1_024
            && !limitation.chars().any(char::is_control)
    });
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_SDK_LIMITATIONS);
    limitations
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SdkPublishDraft {
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkPublishRequest {
    pub executable: PathBuf,
    pub artifact: SdkArtifactIdentity,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkPublishPreview {
    pub request: SdkPublishRequest,
    pub argv: Vec<PathBuf>,
}

impl SdkPublishPreview {
    pub fn new(
        executable: PathBuf,
        artifact: SdkArtifactIdentity,
        destination: PathBuf,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable)
            || !absolute_normal_path(&destination)
            || artifact.validate().is_err()
        {
            return Err("SDK publication preview identity is invalid");
        }
        let request = SdkPublishRequest {
            executable: executable.clone(),
            artifact,
            destination,
        };
        let argv = vec![
            executable,
            request.artifact.path.clone(),
            request.destination.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkNativeMode {
    FindSysroot,
    RunNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SdkToolCapability {
    #[default]
    NotInspected,
    Available {
        publish: Option<PathBuf>,
        find_sysroot: Option<PathBuf>,
        run_native: Option<PathBuf>,
    },
    Failed {
        message: String,
    },
}

impl SdkToolCapability {
    pub fn executable_for(&self, mode: SdkNativeMode) -> Result<PathBuf, &'static str> {
        let Self::Available {
            find_sysroot,
            run_native,
            ..
        } = self
        else {
            return Err("SDK native-tool capability is unavailable");
        };
        match mode {
            SdkNativeMode::FindSysroot => find_sysroot.clone(),
            SdkNativeMode::RunNative => run_native.clone(),
        }
        .filter(|path| absolute_normal_path(path))
        .ok_or("the requested SDK native tool is unavailable")
    }

    pub fn publish_executable(&self) -> Result<PathBuf, &'static str> {
        let Self::Available { publish, .. } = self else {
            return Err("SDK publication capability is unavailable");
        };
        publish
            .clone()
            .filter(|path| absolute_normal_path(path))
            .ok_or("oe-publish-sdk is unavailable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativeDraft {
    pub mode: SdkNativeMode,
    pub extracted_root: String,
    pub recipe: String,
    pub tool: String,
    pub arguments: Vec<String>,
}

impl Default for SdkNativeDraft {
    fn default() -> Self {
        Self {
            mode: SdkNativeMode::FindSysroot,
            extracted_root: String::new(),
            recipe: String::new(),
            tool: String::new(),
            arguments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativeRequest {
    pub executable: PathBuf,
    pub mode: SdkNativeMode,
    pub extracted_root: Option<PathBuf>,
    pub recipe: String,
    pub tool: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativePreview {
    pub request: SdkNativeRequest,
    pub argv: Vec<PathBuf>,
}

impl SdkNativePreview {
    pub fn new(request: SdkNativeRequest) -> Result<Self, &'static str> {
        if !absolute_normal_path(&request.executable)
            || !token_is_valid(&request.recipe)
            || request
                .extracted_root
                .as_ref()
                .is_some_and(|path| !absolute_normal_path(path))
            || request.arguments.len() > MAX_SDK_NATIVE_ARGUMENTS
            || request.arguments.iter().any(|argument| {
                argument.is_empty()
                    || argument.len() > MAX_SDK_NATIVE_ARGUMENT_BYTES
                    || argument.chars().any(char::is_control)
            })
        {
            return Err("SDK native-tool request is invalid or exceeds its bound");
        }
        match request.mode {
            SdkNativeMode::FindSysroot
                if request.tool.is_some() || !request.arguments.is_empty() =>
            {
                return Err("find-native-sysroot does not accept a tool or tool arguments");
            }
            SdkNativeMode::RunNative
                if request
                    .tool
                    .as_deref()
                    .is_none_or(|tool| !token_is_valid(tool)) =>
            {
                return Err("run-native requires a bounded tool token");
            }
            _ => {}
        }
        let mut argv = vec![request.executable.clone(), request.recipe.clone().into()];
        if let Some(tool) = &request.tool {
            argv.push(tool.into());
        }
        argv.extend(request.arguments.iter().map(PathBuf::from));
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkOperation {
    Publish(SdkPublishRequest),
    Native(SdkNativeRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SdkSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkSession {
    pub id: SdkSessionId,
    pub background_job_id: BackgroundJobId,
    pub operation: SdkOperation,
    pub exit_code: Option<i32>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkOutputStream {
    Stdout,
    Stderr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_workflow_build_preview_maps_exact_tasks() {
        for (action, task) in [
            (SdkBuildAction::Populate(SdkKind::Standard), "populate_sdk"),
            (
                SdkBuildAction::Populate(SdkKind::Extensible),
                "populate_sdk_ext",
            ),
            (SdkBuildAction::Test(SdkKind::Standard), "testsdk"),
            (SdkBuildAction::Test(SdkKind::Extensible), "testsdkext"),
        ] {
            let preview = SdkBuildPreview::new(
                "qemux86-64".into(),
                "poky".into(),
                "core-image-minimal".into(),
                action,
            )
            .unwrap();
            assert_eq!(preview.request.task.as_deref(), Some(task));
            assert_eq!(preview.request.targets, ["core-image-minimal"]);
            assert!(!preview.request.force);
        }
        assert!(
            SdkBuildPreview::new(
                "../machine".into(),
                "poky".into(),
                "image".into(),
                SdkBuildAction::Populate(SdkKind::Standard),
            )
            .is_err()
        );
    }

    #[test]
    fn sdk_workflow_inventory_and_tool_previews_are_exact_and_bounded() {
        let request = SdkArtifactInventoryRequest {
            generation: 1,
            root: "/deploy/sdk".into(),
            machine: "qemux86-64".into(),
        };
        let artifact = SdkArtifact {
            identity: SdkArtifactIdentity {
                path: "/deploy/sdk/poky.sh".into(),
                size_bytes: 42,
                modified_unix_seconds: 7,
            },
            kind: SdkArtifactKind::Installer,
            sdk_kind: Some(SdkKind::Standard),
            machine: Some("qemux86-64".into()),
            host_tuple: Some("x86_64-pokysdk-linux".into()),
            target_tuple: Some("x86_64-poky-linux".into()),
            checksums: vec!["/deploy/sdk/poky.sh.sha256".into()],
            manifests: Vec::new(),
            published: None,
        };
        assert_eq!(
            normalize_sdk_artifacts(&request, vec![artifact.clone(), artifact])
                .unwrap()
                .len(),
            1
        );
        let publish = SdkPublishPreview::new(
            "/opt/poky/oe-publish-sdk".into(),
            SdkArtifactIdentity {
                path: "/deploy/sdk/poky.sh".into(),
                size_bytes: 42,
                modified_unix_seconds: 7,
            },
            "/srv/sdk".into(),
        )
        .unwrap();
        assert_eq!(publish.argv[1], Path::new("/deploy/sdk/poky.sh"));
        let native = SdkNativePreview::new(SdkNativeRequest {
            executable: "/opt/poky/oe-run-native".into(),
            mode: SdkNativeMode::RunNative,
            extracted_root: Some("/opt/sdk".into()),
            recipe: "cmake-native".into(),
            tool: Some("cmake".into()),
            arguments: vec!["--version".into()],
        })
        .unwrap();
        assert_eq!(
            native.argv,
            [
                "/opt/poky/oe-run-native",
                "cmake-native",
                "cmake",
                "--version"
            ]
            .map(PathBuf::from)
        );
    }
}

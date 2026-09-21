pub const MAX_SDK_ARTIFACTS: usize = 4_096;
pub const MAX_SDK_ASSOCIATIONS: usize = 256;
pub const MAX_SDK_LIMITATIONS: usize = 64;
pub const MAX_SDK_NATIVE_ARGUMENTS: usize = 128;
pub const MAX_SDK_NATIVE_ARGUMENT_BYTES: usize = 4_096;
pub const MAX_SDK_NATIVE_ARGUMENT_INPUT_BYTES: usize = 4_096;

fn token_is_valid(value: &str) -> bool {
    is_bounded_identifier(value, 256)
}

fn absolute_normal_path(path: &Path) -> bool {
    is_absolute_normal_path_within(path, 4_096)
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

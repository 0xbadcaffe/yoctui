pub const MAX_IMAGE_ARTIFACT_RECORDS: usize = 4_096;
pub const MAX_IMAGE_ARTIFACT_ASSOCIATED_FILES: usize = 256;
pub const MAX_IMAGE_ARTIFACT_CHECKSUMS: usize = 64;
pub const MAX_IMAGE_ARTIFACT_LIMITATIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageArtifactIdentity {
    pub machine: String,
    pub image: String,
    pub path: PathBuf,
}

impl ImageArtifactIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !image_token_is_valid(&self.machine) {
            return Err("image artifact machines must be bounded non-empty tokens");
        }
        if !image_token_is_valid(&self.image) {
            return Err("image artifact targets must be bounded non-empty tokens");
        }
        if !artifact_path_is_valid(&self.path) {
            return Err("image artifact paths must be normalized absolute non-root paths");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ImageArtifactField<T> {
    #[default]
    Unavailable,
    Available(T),
}

impl<T> ImageArtifactField<T> {
    pub fn available(&self) -> Option<&T> {
        match self {
            Self::Unavailable => None,
            Self::Available(value) => Some(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImageArtifactKind {
    RootFilesystem,
    Kernel,
    Bootloader,
    Wic,
    Manifest,
    LicenseManifest,
    Spdx,
    Checksum,
    Other,
}

/// Transport context used by the evaluated terminal-image policy. Native image
/// protocols are deliberately not part of the closed result while the deploy
/// inventory has no raster MIME authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImagePreviewTransport {
    DirectTerminal,
    Ssh,
    Tmux,
    SshThroughTmux,
    TestBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePreviewFallback {
    ExactArtifactMetadata,
    RootfsComposition,
}

impl ImagePreviewFallback {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExactArtifactMetadata => "exact artifact metadata and existing workflows",
            Self::RootfsComposition => "Rootfs packages/filesystem composition",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagePreviewDecision {
    pub transport: ImagePreviewTransport,
    pub fallback: ImagePreviewFallback,
    pub reason: &'static str,
}

impl ImagePreviewDecision {
    pub const fn native_terminal_graphics_enabled(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageArtifactAssociation {
    Manifest,
    License,
    Spdx,
    Wic,
}

impl ImageArtifactKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::RootFilesystem => "root filesystem",
            Self::Kernel => "kernel",
            Self::Bootloader => "bootloader",
            Self::Wic => "wic",
            Self::Manifest => "manifest",
            Self::LicenseManifest => "license manifest",
            Self::Spdx => "spdx/sbom",
            Self::Checksum => "checksum",
            Self::Other => "other",
        }
    }

    pub const fn image_preview_decision(
        self,
        transport: ImagePreviewTransport,
    ) -> ImagePreviewDecision {
        let (fallback, reason) = match self {
            Self::RootFilesystem => (
                ImagePreviewFallback::RootfsComposition,
                "root filesystem artifacts are storage images, not raster images",
            ),
            Self::Wic => (
                ImagePreviewFallback::ExactArtifactMetadata,
                "Wic artifacts are disk images, not raster images",
            ),
            Self::Kernel | Self::Bootloader => (
                ImagePreviewFallback::ExactArtifactMetadata,
                "boot artifacts are executable or binary payloads, not raster images",
            ),
            Self::Manifest | Self::LicenseManifest | Self::Spdx | Self::Checksum => (
                ImagePreviewFallback::ExactArtifactMetadata,
                "metadata artifacts already have exact text and open-file workflows",
            ),
            Self::Other => (
                ImagePreviewFallback::ExactArtifactMetadata,
                "the deploy scanner has no authoritative raster MIME classification",
            ),
        };
        ImagePreviewDecision {
            transport,
            fallback,
            reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageChecksum {
    pub algorithm: String,
    pub digest: String,
    pub source: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageArtifact {
    pub identity: ImageArtifactIdentity,
    pub kind: ImageArtifactKind,
    pub size_bytes: ImageArtifactField<u64>,
    pub modified_unix_seconds: ImageArtifactField<u64>,
    pub checksums: ImageArtifactField<Vec<ImageChecksum>>,
    pub manifests: ImageArtifactField<Vec<PathBuf>>,
    pub licenses: ImageArtifactField<Vec<PathBuf>>,
    pub spdx: ImageArtifactField<Vec<PathBuf>>,
    pub wic_files: ImageArtifactField<Vec<PathBuf>>,
}

impl ImageArtifact {
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.to_ascii_lowercase();
        if query.is_empty() {
            return true;
        }
        let text_matches = |value: &str| value.to_ascii_lowercase().contains(&query);
        text_matches(&self.identity.machine)
            || text_matches(&self.identity.image)
            || text_matches(&self.identity.path.to_string_lossy())
            || text_matches(self.kind.label())
            || self
                .size_bytes
                .available()
                .is_some_and(|value| value.to_string().contains(&query))
            || self
                .modified_unix_seconds
                .available()
                .is_some_and(|value| value.to_string().contains(&query))
            || self.checksums.available().is_some_and(|checksums| {
                checksums.iter().any(|checksum| {
                    text_matches(&checksum.algorithm)
                        || text_matches(&checksum.digest)
                        || text_matches(&checksum.source.to_string_lossy())
                })
            })
            || [&self.manifests, &self.licenses, &self.spdx, &self.wic_files]
                .into_iter()
                .any(|field| {
                    field.available().is_some_and(|paths| {
                        paths
                            .iter()
                            .any(|path| text_matches(&path.to_string_lossy()))
                    })
                })
    }

    pub fn associated_paths(&self, association: ImageArtifactAssociation) -> Option<&[PathBuf]> {
        let field = match association {
            ImageArtifactAssociation::Manifest => &self.manifests,
            ImageArtifactAssociation::License => &self.licenses,
            ImageArtifactAssociation::Spdx => &self.spdx,
            ImageArtifactAssociation::Wic => &self.wic_files,
        };
        field.available().map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageArtifactInventory {
    pub machine: String,
    pub deploy_directory: ImageArtifactField<PathBuf>,
    pub artifacts: Vec<ImageArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageArtifactRequest {
    pub generation: u64,
    pub machine: String,
}

impl ImageArtifactRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 {
            return Err("image artifact request generations must be non-zero");
        }
        if !image_token_is_valid(&self.machine) {
            return Err("image artifact request machines must be bounded non-empty tokens");
        }
        Ok(())
    }
}


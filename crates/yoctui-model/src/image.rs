use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageArtifactNormalizationReport {
    pub duplicate_records: usize,
    pub invalid_records: usize,
    pub invalid_fields: usize,
    pub truncated_records: usize,
    pub truncated_associated_files: usize,
    pub truncated_checksums: usize,
}

impl ImageArtifactNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.invalid_records > 0
            || self.invalid_fields > 0
            || self.truncated_records > 0
            || self.truncated_associated_files > 0
            || self.truncated_checksums > 0
    }
}

pub fn normalize_image_artifact_inventory(
    request: &ImageArtifactRequest,
    mut inventory: ImageArtifactInventory,
    max_records: usize,
) -> (
    Option<ImageArtifactInventory>,
    ImageArtifactNormalizationReport,
) {
    let mut report = ImageArtifactNormalizationReport::default();
    if request.validate().is_err()
        || inventory.machine != request.machine
        || !image_token_is_valid(&inventory.machine)
    {
        report.invalid_records = 1;
        return (None, report);
    }

    normalize_deploy_directory(&mut inventory.deploy_directory, &mut report);
    let deploy_directory = inventory.deploy_directory.available().cloned();
    let mut normalized = BTreeMap::new();
    for mut artifact in inventory.artifacts {
        if artifact.identity.validate().is_err()
            || artifact.identity.machine != request.machine
            || deploy_directory
                .as_ref()
                .is_some_and(|directory| !artifact.identity.path.starts_with(directory))
        {
            report.invalid_records += 1;
            continue;
        }
        normalize_checksums(
            &mut artifact.checksums,
            deploy_directory.as_deref(),
            &mut report,
        );
        normalize_paths(
            &mut artifact.manifests,
            deploy_directory.as_deref(),
            &mut report,
        );
        normalize_paths(
            &mut artifact.licenses,
            deploy_directory.as_deref(),
            &mut report,
        );
        normalize_paths(&mut artifact.spdx, deploy_directory.as_deref(), &mut report);
        normalize_paths(
            &mut artifact.wic_files,
            deploy_directory.as_deref(),
            &mut report,
        );
        if let Some(existing) = normalized.get(&artifact.identity) {
            report.duplicate_records += 1;
            if &artifact < existing {
                normalized.insert(artifact.identity.clone(), artifact);
            }
        } else {
            normalized.insert(artifact.identity.clone(), artifact);
        }
    }
    inventory.artifacts = normalized.into_values().collect();
    if inventory.artifacts.len() > max_records {
        report.truncated_records = inventory.artifacts.len() - max_records;
        inventory.artifacts.truncate(max_records);
    }
    (Some(inventory), report)
}

fn normalize_deploy_directory(
    field: &mut ImageArtifactField<PathBuf>,
    report: &mut ImageArtifactNormalizationReport,
) {
    if matches!(field, ImageArtifactField::Available(path) if !artifact_path_is_valid(path)) {
        *field = ImageArtifactField::Unavailable;
        report.invalid_fields += 1;
    }
}

fn normalize_paths(
    field: &mut ImageArtifactField<Vec<PathBuf>>,
    deploy_directory: Option<&Path>,
    report: &mut ImageArtifactNormalizationReport,
) {
    let ImageArtifactField::Available(paths) = field else {
        return;
    };
    let before = paths.len();
    paths.retain(|path| {
        artifact_path_is_valid(path)
            && deploy_directory.is_none_or(|directory| path.starts_with(directory))
    });
    report.invalid_fields += before - paths.len();
    paths.sort();
    paths.dedup();
    if paths.len() > MAX_IMAGE_ARTIFACT_ASSOCIATED_FILES {
        report.truncated_associated_files += paths.len() - MAX_IMAGE_ARTIFACT_ASSOCIATED_FILES;
        paths.truncate(MAX_IMAGE_ARTIFACT_ASSOCIATED_FILES);
    }
}

fn normalize_checksums(
    field: &mut ImageArtifactField<Vec<ImageChecksum>>,
    deploy_directory: Option<&Path>,
    report: &mut ImageArtifactNormalizationReport,
) {
    let ImageArtifactField::Available(checksums) = field else {
        return;
    };
    let before = checksums.len();
    checksums.retain(|checksum| {
        image_token_is_valid(&checksum.algorithm)
            && checksum.digest.len() <= 4_096
            && !checksum.digest.is_empty()
            && !checksum.digest.chars().any(char::is_whitespace)
            && !checksum.digest.chars().any(char::is_control)
            && artifact_path_is_valid(&checksum.source)
            && deploy_directory.is_none_or(|directory| checksum.source.starts_with(directory))
    });
    report.invalid_fields += before - checksums.len();
    checksums.sort();
    checksums.dedup();
    if checksums.len() > MAX_IMAGE_ARTIFACT_CHECKSUMS {
        report.truncated_checksums += checksums.len() - MAX_IMAGE_ARTIFACT_CHECKSUMS;
        checksums.truncate(MAX_IMAGE_ARTIFACT_CHECKSUMS);
    }
}

fn image_token_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn artifact_path_is_valid(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && !path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImageArtifactInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: ImageArtifactRequest,
    },
    AvailableEmpty {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
    },
    Available {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
    },
    Partial {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
        limitations: Vec<String>,
    },
    Failed {
        request: ImageArtifactRequest,
        message: String,
    },
}

impl ImageArtifactInventoryState {
    pub fn request(&self) -> Option<&ImageArtifactRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request, .. }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(request),
        }
    }

    pub fn inventory(&self) -> Option<&ImageArtifactInventory> {
        match self {
            Self::AvailableEmpty { inventory, .. }
            | Self::Available { inventory, .. }
            | Self::Partial { inventory, .. } => Some(inventory),
            Self::NotLoaded | Self::Loading { .. } | Self::Failed { .. } => None,
        }
    }

    pub fn artifacts(&self) -> Option<&[ImageArtifact]> {
        self.inventory()
            .map(|inventory| inventory.artifacts.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(image: &str, path: &str) -> ImageArtifact {
        ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: image.into(),
                path: path.into(),
            },
            kind: ImageArtifactKind::RootFilesystem,
            size_bytes: ImageArtifactField::Available(1_024),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Available(Vec::new()),
            manifests: ImageArtifactField::Available(Vec::new()),
            licenses: ImageArtifactField::Available(Vec::new()),
            spdx: ImageArtifactField::Available(Vec::new()),
            wic_files: ImageArtifactField::Available(Vec::new()),
        }
    }

    #[test]
    fn image_artifact_model_normalizes_exact_identities_fields_and_bounds() {
        let request = ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        let mut preferred = artifact(
            "core-image-minimal",
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4",
        );
        preferred.manifests = ImageArtifactField::Available(vec![
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest".into(),
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest".into(),
            "/outside/core-image-minimal.manifest".into(),
        ]);
        let mut duplicate = preferred.clone();
        duplicate.size_bytes = ImageArtifactField::Available(2_048);
        let mut wrong_machine = artifact(
            "core-image-base",
            "/build/tmp/deploy/images/qemux86-64/core-image-base.ext4",
        );
        wrong_machine.identity.machine = "qemuarm64".into();
        let overflow = artifact(
            "core-image-sato",
            "/build/tmp/deploy/images/qemux86-64/core-image-sato.wic",
        );
        let inventory = ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![duplicate, overflow, wrong_machine, preferred],
        };
        let (inventory, report) = normalize_image_artifact_inventory(&request, inventory, 1);
        let inventory = inventory.unwrap();
        assert_eq!(inventory.artifacts.len(), 1);
        assert_eq!(
            inventory.artifacts[0].size_bytes,
            ImageArtifactField::Available(1_024)
        );
        assert_eq!(
            inventory.artifacts[0].manifests,
            ImageArtifactField::Available(vec![PathBuf::from(
                "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest"
            )])
        );
        assert_eq!(report.duplicate_records, 1);
        assert_eq!(report.invalid_records, 1);
        assert_eq!(report.invalid_fields, 2);
        assert_eq!(report.truncated_records, 1);
        assert!(report.is_partial());
    }

    #[test]
    fn image_artifact_model_rejects_request_inventory_identity_mismatch() {
        let request = ImageArtifactRequest {
            generation: 2,
            machine: "qemux86-64".into(),
        };
        let inventory = ImageArtifactInventory {
            machine: "qemuarm64".into(),
            deploy_directory: ImageArtifactField::Unavailable,
            artifacts: Vec::new(),
        };
        let (inventory, report) = normalize_image_artifact_inventory(&request, inventory, 10);
        assert!(inventory.is_none());
        assert_eq!(report.invalid_records, 1);
    }
}

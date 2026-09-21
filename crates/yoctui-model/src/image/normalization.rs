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

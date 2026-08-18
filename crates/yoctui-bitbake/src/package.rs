use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Notify,
};
use yoctui_model::{
    CapabilityId, DaemonCompatibilitySnapshot, MAX_PACKAGE_LIMITATIONS, MAX_PACKAGE_RECORDS,
    PackageDetail, PackageDetailRequest, PackageField, PackageIdentity, PackageInventoryRequest,
    PackageSummary, normalize_package_detail, normalize_package_summaries,
};

const MAX_PACKAGE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_OUTPUT_LINES: usize = 32_768;
const PACKAGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const PACKAGE_ARGUMENT_BATCH: usize = 128;
pub const PKGDATA_LIST_PACKAGES_IMPLEMENTATION: &str = "pkgdata.list_packages.argv";
pub const PKGDATA_PACKAGE_INFO_IMPLEMENTATION: &str = "pkgdata.package_info.argv";
pub const PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION: &str = "pkgdata.list_package_files.argv";
pub const PKGDATA_READ_VALUE_IMPLEMENTATION: &str = "pkgdata.read_value.argv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDataCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl PackageDataCommandSpec {
    pub(crate) fn new(
        executable: &Path,
        pkgdata_dir: &Path,
        subcommand: &str,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Self {
        let mut exact = vec![
            OsString::from("-p"),
            pkgdata_dir.as_os_str().to_owned(),
            OsString::from(subcommand),
        ];
        exact.extend(arguments);
        Self {
            executable: executable.to_owned(),
            arguments: exact,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInventoryResponse {
    pub request: PackageInventoryRequest,
    pub packages: Vec<PackageSummary>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDetailResponse {
    pub request: PackageDetailRequest,
    pub detail: PackageDetail,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PackageDataAdapterError {
    #[error("invalid package-data request: {0}")]
    InvalidRequest(String),
    #[error("package-data build directory is unavailable: {0}")]
    BuildDirectory(PathBuf),
    #[error("generated pkgdata is unavailable: {0}; build a target through do_package first")]
    MissingPkgdata(PathBuf),
    #[error("package-data path is invalid or is a symlink: {0}")]
    InvalidPath(PathBuf),
    #[error("package-data path escapes its configured root: {0}")]
    PathEscape(PathBuf),
    #[error("oe-pkgdata-util is missing beneath: {0}")]
    MissingTool(PathBuf),
    #[error("package-data capability {capability:?} is unavailable: {reason}")]
    CapabilityUnavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("package-data capability generation is stale: expected {expected}, got {actual}")]
    StaleCapability { expected: u64, actual: u64 },
    #[error("package-data capability snapshot belongs to another build environment")]
    CapabilityEnvironmentMismatch,
    #[error("could not start oe-pkgdata-util: {0}")]
    Spawn(String),
    #[error("oe-pkgdata-util exited with {exit_code:?}: {message}")]
    NonZero {
        exit_code: Option<i32>,
        message: String,
    },
    #[error("oe-pkgdata-util timed out after {0} seconds")]
    Timeout(u64),
    #[error("package-data operation was cancelled")]
    Cancelled,
    #[error("package-data output is malformed: {0}")]
    Malformed(String),
    #[error("package-data I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct PackageDataCancellation {
    inner: Arc<PackageDataCancellationInner>,
}

#[derive(Debug, Default)]
struct PackageDataCancellationInner {
    requested: AtomicBool,
    notify: Notify,
}

impl PackageDataCancellation {
    pub fn cancel(&self) -> bool {
        let first = !self.inner.requested.swap(true, Ordering::SeqCst);
        if first {
            self.inner.notify.notify_waiters();
        }
        first
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.requested.load(Ordering::SeqCst)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageDataAdapter {
    build_dir: PathBuf,
    tool: Option<PathBuf>,
    pkgdata_dir: Option<PathBuf>,
    timeout: Duration,
    argument_batch: usize,
    compatibility: Option<DaemonCompatibilitySnapshot>,
    expected_generation: Option<u64>,
}

impl PackageDataAdapter {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: None,
            pkgdata_dir: None,
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
            compatibility: None,
            expected_generation: None,
        }
    }

    pub fn with_paths(build_dir: PathBuf, tool: PathBuf, pkgdata_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: Some(tool),
            pkgdata_dir: Some(pkgdata_dir),
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
            compatibility: None,
            expected_generation: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_compatibility(
        mut self,
        compatibility: DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, PackageDataAdapterError> {
        let compatibility = compatibility
            .normalize()
            .map_err(|error| PackageDataAdapterError::InvalidRequest(error.to_string()))?;
        if compatibility.snapshot.generation != expected_generation {
            return Err(PackageDataAdapterError::StaleCapability {
                expected: expected_generation,
                actual: compatibility.snapshot.generation,
            });
        }
        self.compatibility = Some(compatibility);
        self.expected_generation = Some(expected_generation);
        Ok(self)
    }

    #[cfg(test)]
    fn with_argument_batch(mut self, argument_batch: usize) -> Self {
        self.argument_batch = argument_batch.max(1);
        self
    }

    pub async fn inventory(
        &self,
        request: PackageInventoryRequest,
    ) -> Result<PackageInventoryResponse, PackageDataAdapterError> {
        self.inventory_with_cancellation(request, PackageDataCancellation::default())
            .await
    }

    pub async fn inventory_with_cancellation(
        &self,
        request: PackageInventoryRequest,
        cancellation: PackageDataCancellation,
    ) -> Result<PackageInventoryResponse, PackageDataAdapterError> {
        validate_inventory_request(request)?;
        let context = self.context().await?;
        let (identities, mut limitations) = self.list_packages(&context, &cancellation).await?;
        let mut summaries = identities
            .iter()
            .cloned()
            .map(unavailable_summary)
            .collect::<BTreeMap<_, _>>();

        for chunk in identities.chunks(self.argument_batch) {
            if cancellation.is_cancelled() {
                return Err(PackageDataAdapterError::Cancelled);
            }
            let arguments = [
                vec![OsString::from("-e"), OsString::from("LICENSE")],
                chunk
                    .iter()
                    .map(|identity| OsString::from(&identity.name))
                    .collect(),
            ]
            .concat();
            let spec = context.command(
                CapabilityId::PkgDataPackageInfo,
                PKGDATA_PACKAGE_INFO_IMPLEMENTATION,
                "package-info",
                arguments,
            )?;
            let output =
                run_package_command(spec, &context.build_dir, self.timeout, &cancellation).await?;
            if output.truncated {
                push_limitation(
                    &mut limitations,
                    format!("package-info output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
                );
            }
            parse_package_info(&output.stdout, &mut summaries, &mut limitations)?;
        }

        for identity in &identities {
            if summaries
                .get(identity)
                .is_some_and(summary_has_no_information)
            {
                push_limitation(
                    &mut limitations,
                    format!("metadata was unavailable for package {}", identity.name),
                );
            }
        }
        if !identities.is_empty() {
            push_limitation(
                &mut limitations,
                "provider recipe paths are unavailable from oe-pkgdata-util package records".into(),
            );
            push_limitation(
                &mut limitations,
                "image membership is unavailable until an authoritative image manifest is selected"
                    .into(),
            );
        }
        let (packages, report) =
            normalize_package_summaries(summaries.into_values().collect(), MAX_PACKAGE_RECORDS);
        append_normalization_limitations(&mut limitations, &report);
        Ok(PackageInventoryResponse {
            request,
            packages,
            limitations,
        })
    }

    pub async fn detail(
        &self,
        request: PackageDetailRequest,
    ) -> Result<PackageDetailResponse, PackageDataAdapterError> {
        self.detail_with_cancellation(request, PackageDataCancellation::default())
            .await
    }

    pub async fn detail_with_cancellation(
        &self,
        request: PackageDetailRequest,
        cancellation: PackageDataCancellation,
    ) -> Result<PackageDetailResponse, PackageDataAdapterError> {
        validate_detail_request(&request)?;
        let context = self.context().await?;
        let mut limitations = Vec::new();
        let files_spec = context.command(
            CapabilityId::PkgDataListPackageFiles,
            PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION,
            "list-pkg-files",
            [OsString::from("-r"), OsString::from(&request.identity.name)],
        )?;
        let files_output =
            run_package_command(files_spec, &context.build_dir, self.timeout, &cancellation)
                .await?;
        if files_output.truncated {
            push_limitation(
                &mut limitations,
                format!("package file output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
            );
        }
        let files = parse_package_files(&request.identity, &files_output.stdout, &mut limitations)?;

        let (inventory, inventory_limitations) =
            self.list_packages(&context, &cancellation).await?;
        for limitation in inventory_limitations {
            push_limitation(&mut limitations, limitation);
        }
        if !inventory.contains(&request.identity) {
            return Err(PackageDataAdapterError::InvalidRequest(format!(
                "package {} is not present in the authoritative runtime inventory",
                request.identity.name
            )));
        }
        let mut dependencies = BTreeMap::new();
        for chunk in inventory.chunks(self.argument_batch) {
            let arguments = [
                vec![OsString::from("RDEPENDS"), OsString::from("-n")],
                chunk
                    .iter()
                    .map(|identity| OsString::from(&identity.name))
                    .collect(),
            ]
            .concat();
            let spec = context.command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                arguments,
            )?;
            let output =
                run_package_command(spec, &context.build_dir, self.timeout, &cancellation).await?;
            if output.truncated {
                push_limitation(
                    &mut limitations,
                    format!(
                        "runtime dependency output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"
                    ),
                );
            }
            parse_runtime_dependencies(&output.stdout, &mut dependencies, &mut limitations)?;
        }
        let runtime_dependencies = dependencies
            .get(&request.identity)
            .cloned()
            .unwrap_or_default();
        if !dependencies.contains_key(&request.identity) {
            push_limitation(
                &mut limitations,
                format!(
                    "runtime dependency data was unavailable for {}",
                    request.identity.name
                ),
            );
        }
        let reverse_dependencies = dependencies
            .iter()
            .filter_map(|(identity, values)| {
                values
                    .contains(&request.identity)
                    .then_some(identity.clone())
            })
            .collect();
        let detail = PackageDetail {
            identity: request.identity.clone(),
            files: PackageField::Available(files),
            runtime_dependencies: if dependencies.contains_key(&request.identity) {
                PackageField::Available(runtime_dependencies)
            } else {
                PackageField::Unavailable
            },
            reverse_dependencies: PackageField::Available(reverse_dependencies),
        };
        let (detail, report) = normalize_package_detail(&request.identity, detail);
        append_normalization_limitations(&mut limitations, &report);
        let detail = detail.ok_or_else(|| {
            PackageDataAdapterError::Malformed(
                "normalized package detail did not match the request".into(),
            )
        })?;
        Ok(PackageDetailResponse {
            request,
            detail,
            limitations,
        })
    }

    async fn context(&self) -> Result<PackageDataContext, PackageDataAdapterError> {
        let build_dir = canonical_directory(&self.build_dir)
            .await
            .map_err(|_| PackageDataAdapterError::BuildDirectory(self.build_dir.clone()))?;
        let compatibility = self.compatibility.clone().ok_or_else(|| {
            PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataGenerated,
                reason: "the current environment capability snapshot is unavailable".into(),
            }
        })?;
        if self.expected_generation != Some(compatibility.snapshot.generation) {
            return Err(PackageDataAdapterError::StaleCapability {
                expected: self.expected_generation.unwrap_or_default(),
                actual: compatibility.snapshot.generation,
            });
        }
        if compatibility
            .snapshot
            .environment
            .build_directory
            .value()
            .map(PathBuf::as_path)
            != Some(build_dir.as_path())
        {
            return Err(PackageDataAdapterError::CapabilityEnvironmentMismatch);
        }
        let generated = compatibility
            .snapshot
            .capability(CapabilityId::PkgDataGenerated);
        if !generated.is_some_and(|record| record.state.is_enabled()) {
            return Err(PackageDataAdapterError::MissingPkgdata(
                self.pkgdata_dir
                    .clone()
                    .unwrap_or_else(|| build_dir.join("tmp/pkgdata")),
            ));
        }
        let pkgdata_path = self
            .pkgdata_dir
            .clone()
            .unwrap_or_else(|| build_dir.join("tmp/pkgdata"));
        let pkgdata_dir = canonical_directory(&pkgdata_path)
            .await
            .map_err(|_| PackageDataAdapterError::MissingPkgdata(pkgdata_path.clone()))?;
        if self.pkgdata_dir.is_none() && !pkgdata_dir.starts_with(&build_dir) {
            return Err(PackageDataAdapterError::PathEscape(pkgdata_dir));
        }
        let detected_tool = compatibility
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "oe-pkgdata-util"))
            .map(|tool| tool.executable.clone())
            .ok_or_else(|| PackageDataAdapterError::MissingTool(build_dir.clone()))?;
        if self
            .tool
            .as_ref()
            .is_some_and(|tool| tool != &detected_tool)
        {
            return Err(PackageDataAdapterError::CapabilityEnvironmentMismatch);
        }
        if !detected_tool.exists() {
            return Err(PackageDataAdapterError::MissingTool(detected_tool));
        }
        let tool = canonical_regular_file(&detected_tool).await?;
        Ok(PackageDataContext {
            build_dir,
            pkgdata_dir,
            tool,
            compatibility,
        })
    }

    async fn list_packages(
        &self,
        context: &PackageDataContext,
        cancellation: &PackageDataCancellation,
    ) -> Result<(Vec<PackageIdentity>, Vec<String>), PackageDataAdapterError> {
        let spec = context.command(
            CapabilityId::PkgDataListPackages,
            PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
            "list-pkgs",
            [OsString::from("-r")],
        )?;
        let output =
            match run_package_command(spec, &context.build_dir, self.timeout, cancellation).await {
                Ok(output) => output,
                Err(PackageDataAdapterError::NonZero { message, .. })
                    if message.contains("No packages found") =>
                {
                    return Ok((Vec::new(), Vec::new()));
                }
                Err(error) => return Err(error),
            };
        let mut limitations = Vec::new();
        if output.truncated {
            push_limitation(
                &mut limitations,
                format!("package inventory output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
            );
        }
        let identities = parse_package_list(&output.stdout, &mut limitations)?;
        Ok((identities, limitations))
    }
}

#[derive(Debug)]
struct PackageDataContext {
    build_dir: PathBuf,
    pkgdata_dir: PathBuf,
    tool: PathBuf,
    compatibility: DaemonCompatibilitySnapshot,
}

impl PackageDataContext {
    fn command(
        &self,
        capability: CapabilityId,
        implementation: &str,
        subcommand: &str,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Result<PackageDataCommandSpec, PackageDataAdapterError> {
        let record = self
            .compatibility
            .snapshot
            .capability(capability)
            .ok_or_else(|| PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: "the capability record is missing".into(),
            })?;
        if !record.state.is_enabled() {
            return Err(PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| "no positive capability evidence is available".into()),
            });
        }
        let selected = self
            .compatibility
            .implementations
            .get(&capability)
            .ok_or_else(|| PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: "no compatible implementation was selected".into(),
            })?;
        if selected.id != implementation {
            return Err(PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: format!("selected implementation {} is incompatible", selected.id),
            });
        }
        Ok(PackageDataCommandSpec::new(
            &self.tool,
            &self.pkgdata_dir,
            subcommand,
            arguments,
        ))
    }
}

fn validate_inventory_request(
    request: PackageInventoryRequest,
) -> Result<(), PackageDataAdapterError> {
    if request.generation == 0 {
        Err(PackageDataAdapterError::InvalidRequest(
            "inventory generation must be nonzero".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_detail_request(request: &PackageDetailRequest) -> Result<(), PackageDataAdapterError> {
    if request.generation == 0 {
        return Err(PackageDataAdapterError::InvalidRequest(
            "detail generation must be nonzero".into(),
        ));
    }
    request
        .identity
        .validate()
        .map_err(|message| PackageDataAdapterError::InvalidRequest(message.into()))
}

fn unavailable_summary(identity: PackageIdentity) -> (PackageIdentity, PackageSummary) {
    (
        identity.clone(),
        PackageSummary {
            identity,
            recipe: PackageField::Unavailable,
            provider: PackageField::Unavailable,
            version: PackageField::Unavailable,
            installed_size_bytes: PackageField::Unavailable,
            license: PackageField::Unavailable,
            image_membership: PackageField::Unavailable,
        },
    )
}

fn summary_has_no_information(summary: &PackageSummary) -> bool {
    matches!(summary.recipe, PackageField::Unavailable)
        && matches!(summary.version, PackageField::Unavailable)
        && matches!(summary.installed_size_bytes, PackageField::Unavailable)
        && matches!(summary.license, PackageField::Unavailable)
}

fn parse_package_list(
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<PackageIdentity>, PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PackageDataAdapterError::Malformed("inventory is not UTF-8".into()))?;
    let mut identities = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        if index >= MAX_PACKAGE_OUTPUT_LINES {
            push_limitation(
                limitations,
                format!("package inventory was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
            );
            break;
        }
        let identity = PackageIdentity::new(line.trim());
        if identity.validate().is_err() {
            push_limitation(
                limitations,
                format!(
                    "invalid package inventory record was omitted at line {}",
                    index + 1
                ),
            );
            continue;
        }
        identities.insert(identity);
        if identities.len() == MAX_PACKAGE_RECORDS {
            if text
                .lines()
                .skip(index + 1)
                .any(|line| !line.trim().is_empty())
            {
                push_limitation(
                    limitations,
                    format!("package inventory was limited to {MAX_PACKAGE_RECORDS} records"),
                );
            }
            break;
        }
    }
    Ok(identities.into_iter().collect())
}

fn parse_package_info(
    bytes: &[u8],
    summaries: &mut BTreeMap<PackageIdentity, PackageSummary>,
    limitations: &mut Vec<String>,
) -> Result<(), PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        PackageDataAdapterError::Malformed("package information is not UTF-8".into())
    })?;
    for (index, line) in text.lines().take(MAX_PACKAGE_OUTPUT_LINES).enumerate() {
        let Some((fields, remainder)) = split_prefix_tokens(line, 5) else {
            push_limitation(
                limitations,
                format!(
                    "malformed package-info record was omitted at line {}",
                    index + 1
                ),
            );
            continue;
        };
        let identity = PackageIdentity::new(fields[0]);
        let Some(summary) = summaries.get_mut(&identity) else {
            push_limitation(
                limitations,
                format!(
                    "unexpected package-info identity {} was omitted",
                    identity.name
                ),
            );
            continue;
        };
        summary.version = nonempty_field(fields[1]);
        summary.recipe = nonempty_field(fields[2]);
        summary.installed_size_bytes = fields[4]
            .parse::<u64>()
            .map(PackageField::Available)
            .unwrap_or(PackageField::Unavailable);
        if matches!(summary.installed_size_bytes, PackageField::Unavailable) {
            push_limitation(
                limitations,
                format!("installed size was unavailable for {}", identity.name),
            );
        }
        let license = remainder.trim();
        let license = license
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(license);
        summary.license = nonempty_field(license);
    }
    if text.lines().count() > MAX_PACKAGE_OUTPUT_LINES {
        push_limitation(
            limitations,
            format!("package information was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
        );
    }
    Ok(())
}

fn split_prefix_tokens(line: &str, count: usize) -> Option<(Vec<&str>, &str)> {
    let mut tokens = Vec::with_capacity(count);
    let mut cursor = 0;
    while tokens.len() < count {
        cursor += line[cursor..].find(|character: char| !character.is_whitespace())?;
        let end = line[cursor..]
            .find(char::is_whitespace)
            .map_or(line.len(), |offset| cursor + offset);
        tokens.push(&line[cursor..end]);
        cursor = end;
    }
    Some((tokens, line[cursor..].trim_start()))
}

fn nonempty_field(value: &str) -> PackageField<String> {
    if value.is_empty() {
        PackageField::Unavailable
    } else {
        PackageField::Available(value.into())
    }
}

fn parse_package_files(
    identity: &PackageIdentity,
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<PathBuf>, PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PackageDataAdapterError::Malformed("package file list is not UTF-8".into()))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some(format!("{}:", identity.name).as_str()) {
        return Err(PackageDataAdapterError::Malformed(format!(
            "file list identity does not match {}",
            identity.name
        )));
    }
    let mut files = Vec::new();
    for (index, line) in lines.enumerate() {
        if index >= MAX_PACKAGE_OUTPUT_LINES {
            push_limitation(
                limitations,
                format!("package file list was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
            );
            break;
        }
        let path = PathBuf::from(line.trim());
        if !path.is_absolute() || path == Path::new("/") {
            push_limitation(
                limitations,
                format!(
                    "invalid package file path was omitted at line {}",
                    index + 2
                ),
            );
            continue;
        }
        files.push(path);
    }
    Ok(files)
}

fn parse_runtime_dependencies(
    bytes: &[u8],
    dependencies: &mut BTreeMap<PackageIdentity, Vec<PackageIdentity>>,
    limitations: &mut Vec<String>,
) -> Result<(), PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        PackageDataAdapterError::Malformed("runtime dependencies are not UTF-8".into())
    })?;
    for (index, line) in text.lines().take(MAX_PACKAGE_OUTPUT_LINES).enumerate() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let identity = PackageIdentity::new(name);
        if identity.validate().is_err() {
            push_limitation(
                limitations,
                format!("invalid dependency owner was omitted at line {}", index + 1),
            );
            continue;
        }
        let mut values = BTreeSet::new();
        let mut in_constraint = false;
        for token in fields {
            if token.starts_with('(') {
                in_constraint = !token.ends_with(')');
                continue;
            }
            if in_constraint {
                in_constraint = !token.ends_with(')');
                continue;
            }
            if token == "|" || token.contains(['<', '>', '=']) {
                continue;
            }
            let dependency = PackageIdentity::new(token);
            if dependency.validate().is_ok() {
                values.insert(dependency);
            } else {
                push_limitation(
                    limitations,
                    format!(
                        "invalid runtime dependency was omitted at line {}",
                        index + 1
                    ),
                );
            }
        }
        dependencies.insert(identity, values.into_iter().collect());
    }
    if text.lines().count() > MAX_PACKAGE_OUTPUT_LINES {
        push_limitation(
            limitations,
            format!("runtime dependencies were limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
        );
    }
    Ok(())
}

fn append_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &yoctui_model::PackageNormalizationReport,
) {
    if report.invalid_records > 0 {
        push_limitation(
            limitations,
            format!(
                "{} invalid package records were omitted",
                report.invalid_records
            ),
        );
    }
    if report.invalid_fields > 0 {
        push_limitation(
            limitations,
            format!(
                "{} invalid package fields were unavailable",
                report.invalid_fields
            ),
        );
    }
    if report.truncated_records > 0
        || report.truncated_files > 0
        || report.truncated_dependencies > 0
        || report.truncated_image_memberships > 0
    {
        push_limitation(
            limitations,
            "one or more package data collections reached their hard limit".into(),
        );
    }
}

async fn canonical_directory(path: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    if !path.is_absolute() {
        return Err(PackageDataAdapterError::PathEscape(path.to_owned()));
    }
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| PackageDataAdapterError::InvalidPath(path.to_owned()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageDataAdapterError::InvalidPath(path.to_owned()));
    }
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))
}

async fn canonical_regular_file(path: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    if !path.is_absolute() {
        return Err(PackageDataAdapterError::PathEscape(path.to_owned()));
    }
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| PackageDataAdapterError::MissingTool(path.to_owned()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PackageDataAdapterError::InvalidPath(path.to_owned()));
    }
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))
}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

struct CommandOutput {
    stdout: Vec<u8>,
    truncated: bool,
}

async fn read_bounded<R>(mut reader: R) -> Result<BoundedOutput, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = MAX_PACKAGE_OUTPUT_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok(BoundedOutput { bytes, truncated })
}

async fn run_package_command(
    spec: PackageDataCommandSpec,
    build_dir: &Path,
    timeout: Duration,
    cancellation: &PackageDataCancellation,
) -> Result<CommandOutput, PackageDataAdapterError> {
    if cancellation.is_cancelled() {
        return Err(PackageDataAdapterError::Cancelled);
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.arguments)
        .current_dir(build_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            PackageDataAdapterError::MissingTool(spec.executable.clone())
        } else {
            PackageDataAdapterError::Spawn(error.to_string())
        }
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PackageDataAdapterError::Spawn("stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PackageDataAdapterError::Spawn("stderr is unavailable".into()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    let terminal = tokio::select! {
        status = child.wait() => status.map_err(|error| PackageDataAdapterError::Io(error.to_string())),
        _ = cancellation.cancelled() => {
            terminate_package_child(&mut child).await;
            Err(PackageDataAdapterError::Cancelled)
        }
        _ = tokio::time::sleep(timeout) => {
            terminate_package_child(&mut child).await;
            Err(PackageDataAdapterError::Timeout(timeout.as_secs()))
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
    let stderr = stderr_task
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
    let status = terminal?;
    if !status.success() {
        return Err(PackageDataAdapterError::NonZero {
            exit_code: status.code(),
            message: bounded_error_message(&stderr.bytes, &stdout.bytes),
        });
    }
    Ok(CommandOutput {
        stdout: stdout.bytes,
        truncated: stdout.truncated || stderr.truncated,
    })
}

async fn terminate_package_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(id) = child.id() {
        // SAFETY: the process group is the child PID created by `process_group(0)`.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGTERM) };
        if tokio::time::timeout(Duration::from_millis(500), child.wait())
            .await
            .is_ok()
        {
            return;
        }
        // SAFETY: same child-owned process group as the graceful signal above.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGKILL) };
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn bounded_error_message(stderr: &[u8], stdout: &[u8]) -> String {
    let bytes = if stderr.is_empty() { stdout } else { stderr };
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .next()
        .unwrap_or("no diagnostic output")
        .chars()
        .take(512)
        .collect()
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitations.len() < MAX_PACKAGE_LIMITATIONS && !limitations.contains(&limitation) {
        limitations.push(limitation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompatibilityFixtureRole, release_capability_fixtures};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
    };

    fn compatibility(build: &Path, tool: &Path) -> DaemonCompatibilitySnapshot {
        let capabilities = [
            (
                CapabilityId::PkgDataGenerated,
                "pkgdata.generated",
                CapabilityImplementationKind::ProcessAdapter,
            ),
            (
                CapabilityId::PkgDataListPackages,
                PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
                CapabilityImplementationKind::Command,
            ),
            (
                CapabilityId::PkgDataPackageInfo,
                PKGDATA_PACKAGE_INFO_IMPLEMENTATION,
                CapabilityImplementationKind::Command,
            ),
            (
                CapabilityId::PkgDataListPackageFiles,
                PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION,
                CapabilityImplementationKind::Command,
            ),
            (
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                CapabilityImplementationKind::Command,
            ),
        ];
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "oe-pkgdata-util".into(),
                            executable: tool.to_owned(),
                            version: None,
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .iter()
                    .map(|(id, _, _)| CapabilityRecord {
                        id: *id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: format!("{} fixture probe", id.as_str()),
                            detail: "The fixture exposes this exact package-data behavior.".into(),
                            argv: vec![tool.display().to_string(), "--help".into()],
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|(id, implementation, kind)| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yoctui-pkgdata-{name}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture(name: &str, script: &str) -> (TestDirectory, PackageDataAdapter, PathBuf) {
        let directory = TestDirectory::new(name);
        let build_dir = directory.path().join("build");
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        fs::create_dir_all(&pkgdata_dir).unwrap();
        let tool = directory.path().join("oe-pkgdata-util");
        fs::write(&tool, script).unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&tool).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool, permissions).unwrap();
        }
        let authority = compatibility(&build_dir, &tool);
        let adapter = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
            .with_compatibility(authority, 1)
            .unwrap();
        let log = directory.path().join("arguments.log");
        (directory, adapter, log)
    }

    fn inventory_request() -> PackageInventoryRequest {
        PackageInventoryRequest { generation: 1 }
    }

    fn detail_request() -> PackageDetailRequest {
        PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 2,
        }
    }

    #[test]
    fn compatibility_command_shared_fixture_builds_pkgdata_argv_without_spawning() {
        let mut authority = release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == CompatibilityFixtureRole::LatestSupportCandidate)
            .unwrap()
            .command_authority(39);
        let build_dir = authority
            .snapshot
            .environment
            .build_directory
            .value()
            .unwrap()
            .clone();
        let tool = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .unwrap()
            .iter()
            .find(|tool| tool.id == "oe-pkgdata-util")
            .unwrap()
            .executable
            .clone();
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        let context = PackageDataContext {
            build_dir,
            pkgdata_dir: pkgdata_dir.clone(),
            tool: tool.clone(),
            compatibility: authority.clone(),
        };

        let list = context
            .command(
                CapabilityId::PkgDataListPackages,
                PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
                "list-pkgs",
                [OsString::from("-r")],
            )
            .unwrap();
        assert_eq!(list.executable(), tool);
        assert_eq!(
            list.arguments(),
            [
                OsString::from("-p"),
                pkgdata_dir.as_os_str().to_owned(),
                OsString::from("list-pkgs"),
                OsString::from("-r"),
            ]
        );

        let read = context
            .command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                [OsString::from("RDEPENDS"), OsString::from("-n")],
            )
            .unwrap();
        assert_eq!(
            read.arguments(),
            [
                OsString::from("-p"),
                pkgdata_dir.as_os_str().to_owned(),
                OsString::from("read-value"),
                OsString::from("RDEPENDS"),
                OsString::from("-n"),
            ]
        );

        let record = authority
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == CapabilityId::PkgDataReadValue)
            .unwrap();
        record.state = CapabilityState::Unavailable {
            reason: yoctui_model::CapabilityReason::new(
                "fixture.command_absent",
                "The fixture does not expose read-value.",
                Some("Required command: read-value".into()),
            )
            .unwrap(),
        };
        record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
        authority
            .implementations
            .remove(&CapabilityId::PkgDataReadValue);
        let unavailable = PackageDataContext {
            build_dir: context.build_dir,
            pkgdata_dir: context.pkgdata_dir,
            tool: context.tool,
            compatibility: authority.normalize().unwrap(),
        };
        assert!(matches!(
            unavailable.command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                [OsString::from("RDEPENDS")],
            ),
            Err(PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataReadValue,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_builds_exact_authorized_commands_and_parses_results() {
        let script = r#"#!/bin/sh
log="$(dirname "$0")/arguments.log"
printf '%s\n' "--" "$@" >> "$log"
case "$3" in
  list-pkgs)
    printf 'libc6\nbusybox\ninit\n'
    ;;
  package-info)
    printf 'busybox 1.37.0-r0 busybox 1.37.0-r0 1024 "GPL-2.0-only"\n'
    printf 'init 1.0-r0 init 1.0-r0 64 "MIT"\n'
    printf 'libc6 2.40-r0 glibc 2.40-r0 4096 "GPL-2.0-or-later"\n'
    ;;
  list-pkg-files)
    printf 'busybox:\n\t/bin/busybox\n\t/etc/busybox.conf\n'
    ;;
  read-value)
    printf 'busybox libc6 (>= 2.40)\n'
    printf 'init busybox\n'
    printf 'libc6\n'
    ;;
  *)
    printf 'unexpected subcommand\n' >&2
    exit 9
    ;;
esac
"#;
        let (_directory, adapter, log) = fixture("typed", script);
        let inventory = adapter
            .clone()
            .with_argument_batch(8)
            .inventory(inventory_request())
            .await
            .unwrap();
        assert_eq!(
            inventory
                .packages
                .iter()
                .map(|package| package.identity.name.as_str())
                .collect::<Vec<_>>(),
            vec!["busybox", "init", "libc6"]
        );
        let busybox = &inventory.packages[0];
        assert_eq!(busybox.recipe, PackageField::Available("busybox".into()));
        assert_eq!(busybox.version, PackageField::Available("1.37.0-r0".into()));
        assert_eq!(busybox.installed_size_bytes, PackageField::Available(1_024));
        assert_eq!(
            busybox.license,
            PackageField::Available("GPL-2.0-only".into())
        );
        assert_eq!(busybox.provider, PackageField::Unavailable);
        assert!(
            inventory
                .limitations
                .iter()
                .any(|limitation| limitation.contains("provider recipe paths"))
        );

        let detail = adapter
            .with_argument_batch(2)
            .detail(detail_request())
            .await
            .unwrap();
        assert_eq!(
            detail.detail.files,
            PackageField::Available(vec![
                PathBuf::from("/bin/busybox"),
                PathBuf::from("/etc/busybox.conf"),
            ])
        );
        assert_eq!(
            detail.detail.runtime_dependencies,
            PackageField::Available(vec![PackageIdentity::new("libc6")])
        );
        assert_eq!(
            detail.detail.reverse_dependencies,
            PackageField::Available(vec![PackageIdentity::new("init")])
        );

        let arguments = fs::read_to_string(log).unwrap();
        assert!(arguments.contains("\n-p\n"));
        assert!(arguments.contains("\nlist-pkgs\n-r\n"));
        assert!(arguments.contains("\npackage-info\n-e\nLICENSE\nbusybox\n"));
        assert!(arguments.contains("\nlist-pkg-files\n-r\nbusybox\n"));
        assert!(arguments.contains("\nread-value\nRDEPENDS\n-n\n"));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_uses_detected_tool_and_preserves_valid_empty_inventory() {
        let directory = TestDirectory::new("discover");
        let build_dir = directory.path().join("build");
        fs::create_dir_all(build_dir.join("tmp/pkgdata")).unwrap();
        let scripts = directory.path().join("layers/openembedded-core/scripts");
        fs::create_dir_all(&scripts).unwrap();
        let tool = scripts.join("oe-pkgdata-util");
        fs::write(
            &tool,
            "#!/bin/sh\nprintf 'No packages found\\n' >&2\nexit 1\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&tool).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool, permissions).unwrap();
        }
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        let authority = compatibility(&build_dir, &tool);
        let response = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap();
        assert!(response.packages.is_empty());
        assert!(response.limitations.is_empty());
    }

    #[test]
    fn pkgdata_adapter_parsers_bound_and_reject_untrusted_records() {
        let mut limitations = Vec::new();
        let mut input = String::new();
        for index in 0..=MAX_PACKAGE_RECORDS {
            input.push_str(&format!("package-{index}\n"));
        }
        input.push_str("bad package\n");
        let identities = parse_package_list(input.as_bytes(), &mut limitations).unwrap();
        assert_eq!(identities.len(), MAX_PACKAGE_RECORDS);
        assert!(
            limitations
                .iter()
                .any(|limitation| limitation.contains("limited"))
        );

        let mut summaries = BTreeMap::from([unavailable_summary(PackageIdentity::new("busybox"))]);
        parse_package_info(
            b"other 1.0 other 1.0 5 \"MIT\"\nmalformed\nbusybox 1.0 busybox 1.0 nope \"MIT\"\n",
            &mut summaries,
            &mut limitations,
        )
        .unwrap();
        assert_eq!(
            summaries[&PackageIdentity::new("busybox")].installed_size_bytes,
            PackageField::Unavailable
        );
        assert!(
            limitations
                .iter()
                .any(|limitation| limitation.contains("unexpected package-info identity"))
        );

        assert!(matches!(
            parse_package_files(
                &PackageIdentity::new("busybox"),
                b"wrong:\n\t/bin/value\n",
                &mut limitations
            ),
            Err(PackageDataAdapterError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_distinguishes_missing_generated_data_and_command_failure() {
        let directory = TestDirectory::new("failures");
        let build_dir = directory.path().join("build");
        fs::create_dir_all(&build_dir).unwrap();
        let tool = directory.path().join("oe-pkgdata-util");
        fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let authority = compatibility(&build_dir, &tool);
        let missing =
            PackageDataAdapter::with_paths(build_dir.clone(), tool, build_dir.join("tmp/pkgdata"))
                .with_compatibility(authority, 1)
                .unwrap()
                .inventory(inventory_request())
                .await
                .unwrap_err();
        assert!(matches!(
            missing,
            PackageDataAdapterError::MissingPkgdata(_)
        ));

        let script = "#!/bin/sh\nprintf 'broken metadata\\n' >&2\nexit 17\n";
        let (_fixture, adapter, _log) = fixture("nonzero", script);
        assert_eq!(
            adapter.inventory(inventory_request()).await.unwrap_err(),
            PackageDataAdapterError::NonZero {
                exit_code: Some(17),
                message: "broken metadata".into(),
            }
        );
        assert!(matches!(
            adapter
                .inventory(PackageInventoryRequest { generation: 0 })
                .await,
            Err(PackageDataAdapterError::InvalidRequest(_))
        ));
        assert!(matches!(
            adapter
                .detail(PackageDetailRequest {
                    identity: PackageIdentity::new("bad package"),
                    generation: 1,
                })
                .await,
            Err(PackageDataAdapterError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_rejects_missing_command_stale_snapshot_and_zero_spawn() {
        let directory = TestDirectory::new("capability-reject");
        let build_dir = directory.path().join("build");
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        fs::create_dir_all(&pkgdata_dir).unwrap();
        let tool = directory.path().join("oe-pkgdata-util");
        let marker = directory.path().join("spawned");
        fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let mut authority = compatibility(&build_dir, &tool);
        let record = authority
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == CapabilityId::PkgDataListPackages)
            .unwrap();
        record.state = CapabilityState::Unavailable {
            reason: yoctui_model::CapabilityReason::new(
                "pkgdata.command_missing",
                "Current oe-pkgdata-util does not expose list-pkgs.",
                Some("Required command: list-pkgs".into()),
            )
            .unwrap(),
        };
        record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
        authority
            .implementations
            .remove(&CapabilityId::PkgDataListPackages);
        let authority = authority.normalize().unwrap();
        assert!(matches!(
            PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), pkgdata_dir.clone())
                .with_compatibility(authority.clone(), 2),
            Err(PackageDataAdapterError::StaleCapability { .. })
        ));
        let error = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataListPackages,
                reason,
            } if reason.contains("does not expose list-pkgs")
        ));
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pkgdata_adapter_rejects_symlinked_tool_and_pkgdata_paths() {
        let directory = TestDirectory::new("symlinks");
        let build_dir = directory.path().join("build");
        let real_pkgdata = directory.path().join("real-pkgdata");
        fs::create_dir_all(&build_dir).unwrap();
        fs::create_dir_all(&real_pkgdata).unwrap();
        let linked_pkgdata = build_dir.join("pkgdata");
        symlink(&real_pkgdata, &linked_pkgdata).unwrap();
        let tool = directory.path().join("tool");
        fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = fs::metadata(&tool).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tool, permissions).unwrap();
        let error = PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), linked_pkgdata)
            .with_compatibility(compatibility(&build_dir, &tool), 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::MissingPkgdata(_)));

        let linked_tool = directory.path().join("linked-tool");
        symlink(&tool, &linked_tool).unwrap();
        let authority = compatibility(&build_dir, &linked_tool);
        let error = PackageDataAdapter::with_paths(build_dir, linked_tool, real_pkgdata)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::InvalidPath(_)));
    }

    #[tokio::test]
    async fn pkgdata_adapter_times_out_and_cancels_process_groups() {
        let script = "#!/bin/sh\nsleep 5\nprintf 'busybox\\n'\n";
        let (_timeout_directory, timeout_adapter, _log) = fixture("timeout", script);
        let error = timeout_adapter
            .with_timeout(Duration::from_millis(20))
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::Timeout(_)));

        let (_cancel_directory, cancel_adapter, _log) = fixture("cancel", script);
        let cancellation = PackageDataCancellation::default();
        let task_cancellation = cancellation.clone();
        let task = tokio::spawn(async move {
            cancel_adapter
                .inventory_with_cancellation(inventory_request(), task_cancellation)
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(cancellation.cancel());
        assert_eq!(
            task.await.unwrap().unwrap_err(),
            PackageDataAdapterError::Cancelled
        );
    }
}

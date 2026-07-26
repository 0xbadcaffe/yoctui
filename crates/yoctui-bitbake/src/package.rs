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
    MAX_PACKAGE_LIMITATIONS, MAX_PACKAGE_RECORDS, PackageDetail, PackageDetailRequest,
    PackageField, PackageIdentity, PackageInventoryRequest, PackageSummary,
    normalize_package_detail, normalize_package_summaries,
};

const MAX_PACKAGE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_OUTPUT_LINES: usize = 32_768;
const MAX_TOOL_SCAN_ENTRIES: usize = 100_000;
const PACKAGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const PACKAGE_ARGUMENT_BATCH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDataCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl PackageDataCommandSpec {
    fn new(
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
}

impl PackageDataAdapter {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: None,
            pkgdata_dir: None,
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
        }
    }

    pub fn with_paths(build_dir: PathBuf, tool: PathBuf, pkgdata_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: Some(tool),
            pkgdata_dir: Some(pkgdata_dir),
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
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
            let spec = PackageDataCommandSpec::new(
                &context.tool,
                &context.pkgdata_dir,
                "package-info",
                arguments,
            );
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
        let files_spec = PackageDataCommandSpec::new(
            &context.tool,
            &context.pkgdata_dir,
            "list-pkg-files",
            [OsString::from("-r"), OsString::from(&request.identity.name)],
        );
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
            let spec = PackageDataCommandSpec::new(
                &context.tool,
                &context.pkgdata_dir,
                "read-value",
                arguments,
            );
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
        let tool = if let Some(tool) = &self.tool {
            canonical_regular_file(tool).await?
        } else {
            discover_tool(&build_dir).await?
        };
        Ok(PackageDataContext {
            build_dir,
            pkgdata_dir,
            tool,
        })
    }

    async fn list_packages(
        &self,
        context: &PackageDataContext,
        cancellation: &PackageDataCancellation,
    ) -> Result<(Vec<PackageIdentity>, Vec<String>), PackageDataAdapterError> {
        let spec = PackageDataCommandSpec::new(
            &context.tool,
            &context.pkgdata_dir,
            "list-pkgs",
            [OsString::from("-r")],
        );
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

async fn discover_tool(build_dir: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    let root = build_dir
        .parent()
        .ok_or_else(|| PackageDataAdapterError::MissingTool(build_dir.to_owned()))?
        .to_owned();
    let scan_root = root.clone();
    let candidate = tokio::task::spawn_blocking(move || discover_tool_sync(&scan_root))
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))??;
    canonical_regular_file(&candidate).await
}

fn discover_tool_sync(root: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    let mut directories = vec![root.to_owned()];
    let mut candidates = Vec::new();
    let mut visited = 0usize;
    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
            visited += 1;
            if visited > MAX_TOOL_SCAN_ENTRIES {
                return Err(PackageDataAdapterError::MissingTool(root.to_owned()));
            }
            let file_type = entry
                .file_type()
                .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file()
                && path.file_name().and_then(|name| name.to_str()) == Some("oe-pkgdata-util")
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    == Some("scripts")
            {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| PackageDataAdapterError::MissingTool(root.to_owned()))
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
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

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
        let adapter = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir);
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

    #[tokio::test]
    async fn pkgdata_adapter_builds_exact_commands_and_parses_typed_inventory_and_detail() {
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
    async fn pkgdata_adapter_discovers_authoritative_tool_and_preserves_valid_empty_inventory() {
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
        let response = PackageDataAdapter::new(build_dir)
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
    async fn pkgdata_adapter_reports_missing_invalid_nonzero_and_request_failures() {
        let directory = TestDirectory::new("failures");
        let build_dir = directory.path().join("build");
        fs::create_dir_all(&build_dir).unwrap();
        let missing = PackageDataAdapter::new(build_dir.clone())
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
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::MissingPkgdata(_)));

        let linked_tool = directory.path().join("linked-tool");
        symlink(&tool, &linked_tool).unwrap();
        let error = PackageDataAdapter::with_paths(build_dir, linked_tool, real_pkgdata)
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

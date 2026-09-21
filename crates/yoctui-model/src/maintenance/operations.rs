#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstateReadinessMode {
    IsolatedTmpdir,
    SameTmpdir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateReadinessRequest {
    pub targets: Vec<String>,
    pub mode: SstateReadinessMode,
    pub output: Option<PathBuf>,
    pub log: Option<PathBuf>,
    pub timeout_seconds: u64,
}

impl SstateReadinessRequest {
    pub fn new(
        mut targets: Vec<String>,
        mode: SstateReadinessMode,
        output: Option<PathBuf>,
        log: Option<PathBuf>,
        timeout_seconds: u64,
    ) -> Result<Self, &'static str> {
        targets.retain(|target| bounded_token(target));
        targets.sort();
        targets.dedup();
        targets.truncate(MAX_MAINTENANCE_TARGETS);
        if targets.is_empty()
            || timeout_seconds == 0
            || [output.as_ref(), log.as_ref()]
                .into_iter()
                .flatten()
                .any(|path| !absolute_normal_path(path))
        {
            return Err("sstate readiness request is invalid");
        }
        Ok(Self {
            targets,
            mode,
            output,
            log,
            timeout_seconds,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstateCleanupMode {
    Duplicates,
    Orphans,
    UnreferencedByStamps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateCleanupRequest {
    pub cache_dir: PathBuf,
    pub stamps_dirs: Vec<PathBuf>,
    pub modes: Vec<SstateCleanupMode>,
    pub jobs: u16,
}

impl SstateCleanupRequest {
    pub fn new(
        cache_dir: PathBuf,
        stamps_dirs: Vec<PathBuf>,
        mut modes: Vec<SstateCleanupMode>,
        jobs: u16,
    ) -> Result<Self, &'static str> {
        let stamps_dirs = normalize_paths(stamps_dirs);
        modes.sort_by_key(|mode| match mode {
            SstateCleanupMode::Duplicates => 0,
            SstateCleanupMode::Orphans => 1,
            SstateCleanupMode::UnreferencedByStamps => 2,
        });
        modes.dedup();
        if !absolute_normal_path(&cache_dir)
            || modes.is_empty()
            || jobs == 0
            || (modes.contains(&SstateCleanupMode::UnreferencedByStamps) && stamps_dirs.is_empty())
        {
            return Err("sstate cleanup request is invalid");
        }
        Ok(Self {
            cache_dir,
            stamps_dirs,
            modes,
            jobs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateCleanupPreview {
    pub request: SstateCleanupRequest,
    pub candidates: Vec<MaintenanceFileIdentity>,
}

impl SstateCleanupPreview {
    pub fn new(
        request: SstateCleanupRequest,
        mut candidates: Vec<MaintenanceFileIdentity>,
    ) -> Result<Self, &'static str> {
        if candidates.iter().any(|candidate| {
            !candidate.is_valid() || !candidate.path.starts_with(&request.cache_dir)
        }) {
            return Err("sstate cleanup candidate escapes the cache root");
        }
        candidates.sort_by(|left, right| left.path.cmp(&right.path));
        candidates.dedup_by(|left, right| left.path == right.path);
        candidates.truncate(MAX_MAINTENANCE_PATHS);
        Ok(Self {
            request,
            candidates,
        })
    }

    pub fn required_phrase(&self) -> String {
        format!(
            "DELETE {} FROM {}",
            self.candidates.len(),
            self.request.cache_dir.display()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrServiceOperation {
    Export,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrServiceRequest {
    pub operation: PrServiceOperation,
    pub file: PathBuf,
    pub build_dir: PathBuf,
    pub endpoint: String,
}

impl PrServiceRequest {
    pub fn new(
        operation: PrServiceOperation,
        file: PathBuf,
        build_dir: PathBuf,
        endpoint: String,
    ) -> Result<Self, &'static str> {
        let extension = file.extension().and_then(|value| value.to_str());
        if !absolute_normal_path(&file)
            || !matches!(extension, Some("conf" | "inc"))
            || !absolute_normal_path(&build_dir)
            || !bounded_text(&endpoint)
        {
            return Err("PR service file must be an absolute .conf or .inc path");
        }
        Ok(Self {
            operation,
            file,
            build_dir,
            endpoint,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSignatureCacheRequest {
    pub locked_signatures: PathBuf,
    pub input_cache: PathBuf,
    pub output_cache: PathBuf,
    pub native_lsb: String,
    pub filter: Option<PathBuf>,
}

impl LockedSignatureCacheRequest {
    pub fn new(
        locked_signatures: PathBuf,
        input_cache: PathBuf,
        output_cache: PathBuf,
        native_lsb: String,
        filter: Option<PathBuf>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&locked_signatures)
            || !absolute_normal_path(&input_cache)
            || !absolute_normal_path(&output_cache)
            || input_cache == output_cache
            || !bounded_token(&native_lsb)
            || filter
                .as_ref()
                .is_some_and(|path| !absolute_normal_path(path))
        {
            return Err("locked signature cache request is invalid");
        }
        Ok(Self {
            locked_signatures,
            input_cache,
            output_cache,
            native_lsb,
            filter,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildComparisonRequest {
    pub repository: PathBuf,
    pub from_revision: Option<String>,
    pub to_revision: Option<String>,
    pub report_version: bool,
    pub report_all: bool,
    pub signatures: bool,
    pub signature_diff: bool,
    pub exclude_paths: Vec<String>,
    pub no_colour: bool,
}

impl BuildComparisonRequest {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        if !absolute_normal_path(&value.repository)
            || [value.from_revision.as_deref(), value.to_revision.as_deref()]
                .into_iter()
                .flatten()
                .any(|revision| !bounded_text(revision))
            || value.exclude_paths.iter().any(|path| !bounded_text(path))
        {
            return Err("build comparison request is invalid");
        }
        value.exclude_paths = normalize_text(value.exclude_paths, MAX_MAINTENANCE_ARGUMENTS);
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitArchiveRequest {
    pub data_dir: PathBuf,
    pub git_dir: PathBuf,
    pub create: bool,
    pub bare: bool,
    pub create_tag: bool,
    pub branch_name: String,
    pub tag_name: Option<String>,
    pub commit_subject: String,
    pub commit_body: String,
    pub tag_subject: String,
    pub tag_body: String,
    pub exclusions: Vec<String>,
    pub notes: Vec<(String, PathBuf)>,
    pub push_remote: Option<String>,
}

impl GitArchiveRequest {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        if !absolute_normal_path(&value.data_dir)
            || !absolute_normal_path(&value.git_dir)
            || !bounded_text(&value.branch_name)
            || !bounded_text(&value.commit_subject)
            || (!value.commit_body.is_empty() && !bounded_text(&value.commit_body))
            || !bounded_text(&value.tag_subject)
            || (!value.tag_body.is_empty() && !bounded_text(&value.tag_body))
            || value
                .tag_name
                .as_deref()
                .is_some_and(|name| !bounded_text(name))
            || value
                .push_remote
                .as_deref()
                .is_some_and(|remote| !bounded_token(remote))
            || value
                .notes
                .iter()
                .any(|(reference, path)| !bounded_text(reference) || !absolute_normal_path(path))
        {
            return Err("Git archive request is invalid");
        }
        value.exclusions = normalize_text(value.exclusions, MAX_MAINTENANCE_ARGUMENTS);
        value
            .notes
            .sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        value.notes.dedup();
        value.notes.truncate(MAX_MAINTENANCE_ARGUMENTS);
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceOperation {
    SstateReadiness(SstateReadinessRequest),
    SstateCleanup(SstateCleanupPreview),
    PrService(PrServiceRequest),
    LockedSignatureCache(LockedSignatureCacheRequest),
    BuildHistoryComparison(BuildComparisonRequest),
    BuildCompare(BuildComparisonRequest),
    GitArchive(GitArchiveRequest),
}

impl MaintenanceOperation {
    pub fn tool(&self) -> MaintenanceTool {
        match self {
            Self::SstateReadiness(_) => MaintenanceTool::OeCheckSstate,
            Self::SstateCleanup(_) => MaintenanceTool::SstateCacheManagement,
            Self::PrService(_) => MaintenanceTool::PrServiceTool,
            Self::LockedSignatureCache(_) => MaintenanceTool::LockedSignatureCache,
            Self::BuildHistoryComparison(_) => MaintenanceTool::BuildHistoryDiff,
            Self::BuildCompare(_) => MaintenanceTool::BuildCompare,
            Self::GitArchive(_) => MaintenanceTool::GitArchive,
        }
    }

    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Self::SstateCleanup(_)
                | Self::PrService(_)
                | Self::LockedSignatureCache(_)
                | Self::GitArchive(_)
        )
    }

    pub fn network_side_effect(&self) -> bool {
        matches!(
            self,
            Self::GitArchive(GitArchiveRequest {
                push_remote: Some(_),
                ..
            })
        )
    }

    pub fn cleanup_phrase(&self) -> Option<String> {
        match self {
            Self::SstateCleanup(preview) => Some(preview.required_phrase()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOperationPreview {
    pub id: u64,
    pub capability_request: u64,
    pub operation: MaintenanceOperation,
    pub arguments: Vec<String>,
    pub limitations: Vec<String>,
}

impl MaintenanceOperationPreview {
    pub fn new(
        id: u64,
        capability_request: u64,
        operation: MaintenanceOperation,
        arguments: Vec<String>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if id == 0
            || capability_request == 0
            || arguments.is_empty()
            || arguments.len() > MAX_MAINTENANCE_ARGUMENTS
            || arguments.iter().any(|argument| !bounded_text(argument))
        {
            return Err("Maintenance operation preview is invalid");
        }
        Ok(Self {
            id,
            capability_request,
            operation,
            arguments,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }
}

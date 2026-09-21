#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaintenanceSessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOutputLine {
    pub stream: MaintenanceOutputStream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceReadinessField {
    #[default]
    Targets,
    Mode,
    Output,
    Log,
    Timeout,
}

impl MaintenanceReadinessField {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Targets, false) | (Self::Output, true) => Self::Mode,
            (Self::Mode, false) | (Self::Log, true) => Self::Output,
            (Self::Output, false) | (Self::Timeout, true) => Self::Log,
            (Self::Log, false) | (Self::Targets, true) => Self::Timeout,
            (Self::Timeout, false) | (Self::Mode, true) => Self::Targets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceReadinessDraft {
    pub field: MaintenanceReadinessField,
    pub targets: String,
    pub mode: SstateReadinessMode,
    pub output: String,
    pub log: String,
    pub timeout: String,
    pub validation: Option<String>,
}

impl Default for MaintenanceReadinessDraft {
    fn default() -> Self {
        Self {
            field: MaintenanceReadinessField::Targets,
            targets: String::new(),
            mode: SstateReadinessMode::IsolatedTmpdir,
            output: String::new(),
            log: String::new(),
            timeout: "3600".into(),
            validation: None,
        }
    }
}

impl MaintenanceReadinessDraft {
    pub fn request(&self) -> Result<SstateReadinessRequest, &'static str> {
        let targets = self
            .targets
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|target| !target.is_empty())
            .map(str::to_owned)
            .collect();
        let optional_path = |value: &str| {
            if value.is_empty() {
                Ok(None)
            } else {
                let path = PathBuf::from(value);
                absolute_normal_path(&path)
                    .then_some(Some(path))
                    .ok_or("output and log paths must be absolute normalized paths")
            }
        };
        let timeout_seconds = self
            .timeout
            .parse::<u64>()
            .map_err(|_| "timeout must be a positive integer")?;
        SstateReadinessRequest::new(
            targets,
            self.mode,
            optional_path(&self.output)?,
            optional_path(&self.log)?,
            timeout_seconds,
        )
    }

    pub fn is_bounded(&self) -> bool {
        [&self.targets, &self.output, &self.log, &self.timeout]
            .into_iter()
            .all(|value| value.len() <= MAX_MAINTENANCE_TEXT_BYTES && !value.contains('\n'))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceCleanupField {
    #[default]
    Duplicates,
    Orphans,
    UnreferencedByStamps,
    Jobs,
}

impl MaintenanceCleanupField {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Duplicates, false) | (Self::UnreferencedByStamps, true) => Self::Orphans,
            (Self::Orphans, false) | (Self::Jobs, true) => Self::UnreferencedByStamps,
            (Self::UnreferencedByStamps, false) | (Self::Duplicates, true) => Self::Jobs,
            (Self::Jobs, false) | (Self::Orphans, true) => Self::Duplicates,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceCleanupDraft {
    pub field: MaintenanceCleanupField,
    pub cache_dir: PathBuf,
    pub stamps_dirs: Vec<PathBuf>,
    pub duplicates: bool,
    pub orphans: bool,
    pub unreferenced_by_stamps: bool,
    pub jobs: String,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenancePrServiceDraft {
    pub operation: PrServiceOperation,
    pub file: String,
    pub build_dir: PathBuf,
    pub endpoint: String,
    pub validation: Option<String>,
}

impl MaintenancePrServiceDraft {
    pub fn from_metadata(
        metadata: &MaintenanceMetadata,
        operation: PrServiceOperation,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            operation,
            file: String::new(),
            build_dir: metadata
                .build_dir
                .clone()
                .ok_or("BUILDDIR is unavailable")?,
            endpoint: metadata
                .prserv_host
                .clone()
                .ok_or("PRSERV_HOST is unavailable")?,
            validation: None,
        })
    }

    pub fn request(&self) -> Result<PrServiceRequest, &'static str> {
        PrServiceRequest::new(
            self.operation,
            PathBuf::from(&self.file),
            self.build_dir.clone(),
            self.endpoint.clone(),
        )
    }

    pub fn is_bounded(&self) -> bool {
        self.file.len() <= MAX_MAINTENANCE_TEXT_BYTES && !self.file.contains('\n')
    }
}

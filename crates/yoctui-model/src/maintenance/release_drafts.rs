#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceLockedCacheField {
    #[default]
    LockedSignatures,
    InputCache,
    OutputCache,
    Filter,
}

impl MaintenanceLockedCacheField {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::LockedSignatures, false) | (Self::OutputCache, true) => Self::InputCache,
            (Self::InputCache, false) | (Self::Filter, true) => Self::OutputCache,
            (Self::OutputCache, false) | (Self::LockedSignatures, true) => Self::Filter,
            (Self::Filter, false) | (Self::InputCache, true) => Self::LockedSignatures,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceLockedCacheDraft {
    pub field: MaintenanceLockedCacheField,
    pub locked_signatures: String,
    pub input_cache: String,
    pub output_cache: String,
    pub native_lsb: String,
    pub filter: String,
    pub validation: Option<String>,
}

impl MaintenanceLockedCacheDraft {
    pub fn from_metadata(metadata: &MaintenanceMetadata) -> Result<Self, &'static str> {
        Ok(Self {
            field: MaintenanceLockedCacheField::LockedSignatures,
            locked_signatures: String::new(),
            input_cache: String::new(),
            output_cache: String::new(),
            native_lsb: metadata
                .native_lsb
                .clone()
                .ok_or("NATIVELSBSTRING is unavailable")?,
            filter: String::new(),
            validation: None,
        })
    }

    pub fn request(&self) -> Result<LockedSignatureCacheRequest, &'static str> {
        LockedSignatureCacheRequest::new(
            PathBuf::from(&self.locked_signatures),
            PathBuf::from(&self.input_cache),
            PathBuf::from(&self.output_cache),
            self.native_lsb.clone(),
            (!self.filter.is_empty()).then(|| PathBuf::from(&self.filter)),
        )
    }

    pub fn is_bounded(&self) -> bool {
        [
            &self.locked_signatures,
            &self.input_cache,
            &self.output_cache,
            &self.filter,
        ]
        .into_iter()
        .all(|value| value.len() <= MAX_MAINTENANCE_TEXT_BYTES && !value.contains('\n'))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceBuildHistoryField {
    #[default]
    FromRevision,
    ToRevision,
    ReportVersion,
    ReportAll,
    Signatures,
    SignatureDiff,
    ExcludePaths,
    NoColour,
}

impl MaintenanceBuildHistoryField {
    pub fn cycle(self, backwards: bool) -> Self {
        const FIELDS: [MaintenanceBuildHistoryField; 8] = [
            MaintenanceBuildHistoryField::FromRevision,
            MaintenanceBuildHistoryField::ToRevision,
            MaintenanceBuildHistoryField::ReportVersion,
            MaintenanceBuildHistoryField::ReportAll,
            MaintenanceBuildHistoryField::Signatures,
            MaintenanceBuildHistoryField::SignatureDiff,
            MaintenanceBuildHistoryField::ExcludePaths,
            MaintenanceBuildHistoryField::NoColour,
        ];
        let index = FIELDS
            .iter()
            .position(|field| *field == self)
            .expect("build-history field belongs to its fixed field order");
        FIELDS[if backwards {
            index.checked_sub(1).unwrap_or(FIELDS.len() - 1)
        } else {
            (index + 1) % FIELDS.len()
        }]
    }

    pub fn is_toggle(self) -> bool {
        matches!(
            self,
            Self::ReportVersion
                | Self::ReportAll
                | Self::Signatures
                | Self::SignatureDiff
                | Self::NoColour
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceBuildHistoryDraft {
    pub field: MaintenanceBuildHistoryField,
    pub repository: PathBuf,
    pub from_revision: String,
    pub to_revision: String,
    pub report_version: bool,
    pub report_all: bool,
    pub signatures: bool,
    pub signature_diff: bool,
    pub exclude_paths: String,
    pub no_colour: bool,
    pub validation: Option<String>,
}

impl MaintenanceBuildHistoryDraft {
    pub fn from_metadata(metadata: &MaintenanceMetadata) -> Result<Self, &'static str> {
        Ok(Self {
            field: MaintenanceBuildHistoryField::FromRevision,
            repository: metadata
                .buildhistory_dir
                .clone()
                .ok_or("BUILDHISTORY_DIR is unavailable")?,
            from_revision: String::new(),
            to_revision: String::new(),
            report_version: false,
            report_all: false,
            signatures: false,
            signature_diff: false,
            exclude_paths: String::new(),
            no_colour: false,
            validation: None,
        })
    }

    pub fn request(&self) -> Result<BuildComparisonRequest, &'static str> {
        let optional = |value: &str| (!value.is_empty()).then(|| value.to_owned());
        BuildComparisonRequest::new(BuildComparisonRequest {
            repository: self.repository.clone(),
            from_revision: optional(&self.from_revision),
            to_revision: optional(&self.to_revision),
            report_version: self.report_version,
            report_all: self.report_all,
            signatures: self.signatures,
            signature_diff: self.signature_diff,
            exclude_paths: self
                .exclude_paths
                .split(',')
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect(),
            no_colour: self.no_colour,
        })
    }

    pub fn is_bounded(&self) -> bool {
        [&self.from_revision, &self.to_revision, &self.exclude_paths]
            .into_iter()
            .all(|value| value.len() <= MAX_MAINTENANCE_TEXT_BYTES && !value.contains('\n'))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceGitArchiveField {
    #[default]
    DataDir,
    GitDir,
    Create,
    Bare,
    CreateTag,
    BranchName,
    TagName,
    CommitSubject,
    CommitBody,
    TagSubject,
    TagBody,
    Exclusions,
    Notes,
    PushRemote,
}

impl MaintenanceGitArchiveField {
    pub fn cycle(self, backwards: bool) -> Self {
        const FIELDS: [MaintenanceGitArchiveField; 14] = [
            MaintenanceGitArchiveField::DataDir,
            MaintenanceGitArchiveField::GitDir,
            MaintenanceGitArchiveField::Create,
            MaintenanceGitArchiveField::Bare,
            MaintenanceGitArchiveField::CreateTag,
            MaintenanceGitArchiveField::BranchName,
            MaintenanceGitArchiveField::TagName,
            MaintenanceGitArchiveField::CommitSubject,
            MaintenanceGitArchiveField::CommitBody,
            MaintenanceGitArchiveField::TagSubject,
            MaintenanceGitArchiveField::TagBody,
            MaintenanceGitArchiveField::Exclusions,
            MaintenanceGitArchiveField::Notes,
            MaintenanceGitArchiveField::PushRemote,
        ];
        let index = FIELDS
            .iter()
            .position(|field| *field == self)
            .expect("Git archive field belongs to its fixed field order");
        FIELDS[if backwards {
            index.checked_sub(1).unwrap_or(FIELDS.len() - 1)
        } else {
            (index + 1) % FIELDS.len()
        }]
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Create | Self::Bare | Self::CreateTag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceGitArchiveDraft {
    pub field: MaintenanceGitArchiveField,
    pub data_dir: String,
    pub git_dir: String,
    pub create: bool,
    pub bare: bool,
    pub create_tag: bool,
    pub branch_name: String,
    pub tag_name: String,
    pub commit_subject: String,
    pub commit_body: String,
    pub tag_subject: String,
    pub tag_body: String,
    pub exclusions: String,
    pub notes: String,
    pub push_remote: String,
    pub validation: Option<String>,
}

impl Default for MaintenanceGitArchiveDraft {
    fn default() -> Self {
        Self {
            field: MaintenanceGitArchiveField::DataDir,
            data_dir: String::new(),
            git_dir: String::new(),
            create: true,
            bare: false,
            create_tag: true,
            branch_name: "release/{machine}".into(),
            tag_name: "release/{tag_number}".into(),
            commit_subject: "Release {commit}".into(),
            commit_body: String::new(),
            tag_subject: "Release tag {tag_number}".into(),
            tag_body: String::new(),
            exclusions: String::new(),
            notes: String::new(),
            push_remote: String::new(),
            validation: None,
        }
    }
}

impl MaintenanceGitArchiveDraft {
    pub fn request(&self) -> Result<GitArchiveRequest, &'static str> {
        let comma_list = |value: &str| {
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let notes = comma_list(&self.notes)
            .into_iter()
            .map(|entry| {
                let (reference, path) = entry
                    .split_once('=')
                    .ok_or("notes must use reference=/absolute/file entries")?;
                if reference.is_empty() || path.is_empty() {
                    return Err("notes must use reference=/absolute/file entries");
                }
                Ok((reference.to_owned(), PathBuf::from(path)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        GitArchiveRequest::new(GitArchiveRequest {
            data_dir: PathBuf::from(&self.data_dir),
            git_dir: PathBuf::from(&self.git_dir),
            create: self.create,
            bare: self.bare,
            create_tag: self.create_tag,
            branch_name: self.branch_name.clone(),
            tag_name: (!self.tag_name.is_empty()).then(|| self.tag_name.clone()),
            commit_subject: self.commit_subject.clone(),
            commit_body: self.commit_body.clone(),
            tag_subject: self.tag_subject.clone(),
            tag_body: self.tag_body.clone(),
            exclusions: comma_list(&self.exclusions),
            notes,
            push_remote: (!self.push_remote.is_empty()).then(|| self.push_remote.clone()),
        })
    }

    pub fn is_bounded(&self) -> bool {
        [
            &self.data_dir,
            &self.git_dir,
            &self.branch_name,
            &self.tag_name,
            &self.commit_subject,
            &self.commit_body,
            &self.tag_subject,
            &self.tag_body,
            &self.exclusions,
            &self.notes,
            &self.push_remote,
        ]
        .into_iter()
        .all(|value| value.len() <= MAX_MAINTENANCE_TEXT_BYTES && !value.contains('\n'))
    }
}

impl MaintenanceCleanupDraft {
    pub fn from_metadata(metadata: &MaintenanceMetadata) -> Result<Self, &'static str> {
        let cache_dir = metadata
            .sstate_dir
            .clone()
            .ok_or("SSTATE_DIR is unavailable")?;
        Ok(Self {
            field: MaintenanceCleanupField::Duplicates,
            cache_dir,
            stamps_dirs: metadata.stamps_dirs.clone(),
            duplicates: true,
            orphans: false,
            unreferenced_by_stamps: false,
            jobs: "1".into(),
            validation: None,
        })
    }

    pub fn request(&self) -> Result<SstateCleanupRequest, &'static str> {
        let modes = [
            (self.duplicates, SstateCleanupMode::Duplicates),
            (self.orphans, SstateCleanupMode::Orphans),
            (
                self.unreferenced_by_stamps,
                SstateCleanupMode::UnreferencedByStamps,
            ),
        ]
        .into_iter()
        .filter_map(|(selected, mode)| selected.then_some(mode))
        .collect();
        let jobs = self
            .jobs
            .parse::<u16>()
            .map_err(|_| "jobs must be a positive integer")?;
        SstateCleanupRequest::new(
            self.cache_dir.clone(),
            self.stamps_dirs.clone(),
            modes,
            jobs,
        )
    }

    pub fn is_bounded(&self) -> bool {
        self.jobs.len() <= 5
            && self
                .jobs
                .chars()
                .all(|character| character.is_ascii_digit())
    }
}

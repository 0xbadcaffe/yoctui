#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceReleaseEvidenceSnapshot {
    pub root: PathBuf,
    pub files: Vec<MaintenanceFileIdentity>,
    pub limitations: Vec<String>,
}

impl MaintenanceReleaseEvidenceSnapshot {
    pub fn capture(root: &Path) -> Result<Self, MaintenanceReleaseAdapterError> {
        let root = canonical_directory(root)?;
        let mut queue = VecDeque::from([root.clone()]);
        let mut files = Vec::new();
        let mut limitations = Vec::new();
        let mut directories = 0usize;
        while let Some(directory) = queue.pop_front() {
            directories += 1;
            if directories > MAX_EVIDENCE_SCAN_DIRECTORIES {
                push_limitation(
                    &mut limitations,
                    "release evidence directory count reached the limit".into(),
                );
                break;
            }
            let mut entries = fs::read_dir(&directory)
                .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(directory.clone()))?
                .take(MAX_MAINTENANCE_PATHS + 1)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(directory.clone()))?;
            entries.sort_by_key(fs::DirEntry::file_name);
            if entries.len() > MAX_MAINTENANCE_PATHS {
                entries.truncate(MAX_MAINTENANCE_PATHS);
                push_limitation(
                    &mut limitations,
                    format!(
                        "entry count reached the limit beneath {}",
                        directory.display()
                    ),
                );
            }
            for entry in entries {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.clone()))?;
                if metadata.file_type().is_symlink() {
                    push_limitation(
                        &mut limitations,
                        format!("ignored symlink evidence {}", path.display()),
                    );
                } else if metadata.is_dir() {
                    if directories + queue.len() < MAX_EVIDENCE_SCAN_DIRECTORIES {
                        queue.push_back(path);
                    } else {
                        push_limitation(
                            &mut limitations,
                            "release evidence directory count reached the limit".into(),
                        );
                    }
                } else if metadata.is_file() {
                    files.push(regular_file_identity(&path)?);
                    if files.len() >= MAX_MAINTENANCE_EVIDENCE {
                        push_limitation(
                            &mut limitations,
                            "release evidence file count reached the limit".into(),
                        );
                        queue.clear();
                        break;
                    }
                }
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self {
            root,
            files,
            limitations,
        })
    }

    pub fn changed_evidence(
        &self,
    ) -> Result<Vec<MaintenanceEvidence>, MaintenanceReleaseAdapterError> {
        let after = Self::capture(&self.root)?;
        let before = self
            .files
            .iter()
            .map(|identity| (identity.path.clone(), identity))
            .collect::<BTreeMap<_, _>>();
        after
            .files
            .into_iter()
            .filter(|identity| {
                before
                    .get(&identity.path)
                    .is_none_or(|old| *old != identity)
            })
            .map(|identity| {
                let label = if before.contains_key(&identity.path) {
                    "replaced locked-signature cache evidence"
                } else {
                    "created locked-signature cache evidence"
                };
                MaintenanceEvidence::new(identity, label.into())
                    .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
            })
            .collect()
    }
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitation.is_empty()
        || limitation.len() > MAX_MAINTENANCE_TEXT_BYTES
        || limitations.len() >= MAX_MAINTENANCE_LIMITATIONS
        || limitations.contains(&limitation)
    {
        return;
    }
    limitations.push(limitation);
}

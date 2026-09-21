#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityMapperCommandSpec {
    id: SecuritySessionId,
    preview: SecurityOperationPreview,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    executable_identity: FileIdentity,
    input_identities: Vec<FileIdentity>,
}

impl SecurityMapperCommandSpec {
    pub fn from_paths(
        id: SecuritySessionId,
        executable: PathBuf,
        arguments: Vec<String>,
        report_roots: Vec<PathBuf>,
    ) -> Result<Self, SecurityMapperAdapterError> {
        let preview = SecurityOperationPreview {
            id,
            scope: yoctui_model::SecurityScope::Image {
                target: "security".into(),
                machine: "unknown".into(),
                distro: "unknown".into(),
            },
            operation: SecurityOperation::PackageMap {
                executable: executable.clone(),
                arguments: arguments.clone(),
            },
            indexed_arguments: indexed_arguments(&executable, &arguments),
            report_roots,
        };
        Self::from_preview(&preview)
    }

    pub fn from_preview(
        preview: &SecurityOperationPreview,
    ) -> Result<Self, SecurityMapperAdapterError> {
        if preview.id.0 == 0
            || !preview.scope.is_valid()
            || preview.report_roots.is_empty()
            || preview.report_roots.len() > MAX_SECURITY_PATHS
        {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "session, scope, or report roots are invalid".into(),
            ));
        }
        let SecurityOperation::PackageMap {
            executable,
            arguments,
        } = &preview.operation
        else {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "operation is not package mapping".into(),
            ));
        };
        if arguments.is_empty() || arguments.len() > MAX_SECURITY_MAPPER_ARGUMENTS {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping arguments are empty or excessive".into(),
            ));
        }
        if arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > MAX_SECURITY_TEXT_BYTES
                || argument.chars().any(char::is_control)
        }) {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping arguments contain an invalid field".into(),
            ));
        }
        let executable_identity = FileIdentity::executable(executable)?;
        let mut report_roots = preview
            .report_roots
            .iter()
            .map(|path| FileIdentity::input(path))
            .collect::<Result<Vec<_>, _>>()?;
        report_roots.sort_by(|left, right| left.path.cmp(&right.path));
        report_roots.dedup_by(|left, right| left.path == right.path);
        if report_roots
            .iter()
            .map(|identity| &identity.path)
            .ne(preview.report_roots.iter())
        {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "report roots must be sorted and unique".into(),
            ));
        }
        let input_identities = arguments
            .iter()
            .map(PathBuf::from)
            .map(|path| {
                if !preview.report_roots.contains(&path) {
                    return Err(SecurityMapperAdapterError::UnsafeInput(path));
                }
                FileIdentity::input(&path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let unique_inputs = input_identities
            .iter()
            .map(|identity| &identity.path)
            .collect::<std::collections::BTreeSet<_>>();
        if unique_inputs.len() != input_identities.len() {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping inputs must be unique".into(),
            ));
        }
        let expected_indexed = indexed_arguments(executable, arguments);
        if preview.indexed_arguments != expected_indexed {
            return Err(SecurityMapperAdapterError::PreviewMismatch);
        }
        let current_directory = input_identities[0]
            .directory
            .then(|| input_identities[0].path.clone())
            .or_else(|| input_identities[0].path.parent().map(Path::to_path_buf))
            .ok_or_else(|| {
                SecurityMapperAdapterError::UnsafeInput(input_identities[0].path.clone())
            })?;
        Ok(Self {
            id: preview.id,
            preview: preview.clone(),
            executable: executable.clone(),
            arguments: arguments.iter().map(OsString::from).collect(),
            current_directory,
            executable_identity,
            input_identities,
        })
    }

    pub fn id(&self) -> SecuritySessionId {
        self.id
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    fn revalidate(&self) -> Result<(), SecurityMapperAdapterError> {
        self.executable_identity.revalidate_executable()?;
        for identity in &self.input_identities {
            identity.revalidate_input()?;
        }
        let reconstructed = Self::from_preview(&self.preview)?;
        if reconstructed.id != self.id
            || reconstructed.executable != self.executable
            || reconstructed.arguments != self.arguments
            || reconstructed.current_directory != self.current_directory
            || reconstructed.executable_identity != self.executable_identity
            || reconstructed.input_identities != self.input_identities
        {
            return Err(SecurityMapperAdapterError::PreviewMismatch);
        }
        Ok(())
    }
}

fn indexed_arguments(executable: &Path, arguments: &[String]) -> Vec<String> {
    std::iter::once(format!("0: {}", executable.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {argument}", index + 1)),
        )
        .collect()
}

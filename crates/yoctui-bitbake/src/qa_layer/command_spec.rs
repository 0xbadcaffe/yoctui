#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCommandSpec {
    id: QaLayerSessionId,
    preview: QaLayerOperationPreview,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
}

impl QaLayerCommandSpec {
    /// Reconstruct a confirmed command at the daemon boundary.
    ///
    /// The wire request intentionally carries paths and bounded arguments rather
    /// than a serialized filesystem identity.  The executable identity is
    /// re-read on the daemon host immediately before validation, so replacement
    /// or symlink attacks still fail the normal `from_preview` checks.
    pub fn from_paths(
        session: QaLayerSessionId,
        operation: QaLayerOperationId,
        check: QaCheckId,
        layer: QaLayerIdentity,
        executable: PathBuf,
        arguments: Vec<String>,
        report_roots: Vec<PathBuf>,
    ) -> Result<Self, QaLayerAdapterError> {
        if session.0 == 0 || operation.0 == 0 || !check.is_valid() || !layer.is_valid() {
            return Err(QaLayerAdapterError::InvalidPreview(
                "session, operation, check, or layer is invalid".into(),
            ));
        }
        let executable = executable_identity(&executable)?;
        let indexed_arguments = indexed_arguments(&executable.path, &arguments);
        let preview = QaLayerOperationPreview {
            id: operation,
            check,
            layer,
            executable,
            indexed_arguments,
            arguments,
            report_roots,
            limitations: Vec::new(),
        };
        Self::from_preview(session, &preview)
    }

    pub fn from_preview(
        session: QaLayerSessionId,
        preview: &QaLayerOperationPreview,
    ) -> Result<Self, QaLayerAdapterError> {
        if session.0 == 0
            || preview.id.0 == 0
            || !preview.check.is_valid()
            || !preview.layer.is_valid()
            || !preview.executable.is_valid()
            || preview.arguments.is_empty()
            || preview.arguments.len() > MAX_QA_LAYER_ARGUMENTS
            || preview.arguments.iter().any(|value| !bounded_text(value))
            || preview.report_roots.len() > MAX_QA_REPORT_PATHS
        {
            return Err(QaLayerAdapterError::InvalidPreview(
                "session, operation, layer, executable, arguments, or roots are invalid".into(),
            ));
        }
        if preview.arguments != vec![preview.layer.root.display().to_string()] {
            return Err(QaLayerAdapterError::InvalidPreview(
                "layer-QA arguments are not the exact configured-layer vector".into(),
            ));
        }
        revalidate_executable(&preview.executable)?;
        let layer = canonical_directory(&preview.layer.root)
            .map_err(|_| QaLayerAdapterError::UnsafeLayer(preview.layer.root.clone()))?;
        let mut roots = preview
            .report_roots
            .iter()
            .map(|path| {
                canonical_file_or_directory(path)
                    .map_err(|_| QaLayerAdapterError::UnsafeReportRoot(path.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        roots.sort();
        roots.dedup();
        if roots != preview.report_roots {
            return Err(QaLayerAdapterError::InvalidPreview(
                "report roots must be canonical, sorted, and unique".into(),
            ));
        }
        let expected_indexed = indexed_arguments(&preview.executable.path, &preview.arguments);
        if preview.indexed_arguments != expected_indexed {
            return Err(QaLayerAdapterError::PreviewMismatch);
        }
        Ok(Self {
            id: session,
            preview: preview.clone(),
            executable: preview.executable.path.clone(),
            arguments: preview.arguments.iter().map(OsString::from).collect(),
            current_directory: layer,
        })
    }

    pub fn id(&self) -> QaLayerSessionId {
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

    fn revalidate(&self) -> Result<(), QaLayerAdapterError> {
        let reconstructed = Self::from_preview(self.id, &self.preview)?;
        if reconstructed != *self {
            return Err(QaLayerAdapterError::PreviewMismatch);
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

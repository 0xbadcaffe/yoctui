impl MaintenanceSstateCommandSpec {
    pub fn external_from_paths(
        session: MaintenanceSessionId,
        executable: PathBuf,
        expected_name: String,
        arguments: Vec<String>,
        current_directory: PathBuf,
    ) -> Result<Self, MaintenanceSstateAdapterError> {
        if session.0 == 0 || arguments.is_empty() || expected_name.is_empty() {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "external command identity is invalid".into(),
            ));
        }
        let identity = executable_identity(&executable, &expected_name)?;
        let request = SstateReadinessRequest::new(
            vec!["maintenance".into()],
            SstateReadinessMode::SameTmpdir,
            None,
            None,
            1,
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        let preview = MaintenanceOperationPreview::new(
            session.0,
            1,
            MaintenanceOperation::SstateReadiness(request),
            indexed_arguments(&executable, &arguments),
            Vec::new(),
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        Self::external(MaintenanceExternalCommand {
            session,
            kind: MaintenanceSstateCommandKind::GitArchiveLocal,
            executable_identity: identity,
            expected_executable_name: expected_name,
            arguments: arguments.into_iter().map(OsString::from).collect(),
            current_directory,
            timeout: SSTATE_OPERATION_TIMEOUT,
            preview,
            guards: Vec::new(),
        })
    }

    pub fn readiness(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        request: SstateReadinessRequest,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        let (executable, interface) = available_tool(snapshot, MaintenanceTool::OeCheckSstate)?;
        if interface != MaintenanceToolInterface::Native {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported oe-check-sstate interface".into(),
            ));
        }
        revalidate_executable(executable, "oe-check-sstate")?;
        let build_dir = snapshot
            .metadata
            .build_dir
            .as_deref()
            .ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })
            .and_then(canonical_directory)?;
        for path in [request.output.as_deref(), request.log.as_deref()]
            .into_iter()
            .flatten()
        {
            validate_output_path(path)?;
        }
        if Duration::from_secs(request.timeout_seconds) > SSTATE_OPERATION_TIMEOUT {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "readiness timeout exceeds the one-hour adapter limit".into(),
            ));
        }
        let timeout = Duration::from_secs(request.timeout_seconds);
        let arguments = readiness_arguments(&request);
        let indexed = indexed_arguments(&executable.path, &arguments);
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::SstateReadiness(request),
            indexed,
            Vec::new(),
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        let mut environment = BTreeMap::new();
        environment.insert("BB_SETSCENE_ENFORCE".into(), "1".into());
        environment.insert("BUILDDIR".into(), build_dir.as_os_str().into());
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: MaintenanceSstateCommandKind::Readiness,
                executable_identity: executable.clone(),
                expected_executable_name: "oe-check-sstate".into(),
                interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment,
                current_directory: build_dir,
                timeout,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: None,
                cleanup_candidates: Vec::new(),
                pr_service_guard: None,
                external_guards: Vec::new(),
            },
        ))
    }

    pub fn cleanup_preview(
        session: MaintenanceSessionId,
        snapshot: &MaintenanceCapabilitySnapshot,
        request: SstateCleanupRequest,
    ) -> Result<Self, MaintenanceSstateAdapterError> {
        let (executable, interface) =
            available_tool(snapshot, MaintenanceTool::SstateCacheManagement)?;
        if !matches!(
            interface,
            MaintenanceToolInterface::SstatePython | MaintenanceToolInterface::SstateLegacyShell
        ) {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported sstate cleanup interface".into(),
            ));
        }
        let name = expected_name(interface, &executable.path).ok_or_else(|| {
            MaintenanceSstateAdapterError::Unavailable(
                "cleanup interface has no executable name".into(),
            )
        })?;
        revalidate_executable(executable, name)?;
        validate_cleanup_request(snapshot, &request)?;
        let arguments = cleanup_arguments(&request, true, false);
        Ok(Self {
            id: session,
            kind: MaintenanceSstateCommandKind::CleanupPreview,
            executable_identity: executable.clone(),
            expected_executable_name: name.into(),
            interface,
            arguments: arguments.iter().map(OsString::from).collect(),
            environment: BTreeMap::new(),
            current_directory: snapshot.metadata.build_dir.clone().ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })?,
            timeout: SSTATE_OPERATION_TIMEOUT,
            stdin_payload: Some(b"n\n".to_vec()),
            preview: None,
            cleanup_request: Some(request),
            cleanup_candidates: Vec::new(),
            pr_service_guard: None,
            external_guards: Vec::new(),
        })
    }

    pub fn cleanup_execution(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        confirmed: &SstateCleanupPreview,
        fresh: &SstateCleanupPreview,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        if confirmed != fresh {
            return Err(MaintenanceSstateAdapterError::CandidateMismatch);
        }
        let preview_command = Self::cleanup_preview(session, snapshot, confirmed.request.clone())?;
        for candidate in &confirmed.candidates {
            let current = identity_for_candidate(&candidate.path, &confirmed.request.cache_dir)?;
            if &current != candidate {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(
                    candidate.path.clone(),
                ));
            }
        }
        let arguments = cleanup_arguments(&confirmed.request, false, true);
        let indexed = indexed_arguments(&preview_command.executable_identity.path, &arguments);
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::SstateCleanup(confirmed.clone()),
            indexed,
            Vec::new(),
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: MaintenanceSstateCommandKind::CleanupExecute,
                executable_identity: preview_command.executable_identity,
                expected_executable_name: preview_command.expected_executable_name,
                interface: preview_command.interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment: BTreeMap::new(),
                current_directory: preview_command.current_directory,
                timeout: SSTATE_OPERATION_TIMEOUT,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: Some(confirmed.request.clone()),
                cleanup_candidates: confirmed.candidates.clone(),
                pr_service_guard: None,
                external_guards: Vec::new(),
            },
        ))
    }

    pub fn pr_service(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        request: PrServiceRequest,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        let (executable, interface) = available_tool(snapshot, MaintenanceTool::PrServiceTool)?;
        if interface != MaintenanceToolInterface::Native {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported bitbake-prserv-tool interface".into(),
            ));
        }
        revalidate_executable(executable, "bitbake-prserv-tool")?;
        let build_dir = snapshot
            .metadata
            .build_dir
            .as_deref()
            .ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })
            .and_then(canonical_directory)?;
        if request.build_dir != build_dir
            || snapshot.metadata.prserv_host.as_ref() != Some(&request.endpoint)
        {
            return Err(MaintenanceSstateAdapterError::PreviewMismatch);
        }
        let guard = inspect_pr_service_file(&request)?;
        let arguments = pr_service_arguments(&request);
        let indexed = indexed_arguments(&executable.path, &arguments);
        let mut limitations = vec![
            format!("build directory: {}", build_dir.display()),
            format!("configured PR endpoint: {}", request.endpoint),
            "the native helper stops any active memory-resident BitBake server".into(),
            "the native helper invalidates BitBake cache records before parsing".into(),
        ];
        match request.operation {
            PrServiceOperation::Export => limitations
                .push("export may replace the exact selected .conf or .inc destination".into()),
            PrServiceOperation::Import => limitations.push("import changes PR service data".into()),
        }
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::PrService(request.clone()),
            indexed,
            limitations,
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: match request.operation {
                    PrServiceOperation::Export => MaintenanceSstateCommandKind::PrServiceExport,
                    PrServiceOperation::Import => MaintenanceSstateCommandKind::PrServiceImport,
                },
                executable_identity: executable.clone(),
                expected_executable_name: "bitbake-prserv-tool".into(),
                interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment: BTreeMap::new(),
                current_directory: build_dir,
                timeout: SSTATE_OPERATION_TIMEOUT,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: None,
                cleanup_candidates: Vec::new(),
                pr_service_guard: Some(guard),
                external_guards: Vec::new(),
            },
        ))
    }

    pub(crate) fn external(
        command: MaintenanceExternalCommand,
    ) -> Result<Self, MaintenanceSstateAdapterError> {
        if !matches!(
            command.kind,
            MaintenanceSstateCommandKind::LockedSignatureCache
                | MaintenanceSstateCommandKind::BuildHistoryComparison
                | MaintenanceSstateCommandKind::BuildCompare
                | MaintenanceSstateCommandKind::GitArchiveLocal
                | MaintenanceSstateCommandKind::GitArchivePush
        ) || command.expected_executable_name.is_empty()
            || command.timeout.is_zero()
        {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "external Maintenance command is invalid".into(),
            ));
        }
        Ok(Self {
            id: command.session,
            kind: command.kind,
            executable_identity: command.executable_identity,
            expected_executable_name: command.expected_executable_name,
            interface: MaintenanceToolInterface::Native,
            arguments: command.arguments,
            environment: BTreeMap::new(),
            current_directory: command.current_directory,
            timeout: command.timeout,
            stdin_payload: None,
            preview: Some(command.preview),
            cleanup_request: None,
            cleanup_candidates: Vec::new(),
            pr_service_guard: None,
            external_guards: command.guards,
        })
    }

    pub fn id(&self) -> MaintenanceSessionId {
        self.id
    }

    pub fn kind(&self) -> MaintenanceSstateCommandKind {
        self.kind
    }

    pub fn executable(&self) -> &Path {
        &self.executable_identity.path
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    fn revalidate(&self) -> Result<(), MaintenanceSstateAdapterError> {
        revalidate_executable(&self.executable_identity, &self.expected_executable_name)?;
        canonical_directory(&self.current_directory)?;
        if self.timeout.is_zero() {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "operation timeout must be nonzero".into(),
            ));
        }
        if let Some(request) = &self.cleanup_request {
            let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
                build_dir: Some(self.current_directory.clone()),
                sstate_dir: Some(request.cache_dir.clone()),
                stamps_dirs: request.stamps_dirs.clone(),
                ..MaintenanceMetadata::default()
            })
            .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
            let snapshot = MaintenanceCapabilitySnapshot::new(
                metadata,
                vec![MaintenanceToolCapability::Available {
                    tool: MaintenanceTool::SstateCacheManagement,
                    executable: self.executable_identity.clone(),
                    interface: self.interface,
                }],
                vec![],
            )
            .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
            validate_cleanup_request(&snapshot, request)?;
            for candidate in &self.cleanup_candidates {
                let current = identity_for_candidate(&candidate.path, &request.cache_dir)?;
                if &current != candidate {
                    return Err(MaintenanceSstateAdapterError::StaleIdentity(
                        candidate.path.clone(),
                    ));
                }
            }
        }
        for guard in &self.external_guards {
            revalidate_filesystem_guard(guard)?;
        }
        if let Some(preview) = &self.preview {
            let arguments = self
                .arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if preview.arguments != indexed_arguments(&self.executable_identity.path, &arguments) {
                return Err(MaintenanceSstateAdapterError::PreviewMismatch);
            }
            match &preview.operation {
                MaintenanceOperation::SstateReadiness(request) => {
                    for path in [request.output.as_deref(), request.log.as_deref()]
                        .into_iter()
                        .flatten()
                    {
                        validate_output_path(path)?;
                    }
                    if arguments != readiness_arguments(request)
                        || self.timeout != Duration::from_secs(request.timeout_seconds)
                        || self.environment.get(OsStr::new("BB_SETSCENE_ENFORCE"))
                            != Some(&OsString::from("1"))
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                MaintenanceOperation::SstateCleanup(cleanup) => {
                    if self.kind != MaintenanceSstateCommandKind::CleanupExecute
                        || arguments != cleanup_arguments(&cleanup.request, false, true)
                        || self.stdin_payload.is_some()
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                MaintenanceOperation::PrService(request) => {
                    let expected_kind = match request.operation {
                        PrServiceOperation::Export => MaintenanceSstateCommandKind::PrServiceExport,
                        PrServiceOperation::Import => MaintenanceSstateCommandKind::PrServiceImport,
                    };
                    if self.kind != expected_kind
                        || arguments != pr_service_arguments(request)
                        || request.build_dir != self.current_directory
                        || self.stdin_payload.is_some()
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                    let guard = self
                        .pr_service_guard
                        .as_ref()
                        .ok_or(MaintenanceSstateAdapterError::PreviewMismatch)?;
                    revalidate_pr_service_file(request, guard)?;
                }
                MaintenanceOperation::LockedSignatureCache(_)
                    if self.kind == MaintenanceSstateCommandKind::LockedSignatureCache => {}
                MaintenanceOperation::BuildHistoryComparison(_)
                    if self.kind == MaintenanceSstateCommandKind::BuildHistoryComparison => {}
                MaintenanceOperation::BuildCompare(_)
                    if self.kind == MaintenanceSstateCommandKind::BuildCompare => {}
                MaintenanceOperation::GitArchive(request) => {
                    let expected_kind = if request.push_remote.is_some() {
                        MaintenanceSstateCommandKind::GitArchivePush
                    } else {
                        MaintenanceSstateCommandKind::GitArchiveLocal
                    };
                    if self.kind != expected_kind {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                _ => return Err(MaintenanceSstateAdapterError::PreviewMismatch),
            }
        } else if let Some(request) = &self.cleanup_request {
            let arguments = self
                .arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if self.kind != MaintenanceSstateCommandKind::CleanupPreview
                || arguments != cleanup_arguments(request, true, false)
                || self.stdin_payload.as_deref() != Some(b"n\n")
            {
                return Err(MaintenanceSstateAdapterError::PreviewMismatch);
            }
        }
        Ok(())
    }
}

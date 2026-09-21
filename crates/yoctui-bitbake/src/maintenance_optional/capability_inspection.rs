#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaintenanceOptionalAdapterError {
    #[error("invalid Maintenance optional-integration input: {0}")]
    InvalidInput(String),
    #[error("unsafe Maintenance optional-integration path: {0}")]
    UnsafePath(PathBuf),
    #[error("Maintenance optional-integration evidence changed: {0}")]
    StaleEvidence(PathBuf),
    #[error("Maintenance optional-integration process inspection failed: {0}")]
    ProcessInspection(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOptionalCapabilityInput {
    pub build_dir: PathBuf,
    pub executable_search_path: Vec<PathBuf>,
    pub git_worktree_candidates: Vec<PathBuf>,
    pub error_report_candidates: Vec<PathBuf>,
    pub repo_workspace_candidates: Vec<PathBuf>,
    pub toaster_configuration_candidates: Vec<PathBuf>,
    pub process_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOptionalInspection {
    pub build: MaintenanceDirectoryIdentity,
    pub capability: MaintenanceCapabilitySnapshot,
    pub pull_request: OptionalPullRequestIntegration,
    pub error_report: OptionalErrorReportIntegration,
    pub repo_manifest: OptionalRepoManifestIntegration,
    pub toaster: OptionalToasterIntegration,
    pub limitations: Vec<String>,
}

impl MaintenanceOptionalInspection {
    pub fn integrations_snapshot(
        &self,
    ) -> Result<MaintenanceIntegrationsSnapshot, MaintenanceOptionalAdapterError> {
        MaintenanceIntegrationsSnapshot::new(MaintenanceIntegrationsSnapshot {
            pull_request: self.pull_request.clone(),
            error_report: self.error_report.clone(),
            repo_manifest: self.repo_manifest.clone(),
            toaster: self.toaster.clone(),
            limitations: self.limitations.clone(),
        })
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))
    }

    pub fn revalidate(&self) -> Result<(), MaintenanceOptionalAdapterError> {
        revalidate_directory(&self.build)?;
        for capability in &self.capability.tools {
            if let MaintenanceToolCapability::Available { executable, .. } = capability {
                revalidate_file(executable, true)?;
            }
        }
        if let Some(worktree) = &self.pull_request.worktree {
            revalidate_directory(&worktree.root)?;
            revalidate_file(&worktree.head, false)?;
        }
        if let Some(report) = &self.error_report.candidate_report {
            revalidate_file(report, false)?;
        }
        if let Some(workspace) = &self.repo_manifest.workspace {
            revalidate_directory(workspace)?;
        }
        if let Some(executable) = &self.repo_manifest.repo_executable {
            revalidate_file(executable, true)?;
        }
        if let Some(manifest) = &self.repo_manifest.manifest {
            revalidate_file(manifest, false)?;
        }
        for configuration in &self.toaster.configurations {
            revalidate_file(configuration, false)?;
        }
        Ok(())
    }
}

pub struct MaintenanceOptionalCapabilityInspector;

impl MaintenanceOptionalCapabilityInspector {
    pub fn inspect(
        input: MaintenanceOptionalCapabilityInput,
    ) -> Result<MaintenanceOptionalInspection, MaintenanceOptionalAdapterError> {
        let build_dir = directory_identity(&input.build_dir)?;
        let mut limitations = Vec::new();
        note_bound(
            &mut limitations,
            "executable search path",
            input.executable_search_path.len(),
        );
        note_bound(
            &mut limitations,
            "Git worktree candidates",
            input.git_worktree_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "error-report candidates",
            input.error_report_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "repo workspace candidates",
            input.repo_workspace_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "Toaster configuration candidates",
            input.toaster_configuration_candidates.len(),
        );

        let create = discover_executable(
            MaintenanceTool::CreatePullRequest,
            "create-pull-request",
            &input.executable_search_path,
            &mut limitations,
        );
        let send = discover_executable(
            MaintenanceTool::SendPullRequest,
            "send-pull-request",
            &input.executable_search_path,
            &mut limitations,
        );
        let error = discover_executable(
            MaintenanceTool::SendErrorReport,
            "send-error-report",
            &input.executable_search_path,
            &mut limitations,
        );
        let toaster_capability = discover_executable(
            MaintenanceTool::Toaster,
            "toaster",
            &input.executable_search_path,
            &mut limitations,
        );
        let repo_executable =
            discover_named_executable("repo", &input.executable_search_path, &mut limitations);

        let worktree = first_git_worktree(&input.git_worktree_candidates, &mut limitations);
        let report = first_regular_candidate(
            "error report",
            &input.error_report_candidates,
            &mut limitations,
        );
        let (repo_workspace, repo_manifest) =
            first_repo_manifest(&input.repo_workspace_candidates, &mut limitations);
        let configurations = regular_candidates(
            "Toaster configuration",
            &input.toaster_configuration_candidates,
            &mut limitations,
        );
        let (observed_processes, process_limitations) = scan_toaster_processes(&input.process_root)
            .unwrap_or_else(|error| (Vec::new(), vec![error.to_string()]));
        for limitation in &process_limitations {
            push_limitation(&mut limitations, limitation.clone());
        }

        let create_identity = available_identity(&create);
        let send_identity = available_identity(&send);
        let error_identity = available_identity(&error);
        let toaster_identity = available_identity(&toaster_capability);
        let worktree_available = worktree.is_some();
        let report_available = report.is_some();
        let repo_executable_available = repo_executable.is_some();
        let repo_workspace_available = repo_workspace.is_some();
        let repo_manifest_available = repo_manifest.is_some();
        let toaster_configuration_available = !configurations.is_empty();

        let pull_request = OptionalPullRequestIntegration {
            state: integration_state([
                create_identity.is_some(),
                send_identity.is_some(),
                worktree_available,
            ]),
            create_helper: create_identity,
            send_helper: send_identity,
            worktree,
            limitations: missing_limitations(&[
                ("create-pull-request helper", available(&create)),
                ("send-pull-request helper", available(&send)),
                ("canonical Git worktree", worktree_available),
            ]),
        };
        let error_report = OptionalErrorReportIntegration {
            state: integration_state([error_identity.is_some(), report_available]),
            helper: error_identity,
            candidate_report: report,
            limitations: missing_limitations(&[
                ("send-error-report helper", available(&error)),
                ("canonical candidate report", report_available),
            ]),
        };
        let repo_manifest_integration = OptionalRepoManifestIntegration {
            state: integration_state([
                repo_executable_available,
                repo_workspace_available,
                repo_manifest_available,
            ]),
            repo_executable,
            workspace: repo_workspace,
            manifest: repo_manifest,
            limitations: missing_limitations(&[
                ("repo executable", repo_executable_available),
                ("canonical repo workspace", repo_workspace_available),
                ("canonical repo manifest", repo_manifest_available),
            ]),
        };
        let mut toaster_limitations = missing_limitations(&[
            ("Toaster executable", available(&toaster_capability)),
            (
                "canonical Toaster configuration",
                toaster_configuration_available,
            ),
        ]);
        for limitation in process_limitations {
            push_limitation(&mut toaster_limitations, limitation);
        }
        if !observed_processes.is_empty() {
            push_limitation(
                &mut toaster_limitations,
                "Toaster process-name evidence is observational and does not prove service health"
                    .into(),
            );
        }
        let toaster = OptionalToasterIntegration {
            state: integration_state([toaster_identity.is_some(), toaster_configuration_available]),
            executable: toaster_identity,
            configurations,
            observed_processes,
            limitations: toaster_limitations,
        };

        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir.path.clone()),
            ..MaintenanceMetadata::default()
        })
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))?;
        let capability = MaintenanceCapabilitySnapshot::new(
            metadata,
            vec![create, send, error, toaster_capability],
            limitations.clone(),
        )
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))?;
        let inspection = MaintenanceOptionalInspection {
            build: build_dir,
            capability,
            pull_request,
            error_report,
            repo_manifest: repo_manifest_integration,
            toaster,
            limitations,
        };
        let _ = inspection.integrations_snapshot()?;
        inspection.revalidate()?;
        Ok(inspection)
    }
}

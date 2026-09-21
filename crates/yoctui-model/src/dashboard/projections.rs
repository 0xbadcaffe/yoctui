impl App {
    pub fn command_center_projection_at(&self, now: SystemTime) -> CommandCenterProjection<'_> {
        let recent_contexts = self
            .job_history_rows()
            .into_iter()
            .filter(|row| match row {
                JobHistoryRowRef::Daemon(_) => true,
                JobHistoryRowRef::Background(job) => job.context != BackgroundJobContext::default(),
                JobHistoryRowRef::Build(record) => record.target.is_some(),
            })
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .collect();
        let active_jobs = self
            .background_jobs
            .jobs
            .iter()
            .rev()
            .filter(|job| !job.status.is_terminal())
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .collect();
        let catalog = builtin_raw_catalog();
        let authority = self.workspace_compatibility.authority();
        let favorite_commands = self
            .raw_mode
            .favorites
            .iter()
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .map(|favorite| CommandCenterFavoriteRef {
                favorite,
                projection: favorite.project(catalog, authority),
            })
            .collect();
        let selected_terminal = self.daemon.pty_sessions.get(self.pty_selection);
        let terminals =
            selected_terminal
                .into_iter()
                .chain(self.daemon.pty_sessions.iter().enumerate().filter_map(
                    |(index, terminal)| (index != self.pty_selection).then_some(terminal),
                ))
                .take(COMMAND_CENTER_COLLECTION_LIMIT)
                .collect();
        CommandCenterProjection {
            dashboard: self.dashboard_projection_at(now),
            recent_contexts,
            active_jobs,
            favorite_commands,
            terminals,
        }
    }

    pub fn dashboard_projection_at(&self, now: SystemTime) -> DashboardProjection<'_> {
        let failures = self
            .logs
            .entries
            .iter()
            .rev()
            .filter(|entry| matches!(entry.severity, Severity::Warning | Severity::Error))
            .take(DASHBOARD_COLLECTION_LIMIT)
            .collect::<Vec<_>>();
        let recent_work = self
            .job_history_rows()
            .into_iter()
            .take(DASHBOARD_COLLECTION_LIMIT)
            .collect::<Vec<_>>();
        let artifacts = self.dashboard_artifacts();
        let environment_ready = self.build_environment.connected()
            || (self.workspace.source_dir.is_some() && self.workspace.build_dir.is_some());
        let environment = match self.daemon.status {
            ClientReplicaStatus::Disconnected => DashboardEnvironmentState::Disconnected,
            ClientReplicaStatus::Synchronizing | ClientReplicaStatus::Stale => {
                DashboardEnvironmentState::Synchronizing
            }
            ClientReplicaStatus::Current if environment_ready => DashboardEnvironmentState::Ready,
            ClientReplicaStatus::Current => DashboardEnvironmentState::NeedsConfiguration,
        };
        let next_kind = if self.build.errors > 0
            || self.build.status == BuildStatus::Failed
            || failures
                .iter()
                .any(|entry| entry.severity == Severity::Error)
        {
            DashboardNextActionKind::ReviewFailures
        } else if matches!(
            self.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        ) {
            DashboardNextActionKind::MonitorTasks
        } else if self.build.status == BuildStatus::Completed && !artifacts.is_empty() {
            DashboardNextActionKind::InspectArtifacts
        } else if !environment_ready {
            DashboardNextActionKind::ConfigureEnvironment
        } else {
            DashboardNextActionKind::StartBuild
        };
        let next_action = compatibility_ui_workspace_action_presentations(
            &self.workspace_compatibility,
            WorkspaceDestination::Dashboard,
        )
        .into_iter()
        .find(|action| action.id == next_kind.action_id())
        .map(|action| DashboardNextAction {
            kind: next_kind,
            label: action.label.into(),
            shortcut: action.shortcut.into(),
            state: action.availability.state,
            enabled: action.availability.enabled,
            reason: action.availability.exact_reason(),
        })
        .unwrap_or_else(|| DashboardNextAction {
            kind: next_kind,
            label: "Dashboard action unavailable".into(),
            shortcut: "–".into(),
            state: WorkspaceAvailabilityState::Unknown,
            enabled: false,
            reason: Some("The typed dashboard action catalog is incomplete.".into()),
        });
        let progress = self.progress_hierarchy_at(now);
        DashboardProjection {
            summary: self.build_summary_at(now),
            progress,
            next_action,
            failures,
            recent_work,
            artifacts,
            health: DashboardHealthProjection {
                environment,
                replica: self.daemon.status,
                bitbake: self.daemon.bitbake,
                build_filesystem_sample: self.workspace.build_dir.is_some()
                    && self.host_telemetry.disk_total_bytes.is_some()
                    && self.host_telemetry.disk_available_bytes.is_some(),
            },
        }
    }

    fn dashboard_artifacts(&self) -> Vec<DashboardArtifactRef<'_>> {
        let mut artifacts = Vec::new();
        for job in self.background_jobs.jobs.iter().rev() {
            if let Some(result) = job.result.as_ref() {
                for path in result.artifacts.iter().rev() {
                    push_dashboard_artifact(
                        &mut artifacts,
                        path,
                        DashboardArtifactSource::BackgroundJob,
                    );
                }
            }
        }
        if let Some(inventory) = self.image_artifacts.artifacts() {
            for artifact in inventory.iter().rev() {
                push_dashboard_artifact(
                    &mut artifacts,
                    artifact.identity.path.as_path(),
                    DashboardArtifactSource::ImageInventory,
                );
            }
        }
        if let Some(inventory) = self.sdk_artifacts.artifacts() {
            for artifact in inventory.iter().rev() {
                push_dashboard_artifact(
                    &mut artifacts,
                    artifact.identity.path.as_path(),
                    DashboardArtifactSource::SdkInventory,
                );
            }
        }
        artifacts
    }
}

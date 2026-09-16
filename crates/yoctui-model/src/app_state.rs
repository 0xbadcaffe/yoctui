//! App state.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub source_git_status: SourceGitStatus,
    pub background_activities: std::collections::BTreeSet<BackgroundActivity>,
    pub daemon: ClientDaemonView,
    pub client_access_origin: ClientAccessOrigin,
    pub workspace_compatibility: WorkspaceCompatibilityState,
    pub compatibility_ui: CompatibilityUiState,
    pub raw_mode: RawModeState,
    pub raw_request_generation: u64,
    pub pty_selection: usize,
    pub pane_layout: PaneLayout,
    pub terminal: TerminalWorkbenchState,
    pub detached_terminal: DetachedTerminalAvailability,
    pub keymap_preferences: KeymapPreferences,
    pub effective_keymap: EffectiveKeymap,
    pub keymap_chord: KeymapChordState,
    pub keymap_preferences_ui: KeymapPreferencesUiState,
    pub menu: MenuState,
    pub onboarding: OnboardingState,
    pub preferences: WorkbenchPreferences,
    pub screen: Screen,
    pub overview_view: OverviewView,
    pub focus: FocusTarget,
    pub focus_return: Option<FocusTarget>,
    pub workspace_subfocus: WorkspaceSubfocus,
    pub inspector_subfocus: InspectorSubfocus,
    pub zoomed_pane: Option<FocusTarget>,
    pub navigator_selection: usize,
    pub navigator_groups_expanded: [bool; NAVIGATOR_GROUPS.len()],
    pub backend: String,
    pub project_profile: ProjectProfileState,
    pub project_profile_selection: usize,
    pub build_environment: BuildEnvironmentState,
    pub build_environment_generation: u64,
    pub build_environment_draft: Option<BuildEnvironmentDraft>,
    pub environment_setup_generation: u64,
    pub available_images: Vec<String>,
    pub color_enabled: bool,
    pub color_forced_off: bool,
    pub theme: Theme,
    pub animation_speed: AnimationSpeed,
    pub reduced_motion: bool,
    pub settings_selection: usize,
    pub settings_dirty: bool,
    pub animation_frame: u64,
    pub workspace: Workspace,
    pub host_telemetry: HostTelemetry,
    pub host_telemetry_history: TelemetryHistory,
    pub build: BuildState,
    pub background_jobs: BackgroundJobs,
    pub build_history: VecDeque<BuildRecord>,
    pub build_history_selection: usize,
    pub dependencies: Option<RecipeDependencies>,
    pub dependency_selection: usize,
    pub dependency_graph: DependencyGraphState,
    pub dependency_graph_selection: Option<DependencyNodeId>,
    pub dependency_graph_anchor: Option<DependencyNodeId>,
    pub dependency_graph_reverse: bool,
    pub dependency_graph_query: String,
    pub dependency_graph_searching: bool,
    pub dependency_graph_collapsed: BTreeSet<DependencyNodeId>,
    pub signature_dump: SignatureDumpState,
    pub signature_selection: Option<SignatureIdentity>,
    pub signature_comparison: SignatureComparisonState,
    pub signature_recipe: Option<RecipeIdentity>,
    pub package_inventory: PackageInventoryState,
    pub package_selection: Option<PackageIdentity>,
    pub package_details: HashMap<PackageIdentity, PackageDetailState>,
    pub package_query: String,
    pub package_searching: bool,
    pub package_request_generation: u64,
    pub package_dependency_reverse: bool,
    pub package_dependency_selection: usize,
    pub package_navigation: Vec<PackageIdentity>,
    pub image_artifacts: ImageArtifactInventoryState,
    pub image_artifact_selection: Option<ImageArtifactIdentity>,
    pub image_artifact_query: String,
    pub image_artifact_searching: bool,
    pub image_artifact_request_generation: u64,
    pub images_view: ImagesView,
    pub kernel: PlatformWorkbench,
    pub firmware: PlatformWorkbench,
    pub rootfs_composition: RootfsCompositionState,
    pub overview_image_size_history: VecDeque<OverviewImageSizeSnapshot>,
    pub rootfs_request_generation: u64,
    pub rootfs_group_selection: Option<RootfsGroupIdentity>,
    pub rootfs_package_selection: Option<PackageIdentity>,
    pub rootfs_entry_selection: Option<RootfsPathIdentity>,
    pub rootfs_systemd_selection: usize,
    pub rootfs_dbus_selection: usize,
    pub rootfs_udev_selection: usize,
    pub rootfs_udev_preview_offset: usize,
    pub sdk_artifacts: SdkArtifactInventoryState,
    pub sdk_artifact_selection: Option<SdkArtifactIdentity>,
    pub sdk_artifact_query: String,
    pub sdk_artifact_searching: bool,
    pub sdk_artifact_generation: u64,
    pub sdk_tool_capability: SdkToolCapability,
    pub sdk_sessions: VecDeque<SdkSession>,
    pub sdk_session_generation: u64,
    pub test_capability: TestCapability,
    pub test_family_selection: TestFamily,
    pub test_sessions: VecDeque<TestSession>,
    pub test_session_generation: u64,
    pub result_tool_capability: ResultToolCapability,
    pub test_view: TestWorkspaceView,
    pub test_results: TestResultInventoryState,
    pub test_result_selection: Option<TestResultIdentity>,
    pub test_result_query: String,
    pub test_result_searching: bool,
    pub test_result_drilled: bool,
    pub test_case_selection: Option<TestCaseIdentity>,
    pub test_result_generation: u64,
    pub test_comparison: TestComparisonState,
    pub test_comparison_selection: Option<TestCaseIdentity>,
    pub test_comparison_generation: u64,
    pub test_junit_export: TestJunitExportState,
    pub test_junit_generation: u64,
    pub security: SecurityState,
    pub qa: QaState,
    pub maintenance: MaintenanceState,
    pub qemu_capability: QemuCapability,
    pub ssh_client_capability: SshClientCapability,
    pub qemu_sessions: VecDeque<QemuSession>,
    pub qemu_session_generation: u64,
    pub wic_capability: WicCapability,
    pub wic_outputs: WicOutputInventoryState,
    pub wic_output_selection: Option<WicOutputIdentity>,
    pub wic_output_generation: u64,
    pub wic_devices: WicDeviceInventoryState,
    pub wic_device_selection: Option<WicDeviceIdentity>,
    pub wic_device_generation: u64,
    pub wic_sessions: VecDeque<WicSession>,
    pub wic_session_generation: u64,
    pub layer_relationships: Option<LayerRelationships>,
    pub recipe_sources: HashMap<String, Vec<PathBuf>>,
    pub recipe_metadata: HashMap<String, RecipeMetadata>,
    pub devtool_statuses: HashMap<RecipeIdentity, DevtoolStatus>,
    pub devtool_status_loading: HashSet<RecipeIdentity>,
    pub variable_details: HashMap<VariableIdentity, VariableDetail>,
    pub variable_detail_loading: HashSet<VariableIdentity>,
    pub variable_detail_errors: HashMap<VariableIdentity, String>,
    pub recipe_metadata_loading: HashSet<String>,
    pub recipe_metadata_errors: HashMap<String, String>,
    pub layer_browser: Option<LayerBrowser>,
    pub dialogs: VecDeque<Dialog>,
    pub tasks: HashMap<TaskId, TaskInfo>,
    pub completed_tasks: VecDeque<CompletedTask>,
    pub task_progress_scroll: usize,
    pub task_filters: TaskFilters,
    pub task_filter_field: TaskFilterField,
    pub task_filter_editing: bool,
    pub task_projection_generation: u64,
    pub task_projection_cache: TaskProjectionCacheCell,
    pub task_progress_events: u64,
    pub task_progress_coalesced: u64,
    pub task_active_overflow: u64,
    pub log_workspace_view: LogWorkspaceView,
    pub logs: LogState,
    pub internal_logs: InternalLogState,
    pub should_quit: bool,
    pub notification: Option<String>,
    pub command_palette_open: bool,
    pub command_palette_mode: CommandPaletteMode,
    pub command_palette_selection: usize,
    pub command_palette_query: String,
    pub global_search_generation: u64,
    pub global_search_content: GlobalSearchContentState,
    pub error_selection: usize,
    pub recipe_selection: usize,
    pub recipe_preview_scroll: usize,
    pub layer_selection: usize,
    pub config_selection: usize,
    pub config_scope: Option<String>,
    pub metadata_query: String,
    pub metadata_searching: bool,
}

/// Return a bounded, selection-centered viewport without retaining derived UI
/// state. Recomputing this range from authoritative selection and row counts
/// makes query/inventory invalidation immediate and stale-state-free.
pub fn centered_viewport_range(
    selected: Option<usize>,
    total: usize,
    visible: usize,
) -> std::ops::Range<usize> {
    if total == 0 || visible == 0 {
        return 0..0;
    }
    let visible = visible.min(total);
    let selected = selected.unwrap_or(0).min(total - 1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(total.saturating_sub(visible));
    start..start + visible
}
impl App {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            source_git_status: SourceGitStatus::default(),
            background_activities: Default::default(),
            daemon: ClientDaemonView::default(),
            client_access_origin: ClientAccessOrigin::default(),
            workspace_compatibility: WorkspaceCompatibilityState::default(),
            compatibility_ui: CompatibilityUiState::default(),
            raw_mode: RawModeState::new(builtin_raw_catalog()),
            raw_request_generation: 0,
            pty_selection: 0,
            pane_layout: PaneLayout::new(PaneId(1)).expect("valid root pane"),
            terminal: TerminalWorkbenchState::default(),
            detached_terminal: DetachedTerminalAvailability::default(),
            keymap_preferences: KeymapPreferences::default(),
            effective_keymap: EffectiveKeymap::default(),
            keymap_chord: KeymapChordState::default(),
            keymap_preferences_ui: KeymapPreferencesUiState::default(),
            menu: MenuState::default(),
            onboarding: OnboardingState::default(),
            preferences: WorkbenchPreferences::default(),
            screen: Screen::Dashboard,
            overview_view: OverviewView::default(),
            focus: FocusTarget::Navigator,
            focus_return: None,
            workspace_subfocus: WorkspaceSubfocus::Main,
            inspector_subfocus: InspectorSubfocus::Facts,
            zoomed_pane: None,
            navigator_selection: 0,
            navigator_groups_expanded: [true; NAVIGATOR_GROUPS.len()],
            backend: "unknown".into(),
            project_profile: ProjectProfileState::NotLoaded,
            project_profile_selection: 0,
            environment_setup_generation: 0,
            build_environment: BuildEnvironmentState::Connected(BuildEnvironmentProfile {
                source_dir: PathBuf::from("/"),
                build_dir: PathBuf::from("/"),
                init_script: PathBuf::from("/"),
            }),
            build_environment_generation: 0,
            build_environment_draft: None,
            available_images: Vec::new(),
            color_enabled: true,
            color_forced_off: false,
            theme: Theme::DarkPro,
            animation_speed: AnimationSpeed::Fast,
            reduced_motion: false,
            settings_selection: 0,
            settings_dirty: false,
            animation_frame: 0,
            workspace: Workspace::default(),
            host_telemetry: HostTelemetry::default(),
            host_telemetry_history: TelemetryHistory::default(),
            build: BuildState::default(),
            background_jobs: BackgroundJobs::default(),
            build_history: VecDeque::new(),
            build_history_selection: 0,
            dependencies: None,
            dependency_selection: 0,
            dependency_graph: DependencyGraphState::NotLoaded,
            dependency_graph_selection: None,
            dependency_graph_anchor: None,
            dependency_graph_reverse: false,
            dependency_graph_query: String::new(),
            dependency_graph_searching: false,
            dependency_graph_collapsed: BTreeSet::new(),
            signature_dump: SignatureDumpState::NotLoaded,
            signature_selection: None,
            signature_comparison: SignatureComparisonState::NotSelected,
            signature_recipe: None,
            package_inventory: PackageInventoryState::NotLoaded,
            package_selection: None,
            package_details: HashMap::new(),
            package_query: String::new(),
            package_searching: false,
            package_request_generation: 0,
            package_dependency_reverse: false,
            package_dependency_selection: 0,
            package_navigation: Vec::new(),
            image_artifacts: ImageArtifactInventoryState::NotLoaded,
            image_artifact_selection: None,
            image_artifact_query: String::new(),
            image_artifact_searching: false,
            image_artifact_request_generation: 0,
            images_view: ImagesView::Artifacts,
            kernel: PlatformWorkbench::default(),
            firmware: PlatformWorkbench::default(),
            rootfs_composition: RootfsCompositionState::NotLoaded,
            overview_image_size_history: VecDeque::new(),
            rootfs_request_generation: 0,
            rootfs_group_selection: None,
            rootfs_package_selection: None,
            rootfs_entry_selection: None,
            rootfs_systemd_selection: 0,
            rootfs_dbus_selection: 0,
            rootfs_udev_selection: 0,
            rootfs_udev_preview_offset: 0,
            sdk_artifacts: SdkArtifactInventoryState::NotLoaded,
            sdk_artifact_selection: None,
            sdk_artifact_query: String::new(),
            sdk_artifact_searching: false,
            sdk_artifact_generation: 0,
            sdk_tool_capability: SdkToolCapability::NotInspected,
            sdk_sessions: VecDeque::new(),
            sdk_session_generation: 0,
            test_capability: TestCapability::default(),
            test_family_selection: TestFamily::OeSelftest,
            test_sessions: VecDeque::new(),
            test_session_generation: 0,
            result_tool_capability: ResultToolCapability::default(),
            test_view: TestWorkspaceView::Launches,
            test_results: TestResultInventoryState::default(),
            test_result_selection: None,
            test_result_query: String::new(),
            test_result_searching: false,
            test_result_drilled: false,
            test_case_selection: None,
            test_result_generation: 0,
            test_comparison: TestComparisonState::default(),
            test_comparison_selection: None,
            test_comparison_generation: 0,
            test_junit_export: TestJunitExportState::default(),
            test_junit_generation: 0,
            security: SecurityState::default(),
            qa: QaState::default(),
            maintenance: MaintenanceState::default(),
            qemu_capability: QemuCapability::default(),
            ssh_client_capability: SshClientCapability::default(),
            qemu_sessions: VecDeque::new(),
            qemu_session_generation: 0,
            wic_capability: WicCapability::default(),
            wic_outputs: WicOutputInventoryState::default(),
            wic_output_selection: None,
            wic_output_generation: 0,
            wic_devices: WicDeviceInventoryState::default(),
            wic_device_selection: None,
            wic_device_generation: 0,
            wic_sessions: VecDeque::new(),
            wic_session_generation: 0,
            layer_relationships: None,
            recipe_sources: HashMap::new(),
            recipe_metadata: HashMap::new(),
            devtool_statuses: HashMap::new(),
            devtool_status_loading: HashSet::new(),
            variable_details: HashMap::new(),
            variable_detail_loading: HashSet::new(),
            variable_detail_errors: HashMap::new(),
            recipe_metadata_loading: HashSet::new(),
            recipe_metadata_errors: HashMap::new(),
            layer_browser: None,
            dialogs: VecDeque::new(),
            tasks: HashMap::new(),
            completed_tasks: VecDeque::new(),
            task_progress_scroll: 0,
            task_filters: TaskFilters::default(),
            task_filter_field: TaskFilterField::default(),
            task_filter_editing: false,
            task_projection_generation: 0,
            task_projection_cache: TaskProjectionCacheCell::default(),
            task_progress_events: 0,
            task_progress_coalesced: 0,
            task_active_overflow: 0,
            log_workspace_view: LogWorkspaceView::BitBake,
            logs: LogState::new(max_entries, max_bytes),
            internal_logs: InternalLogState::new(max_entries, max_bytes),
            should_quit: false,
            notification: None,
            command_palette_open: false,
            command_palette_mode: CommandPaletteMode::Commands,
            command_palette_selection: 0,
            command_palette_query: String::new(),
            global_search_generation: 0,
            global_search_content: GlobalSearchContentState::Idle,
            error_selection: 0,
            recipe_selection: 0,
            recipe_preview_scroll: 0,
            layer_selection: 0,
            config_selection: 0,
            config_scope: None,
            metadata_query: String::new(),
            metadata_searching: false,
        }
    }
    pub fn new_unconfigured(max_entries: usize, max_bytes: usize) -> Self {
        let mut app = Self::new(max_entries, max_bytes);
        app.build_environment = BuildEnvironmentState::Unconfigured;
        app.screen = Screen::BuildEnvironment;
        app.focus = FocusTarget::Navigator;
        app
    }
    pub fn install_keymap(&mut self, preferences: KeymapPreferences) -> Result<(), KeymapError> {
        let preferences = preferences.migrate()?;
        let effective = EffectiveKeymap::from_preferences(&preferences)?;
        self.keymap_preferences = preferences;
        self.effective_keymap = effective;
        self.keymap_chord.clear();
        Ok(())
    }
    pub fn reset_keymap(&mut self) {
        self.keymap_preferences = KeymapPreferences::default();
        self.effective_keymap = EffectiveKeymap::default();
        self.keymap_chord.clear();
    }
    pub fn effective_keymap_report(&self) -> String {
        self.effective_keymap.report()
    }
    pub fn elapsed(&self) -> Option<Duration> {
        self.build
            .started
            .and_then(|s| SystemTime::now().duration_since(s).ok())
    }
    pub fn navigator_screen(&self) -> Screen {
        NAVIGATOR_SCREENS
            .get(self.navigator_selection)
            .copied()
            .unwrap_or(Screen::Dashboard)
    }
    pub fn navigator_compatibility_destination(&self) -> WorkspaceDestination {
        NAVIGATOR_COMPATIBILITY_DESTINATIONS
            .get(self.navigator_selection)
            .copied()
            .unwrap_or(WorkspaceDestination::Dashboard)
    }
    pub fn navigator_group_index(&self) -> usize {
        navigator_group_for_selection(self.navigator_selection)
    }
    pub fn navigator_visible_row_count(&self) -> usize {
        NAVIGATOR_GROUPS.len()
            + NAVIGATOR_GROUPS
                .iter()
                .enumerate()
                .filter(|(index, _)| self.navigator_groups_expanded[*index])
                .map(|(_, group)| group.end - group.start)
                .sum::<usize>()
    }
    pub fn navigator_visual_row(&self) -> usize {
        let selected_group = self.navigator_group_index();
        let rows_before = NAVIGATOR_GROUPS
            .iter()
            .enumerate()
            .take(selected_group)
            .map(|(index, group)| {
                1 + usize::from(self.navigator_groups_expanded[index]) * (group.end - group.start)
            })
            .sum::<usize>();
        if self.navigator_groups_expanded[selected_group] {
            rows_before
                + 1
                + self
                    .navigator_selection
                    .saturating_sub(NAVIGATOR_GROUPS[selected_group].start)
        } else {
            rows_before
        }
    }
    pub fn navigator_selection_at_visual_row(&self, visual_row: usize) -> Option<usize> {
        let mut cursor = 0;
        for (group_index, group) in NAVIGATOR_GROUPS.iter().enumerate() {
            if visual_row == cursor {
                return None;
            }
            cursor += 1;
            if !self.navigator_groups_expanded[group_index] {
                continue;
            }
            let end = cursor + group.end - group.start;
            if visual_row < end {
                return Some(group.start + visual_row - cursor);
            }
            cursor = end;
        }
        None
    }
    pub fn navigator_group_at_visual_row(&self, visual_row: usize) -> Option<usize> {
        let mut cursor = 0;
        for (group_index, group) in NAVIGATOR_GROUPS.iter().enumerate() {
            if visual_row == cursor {
                return Some(group_index);
            }
            cursor += 1;
            if self.navigator_groups_expanded[group_index] {
                cursor += group.end - group.start;
            }
        }
        None
    }
    pub fn navigator_viewport_start(&self, visible_rows: usize) -> usize {
        self.navigator_visual_row()
            .saturating_sub(visible_rows.saturating_sub(1))
            .min(
                self.navigator_visible_row_count()
                    .saturating_sub(visible_rows),
            )
    }
    pub(crate) fn navigator_selection_is_visible(&self, selection: usize) -> bool {
        let group = navigator_group_for_selection(selection);
        self.navigator_groups_expanded[group] || selection == NAVIGATOR_GROUPS[group].start
    }
    pub fn waiting_task_count(&self) -> usize {
        if matches!(
            self.build.status,
            BuildStatus::Completed
                | BuildStatus::Cancelled
                | BuildStatus::Failed
                | BuildStatus::Lost
        ) {
            return 0;
        }
        self.build.total.map_or(0, |total| {
            total.saturating_sub(self.build.completed.saturating_add(self.tasks.len()))
        })
    }
    /// Count observed active workers only when one complete identity namespace is available.
    pub fn active_worker_count(&self) -> Option<usize> {
        if self.daemon.status != ClientReplicaStatus::Current {
            return None;
        }
        match self.build.status {
            BuildStatus::Idle
            | BuildStatus::Completed
            | BuildStatus::Cancelled
            | BuildStatus::Failed => return Some(0),
            BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Lost => {
                return None;
            }
            BuildStatus::Running | BuildStatus::Cancelling => {}
        }
        let mut pids = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        let mut complete_pids = true;
        let mut complete_labels = true;
        let mut observed_active = false;
        for task in self
            .tasks
            .values()
            .filter(|task| task.state == TaskState::Active)
        {
            observed_active = true;
            if let Some(pid) = task.pid.filter(|pid| *pid > 0) {
                pids.insert(pid);
            } else {
                complete_pids = false;
            }
            if let Some(label) = task
                .worker
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                labels.insert(label);
            } else {
                complete_labels = false;
            }
        }
        if !observed_active {
            None
        } else if complete_pids {
            Some(pids.len())
        } else if complete_labels {
            Some(labels.len())
        } else {
            None
        }
    }

    pub fn build_summary_at(&self, now: SystemTime) -> BuildSummary {
        let elapsed = match self.build.status {
            BuildStatus::Completed | BuildStatus::Cancelled | BuildStatus::Failed => {
                self.build_history.back().and_then(|record| record.elapsed)
            }
            BuildStatus::Lost => None,
            BuildStatus::Idle
            | BuildStatus::LoadingWorkspace
            | BuildStatus::Parsing
            | BuildStatus::Running
            | BuildStatus::Cancelling => self
                .build
                .started
                .and_then(|started| now.duration_since(started).ok()),
        };
        BuildSummary {
            completed: self.build.completed,
            total: self.build.total,
            active: self
                .tasks
                .values()
                .filter(|task| task.state == TaskState::Active)
                .count(),
            waiting: self.waiting_task_count(),
            warnings: self.build.warnings,
            errors: self.build.errors,
            elapsed,
        }
    }
    pub fn job_history_rows(&self) -> Vec<JobHistoryRowRef<'_>> {
        let daemon_current = self.daemon.status == ClientReplicaStatus::Current;
        let mut rows = Vec::new();
        if daemon_current {
            rows.extend(
                self.daemon
                    .jobs
                    .iter()
                    .rev()
                    .filter(|job| !job.lifecycle.is_terminal())
                    .map(JobHistoryRowRef::Daemon),
            );
        }
        rows.extend(
            self.background_jobs
                .jobs
                .iter()
                .rev()
                .filter(|job| {
                    !job.status.is_terminal()
                        && (!daemon_current
                            || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0))
                })
                .map(JobHistoryRowRef::Background),
        );
        if daemon_current {
            rows.extend(
                self.daemon
                    .jobs
                    .iter()
                    .rev()
                    .filter(|job| job.lifecycle.is_terminal())
                    .map(JobHistoryRowRef::Daemon),
            );
        }
        rows.extend(
            self.background_jobs
                .jobs
                .iter()
                .rev()
                .filter(|job| {
                    job.status.is_terminal()
                        && (!daemon_current
                            || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0))
                })
                .map(JobHistoryRowRef::Background),
        );
        let mut unmatched_daemon_builds = if daemon_current {
            self.daemon
                .jobs
                .iter()
                .filter(|job| {
                    job.kind == ClientDaemonJobKind::BitBakeBuild && job.lifecycle.is_terminal()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for record in self.build_history.iter().rev() {
            let duplicate = unmatched_daemon_builds.iter().position(|job| {
                let Some(target) = record.target.as_deref() else {
                    return false;
                };
                let daemon_target = job
                    .label
                    .strip_prefix("BitBake build ")
                    .unwrap_or(job.label.as_str());
                let daemon_success = job.lifecycle == ClientDaemonLifecycle::Exited
                    && job.exit_code.is_none_or(|exit_code| exit_code == 0);
                daemon_target == target && daemon_success == record.success
            });
            if let Some(index) = duplicate {
                unmatched_daemon_builds.remove(index);
            } else {
                rows.push(JobHistoryRowRef::Build(record));
            }
        }
        rows
    }
    pub fn job_summary(&self) -> JobSummary {
        let mut summary = JobSummary {
            daemon_owned: (self.daemon.status == ClientReplicaStatus::Current)
                .then_some(self.daemon.jobs.len()),
            ..JobSummary::default()
        };
        if self.daemon.status == ClientReplicaStatus::Current {
            for job in &self.daemon.jobs {
                match job.lifecycle {
                    ClientDaemonLifecycle::Connecting => summary.queued += 1,
                    ClientDaemonLifecycle::Running | ClientDaemonLifecycle::Stopping => {
                        summary.active += 1
                    }
                    ClientDaemonLifecycle::Failed => {
                        summary.failed += 1;
                        summary.recent_completed += 1;
                    }
                    ClientDaemonLifecycle::Exited
                    | ClientDaemonLifecycle::Lost
                    | ClientDaemonLifecycle::Disconnected => summary.recent_completed += 1,
                }
            }
        }
        for job in self.background_jobs.jobs.iter().filter(|job| {
            self.daemon.status != ClientReplicaStatus::Current
                || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0)
        }) {
            match job.status {
                BackgroundJobStatus::Queued => summary.queued += 1,
                BackgroundJobStatus::Starting
                | BackgroundJobStatus::Running
                | BackgroundJobStatus::Cancelling => summary.active += 1,
                BackgroundJobStatus::Failed => {
                    summary.failed += 1;
                    summary.recent_completed += 1;
                }
                BackgroundJobStatus::Succeeded
                | BackgroundJobStatus::Cancelled
                | BackgroundJobStatus::Lost => summary.recent_completed += 1,
            }
        }
        summary
    }
    pub fn transient_status(&self) -> Option<TransientStatus> {
        if let Some(activity) = self.background_activities.first() {
            return Some(TransientStatus {
                kind: TransientStatusKind::Activity,
                text: activity.label().into(),
            });
        }

        let notification = self
            .notification
            .as_deref()
            .map(str::trim)
            .filter(|message| !message.is_empty());

        let notification_kind = notification.map(|message| {
            let logged_severity = self
                .logs
                .entries
                .iter()
                .rev()
                .find(|entry| entry.message == message)
                .map(|entry| entry.severity);
            match logged_severity {
                Some(Severity::Error) => TransientStatusKind::Error,
                Some(Severity::Warning) => TransientStatusKind::Warning,
                _ if self.build.status == BuildStatus::Failed
                    && message.starts_with("Build failed with ") =>
                {
                    TransientStatusKind::Error
                }
                _ if self.build.status == BuildStatus::Completed
                    && message.starts_with("Build completed with ") =>
                {
                    TransientStatusKind::Warning
                }
                _ if self.build.status == BuildStatus::Completed
                    && message == "Build completed successfully with no errors." =>
                {
                    TransientStatusKind::Success
                }
                _ if self.build.status == BuildStatus::Cancelled
                    && message == "Build was cancelled; this is distinct from a build failure." =>
                {
                    TransientStatusKind::Warning
                }
                _ => TransientStatusKind::Notification,
            }
        });
        if notification_kind == Some(TransientStatusKind::Error) {
            return Some(TransientStatus {
                kind: TransientStatusKind::Error,
                text: notification
                    .expect("an error kind requires notification text")
                    .to_owned(),
            });
        }
        if self.active_dialog().is_some_and(Dialog::is_confirmation) {
            return Some(TransientStatus {
                kind: TransientStatusKind::Confirmation,
                text: "Confirmation pending".into(),
            });
        }
        if let Some(message) = notification {
            return Some(TransientStatus {
                kind: notification_kind
                    .expect("notification text always has a projected semantic kind"),
                text: message.to_owned(),
            });
        }

        match self.daemon.status {
            ClientReplicaStatus::Synchronizing => {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Reconnecting,
                    text: "Daemon synchronizing".into(),
                });
            }
            ClientReplicaStatus::Stale => {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Warning,
                    text: "Daemon state stale".into(),
                });
            }
            ClientReplicaStatus::Current
                if self.daemon.bitbake == ClientDaemonLifecycle::Connecting =>
            {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Reconnecting,
                    text: "BitBake connecting".into(),
                });
            }
            ClientReplicaStatus::Disconnected | ClientReplicaStatus::Current => {}
        }

        let text = match self.build.status {
            BuildStatus::LoadingWorkspace => Some("Workspace loading".to_owned()),
            BuildStatus::Parsing => Some("BitBake parsing".to_owned()),
            BuildStatus::Running => {
                let active = self
                    .tasks
                    .values()
                    .filter(|task| task.state == TaskState::Active)
                    .count();
                let queued = self.job_summary().queued;
                Some(if queued > 0 {
                    format!("Build running · {active} active · {queued} queued")
                } else {
                    format!("Build running · {active} active")
                })
            }
            BuildStatus::Cancelling => Some("Build cancellation pending".to_owned()),
            BuildStatus::Idle
            | BuildStatus::Completed
            | BuildStatus::Cancelled
            | BuildStatus::Failed
            | BuildStatus::Lost => None,
        };
        if let Some(text) = text {
            return Some(TransientStatus {
                kind: TransientStatusKind::Activity,
                text,
            });
        }
        let jobs = self.job_summary();
        (jobs.active > 0 || jobs.queued > 0).then(|| TransientStatus {
            kind: TransientStatusKind::Activity,
            text: format!("{} active jobs · {} queued", jobs.active, jobs.queued),
        })
    }
    pub fn inspector_mode(&self) -> InspectorMode {
        if self.focus == FocusTarget::Navigator {
            return InspectorMode::Navigator;
        }
        match self.screen {
            Screen::Dashboard | Screen::Insights => InspectorMode::DaemonSession,
            Screen::Tasks => InspectorMode::Task,
            Screen::BuildHistory => InspectorMode::Job,
            Screen::Dependencies | Screen::LayerRelationships => InspectorMode::Dependency,
            Screen::Signatures => InspectorMode::Signature,
            Screen::Recipes => InspectorMode::Recipe,
            Screen::Packages => InspectorMode::Package,
            Screen::Images | Screen::Kernel | Screen::Firmware | Screen::Sdk => {
                InspectorMode::Artifact
            }
            Screen::Testing => InspectorMode::Test,
            Screen::Security => InspectorMode::Security,
            Screen::Qa => InspectorMode::Qa,
            Screen::Layers => {
                if self
                    .layer_browser
                    .as_ref()
                    .and_then(LayerBrowser::selected_entry)
                    .is_some_and(|entry| !entry.is_dir)
                {
                    InspectorMode::File
                } else {
                    InspectorMode::Layer
                }
            }
            Screen::Configuration | Screen::Bbmask => InspectorMode::Configuration,
            Screen::RawMode => InspectorMode::RawCommand,
            Screen::TerminalSessions => InspectorMode::DaemonSession,
            Screen::Maintenance => InspectorMode::Utility,
            Screen::Logs => InspectorMode::Log,
            Screen::Errors => InspectorMode::Error,
            Screen::Help => InspectorMode::Help,
            Screen::BuildEnvironment => InspectorMode::BuildEnvironment,
            Screen::Compatibility => InspectorMode::CompatibilityCapability,
            Screen::Settings => InspectorMode::Settings,
        }
    }
    pub fn task_inspector<'a>(
        &'a self,
        row: Option<TaskRowRef<'a>>,
        recent_log_limit: usize,
    ) -> TaskInspectorRef<'a> {
        match row {
            None => TaskInspectorRef::None,
            Some(TaskRowRef::WaitingSummary(count)) => TaskInspectorRef::Waiting { count },
            Some(TaskRowRef::Task { task, state }) => {
                let version = self
                    .workspace
                    .recipes
                    .iter()
                    .find(|recipe| recipe.name == task.recipe)
                    .and_then(|recipe| recipe.version.as_deref());
                let mut recent_logs = self
                    .logs
                    .entries
                    .iter()
                    .rev()
                    .filter(|entry| {
                        entry.recipe.as_deref() == Some(task.recipe.as_str())
                            && entry.task.as_deref() == Some(task.task.as_str())
                    })
                    .take(recent_log_limit)
                    .collect::<Vec<_>>();
                recent_logs.reverse();
                TaskInspectorRef::Task {
                    task,
                    state,
                    version,
                    revision: None,
                    workdir: None,
                    recent_logs,
                }
            }
        }
    }
    pub fn invalidate_task_projection(&mut self) {
        self.task_projection_generation = self.task_projection_generation.wrapping_add(1);
    }
    pub fn task_projection_rebuilds(&self) -> u64 {
        self.task_projection_cache.0.borrow().rebuilds
    }
    pub fn visible_task_row_refs_at(&self, now: SystemTime) -> Vec<TaskRowRef<'_>> {
        let duration_second = self.task_filters.minimum_duration.map(|_| {
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        let waiting = self.waiting_task_count();
        let cache_current = {
            let cache = self.task_projection_cache.0.borrow();
            cache.generation == self.task_projection_generation
                && cache.filters == self.task_filters
                && cache.duration_second == duration_second
                && cache.active_len == self.tasks.len()
                && cache.completed_len == self.completed_tasks.len()
                && cache.waiting == waiting
        };
        if !cache_current {
            self.rebuild_task_projection(now, duration_second, waiting);
        }
        let cache = self.task_projection_cache.0.borrow();
        cache
            .rows
            .iter()
            .filter_map(|key| match key {
                TaskProjectionKey::Active(id, state) => {
                    self.tasks.get(id).map(|task| TaskRowRef::Task {
                        task,
                        state: *state,
                    })
                }
                TaskProjectionKey::Completed(index, state) => {
                    self.completed_tasks
                        .get(*index)
                        .map(|completed| TaskRowRef::Task {
                            task: &completed.task,
                            state: *state,
                        })
                }
                TaskProjectionKey::WaitingSummary(count) => {
                    Some(TaskRowRef::WaitingSummary(*count))
                }
            })
            .collect()
    }
    pub(crate) fn rebuild_task_projection(
        &self,
        now: SystemTime,
        duration_second: Option<u64>,
        waiting: usize,
    ) {
        let state_matches = |state: TaskState| match self.task_filters.state {
            TaskStateFilter::All => true,
            TaskStateFilter::Active => state == TaskState::Active,
            TaskStateFilter::Waiting => matches!(state, TaskState::Queued | TaskState::Waiting),
            TaskStateFilter::Completed => state == TaskState::Completed,
            TaskStateFilter::Failed => {
                matches!(
                    state,
                    TaskState::Failed | TaskState::Cancelled | TaskState::Lost
                )
            }
        };
        let text_matches = |task: &TaskInfo| {
            contains_case_insensitive(&task.recipe, &self.task_filters.recipe)
                && contains_case_insensitive(&task.task, &self.task_filters.task)
                && contains_case_insensitive(
                    task.worker.as_deref().unwrap_or(""),
                    &self.task_filters.worker,
                )
                && self.task_filters.minimum_duration.is_none_or(|minimum| {
                    task.elapsed_at(now)
                        .is_some_and(|elapsed| elapsed >= minimum)
                })
        };
        let retained = self
            .tasks
            .values()
            .map(|task| {
                (
                    TaskProjectionKey::Active(task.id.clone(), task.state),
                    task,
                    task.state,
                )
            })
            .chain(
                self.completed_tasks
                    .iter()
                    .enumerate()
                    .map(|(index, completed)| {
                        let state = if matches!(
                            completed.task.state,
                            TaskState::Queued | TaskState::Active | TaskState::Waiting
                        ) {
                            if completed.success {
                                TaskState::Completed
                            } else {
                                TaskState::Failed
                            }
                        } else {
                            completed.task.state
                        };
                        (
                            TaskProjectionKey::Completed(index, state),
                            &completed.task,
                            state,
                        )
                    }),
            );
        let mut rows = retained
            .filter(|(_, task, state)| state_matches(*state) && text_matches(task))
            .collect::<Vec<_>>();
        rows.sort_unstable_by(|left, right| {
            let (_, left, left_state) = left;
            let (_, right, right_state) = right;
            (
                left.started.is_none(),
                left.started,
                task_state_order(*left_state),
                left.recipe.as_str(),
                left.task.as_str(),
                left.id.0.as_str(),
            )
                .cmp(&(
                    right.started.is_none(),
                    right.started,
                    task_state_order(*right_state),
                    right.recipe.as_str(),
                    right.task.as_str(),
                    right.id.0.as_str(),
                ))
        });
        let waiting_filter_matches = matches!(
            self.task_filters.state,
            TaskStateFilter::All | TaskStateFilter::Waiting
        ) && self.task_filters.recipe.is_empty()
            && self.task_filters.task.is_empty()
            && self.task_filters.worker.is_empty()
            && self.task_filters.minimum_duration.is_none();
        let mut projection = rows.into_iter().map(|(key, _, _)| key).collect::<Vec<_>>();
        if waiting > 0 && waiting_filter_matches {
            projection.push(TaskProjectionKey::WaitingSummary(waiting));
        }
        let mut cache = self.task_projection_cache.0.borrow_mut();
        cache.generation = self.task_projection_generation;
        cache.filters = self.task_filters.clone();
        cache.duration_second = duration_second;
        cache.active_len = self.tasks.len();
        cache.completed_len = self.completed_tasks.len();
        cache.waiting = waiting;
        cache.rows = projection;
        cache.rebuilds = cache.rebuilds.saturating_add(1);
    }
    pub fn visible_task_rows(&self) -> Vec<TaskRow> {
        self.visible_task_row_refs_at(SystemTime::now())
            .into_iter()
            .map(TaskRowRef::into_owned)
            .collect()
    }
    pub fn selected_task_row(&self) -> Option<TaskRow> {
        self.visible_task_row_refs_at(SystemTime::now())
            .get(self.task_progress_scroll)
            .copied()
            .map(TaskRowRef::into_owned)
    }
    pub fn filtered_packages(&self) -> Vec<&PackageSummary> {
        let query = self.package_query.to_ascii_lowercase();
        self.package_inventory
            .packages()
            .unwrap_or_default()
            .iter()
            .filter(|package| {
                query.is_empty()
                    || [
                        Some(package.identity.name.as_str()),
                        package.recipe.available().map(String::as_str),
                        package.version.available().map(String::as_str),
                        package.license.available().map(String::as_str),
                        package.provider.available().and_then(|path| path.to_str()),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_package(&self) -> Option<&PackageSummary> {
        let selected = self.package_selection.as_ref()?;
        self.filtered_packages()
            .into_iter()
            .find(|package| &package.identity == selected)
    }
    pub fn selected_package_detail(&self) -> Option<&PackageDetailState> {
        self.package_selection
            .as_ref()
            .and_then(|identity| self.package_details.get(identity))
    }
    pub fn selected_package_dependencies(&self) -> Option<&[PackageIdentity]> {
        let detail = self.selected_package_detail()?.detail()?;
        if self.package_dependency_reverse {
            detail.reverse_dependencies.available().map(Vec::as_slice)
        } else {
            detail.runtime_dependencies.available().map(Vec::as_slice)
        }
    }
    pub fn selected_package_dependency(&self) -> Option<&PackageIdentity> {
        self.selected_package_dependencies()?
            .get(self.package_dependency_selection)
    }
    pub fn filtered_image_artifacts(&self) -> Vec<&ImageArtifact> {
        self.image_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.image_artifact_query))
            .collect()
    }
    pub fn selected_image_artifact(&self) -> Option<&ImageArtifact> {
        let selected = self.image_artifact_selection.as_ref()?;
        self.filtered_image_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn filtered_sdk_artifacts(&self) -> Vec<&SdkArtifact> {
        self.sdk_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.sdk_artifact_query))
            .collect()
    }
    pub fn selected_sdk_artifact(&self) -> Option<&SdkArtifact> {
        let selected = self.sdk_artifact_selection.as_ref()?;
        self.filtered_sdk_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn sdk_session(&self, id: SdkSessionId) -> Option<&SdkSession> {
        self.sdk_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.back()
    }
    pub fn test_session(&self, id: TestSessionId) -> Option<&TestSession> {
        self.test_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.iter().rev().find(|session| {
            session.background_job_id.is_none()
                || session.background_job_id.is_some_and(|job_id| {
                    self.background_jobs
                        .get(job_id)
                        .is_some_and(|job| !job.status.is_terminal())
                })
        })
    }
    pub fn latest_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.back()
    }
    pub fn filtered_test_results(&self) -> Vec<&TestResultRecord> {
        let query = self.test_result_query.to_ascii_lowercase();
        self.test_results
            .records()
            .iter()
            .filter(|record| {
                query.is_empty()
                    || [
                        record.identity.path.to_str(),
                        Some(record.identity.fingerprint.as_str()),
                        record.machine.as_deref(),
                        record.image.as_deref(),
                        record.revision.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .chain(
                        record
                            .metadata
                            .iter()
                            .flat_map(|entry| [entry.key.as_str(), entry.value.as_str()]),
                    )
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_test_result(&self) -> Option<&TestResultRecord> {
        let selected = self.test_result_selection.as_ref()?;
        self.filtered_test_results()
            .into_iter()
            .find(|record| &record.identity == selected)
    }
    pub fn selected_test_case(&self) -> Option<&TestCaseRecord> {
        let identity = self.test_case_selection.as_ref()?;
        self.selected_test_result()?.case(identity)
    }
    pub fn test_comparison_transitions(&self) -> &[TestCaseTransition] {
        match &self.test_comparison {
            TestComparisonState::Available { comparison, .. }
            | TestComparisonState::Partial { comparison, .. } => &comparison.transitions,
            _ => &[],
        }
    }
    pub fn selected_test_transition(&self) -> Option<&TestCaseTransition> {
        let selected = self.test_comparison_selection.as_ref()?;
        self.test_comparison_transitions()
            .iter()
            .find(|transition| &transition.identity == selected)
    }
    pub fn qemu_session(&self, id: QemuSessionId) -> Option<&QemuSession> {
        self.qemu_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.back()
    }
    pub fn wic_session(&self, id: WicSessionId) -> Option<&WicSession> {
        self.wic_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.back()
    }
    pub fn wic_output_rows(&self) -> &[WicOutput] {
        match &self.wic_outputs {
            WicOutputInventoryState::Available { outputs, .. }
            | WicOutputInventoryState::Partial { outputs, .. } => outputs,
            _ => &[],
        }
    }
    pub fn selected_wic_output(&self) -> Option<&WicOutput> {
        let selected = self.wic_output_selection.as_ref()?;
        self.wic_output_rows()
            .iter()
            .find(|output| &output.identity == selected)
    }
    pub fn wic_device_rows(&self) -> &[WicDevice] {
        match &self.wic_devices {
            WicDeviceInventoryState::Available { devices, .. }
            | WicDeviceInventoryState::Partial { devices, .. } => devices,
            _ => &[],
        }
    }
    pub fn selected_wic_device(&self) -> Option<&WicDevice> {
        let selected = self.wic_device_selection.as_ref()?;
        self.wic_device_rows()
            .iter()
            .find(|device| &device.identity == selected)
    }
    pub fn selected_wic_write_image(&self) -> Result<WicOutputIdentity, String> {
        if self.wic_output_selection.is_some() {
            let output = self
                .selected_wic_output()
                .ok_or_else(|| "The selected generated Wic output is stale.".to_owned())?;
            if !matches!(output.kind, WicOutputKind::Wic | WicOutputKind::Direct)
                || !is_uncompressed_wic_path(&output.identity.path)
            {
                return Err("Select an uncompressed generated .wic or .direct image first.".into());
            }
            return Ok(output.identity.clone());
        }
        let artifact = self
            .selected_image_artifact()
            .ok_or_else(|| "Select a deployed Wic or generated Wic output first.".to_owned())?;
        if artifact.kind != ImageArtifactKind::Wic
            || !is_uncompressed_wic_path(&artifact.identity.path)
        {
            return Err("Select an uncompressed deployed .wic or .direct image first.".into());
        }
        let size_bytes = artifact
            .size_bytes
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image size is unavailable.".to_owned())?;
        let modified_unix_seconds = artifact
            .modified_unix_seconds
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image timestamp is unavailable.".to_owned())?;
        Ok(WicOutputIdentity {
            path: artifact.identity.path.clone(),
            size_bytes,
            modified_unix_seconds,
        })
    }
    pub fn wic_device_write_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        match &self.wic_capability {
            WicCapability::Available { executable, .. }
            | WicCapability::MissingKickstarts { executable }
                if executable.is_absolute() => {}
            WicCapability::NotInspected => {
                return Some("Wic capability has not been inspected.".into());
            }
            WicCapability::MissingTool => return Some("wic is not available.".into()),
            WicCapability::Failed { message } => {
                return Some(format!("Wic capability inspection failed: {message}"));
            }
            WicCapability::Available { .. } | WicCapability::MissingKickstarts { .. } => {
                return Some("The inspected Wic executable identity is invalid.".into());
            }
        }
        self.selected_wic_write_image().err()
    }
    pub fn wic_create_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        let WicCapability::Available {
            kickstarts,
            image_targets,
            ..
        } = &self.wic_capability
        else {
            return Some(match &self.wic_capability {
                WicCapability::NotInspected => "Wic capability has not been inspected.".into(),
                WicCapability::MissingTool => "wic is not available.".into(),
                WicCapability::MissingKickstarts { .. } => {
                    "No Wic kickstarts are available.".into()
                }
                WicCapability::Failed { message } => {
                    format!("Wic capability inspection failed: {message}")
                }
                WicCapability::Available { .. } => unreachable!(),
            });
        };
        if kickstarts.is_empty()
            || !image_targets
                .iter()
                .any(|target| target == &artifact.identity.image)
        {
            return Some("The selected image is not in the inspected Wic capability.".into());
        }
        None
    }
    pub fn qemu_launch_unavailable_reason(&self) -> Option<String> {
        if self.active_qemu_session().is_some() {
            return Some("A managed runqemu session is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        if !matches!(
            artifact.kind,
            ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
        ) {
            return Some("runqemu requires a root filesystem or Wic artifact.".into());
        }
        match &self.qemu_capability {
            QemuCapability::NotInspected => {
                Some("runqemu capability has not been inspected.".into())
            }
            QemuCapability::MissingTool => Some("runqemu is not available.".into()),
            QemuCapability::MissingCompatibleImage => {
                Some("No compatible deployed runqemu image is available.".into())
            }
            QemuCapability::Failed { message } => {
                Some(format!("runqemu capability inspection failed: {message}"))
            }
            QemuCapability::Available {
                executable: _,
                compatible_images,
            } if !compatible_images.contains(&artifact.identity) => {
                Some("The selected artifact is not in the inspected runqemu capability.".into())
            }
            QemuCapability::Available { .. } => None,
        }
    }
    pub fn active_dialog(&self) -> Option<&Dialog> {
        self.dialogs.front()
    }
    pub fn active_dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialogs.front_mut()
    }
    pub fn command_palette_commands(&self) -> Vec<PaletteCommand> {
        global_operator_action_definitions()
            .into_iter()
            .map(|definition| {
                let OperatorActionTarget::Command(id) = definition.target else {
                    unreachable!("global catalog entries target command IDs")
                };
                let pane_disabled_reason = match id {
                    CommandId::FocusWorkspace
                        if !focus_target_is_relevant(self, FocusTarget::Workspace) =>
                    {
                        Some("The current Workspace is read-only")
                    }
                    CommandId::FocusInspector => Some("The Inspector is read-only"),
                    _ => None,
                };
                let local_disabled_reason =
                    pane_disabled_reason.or(match definition.local_requirement {
                        OperatorActionLocalRequirement::None => None,
                        OperatorActionLocalRequirement::WorkspaceLoaded => self
                            .workspace
                            .build_dir
                            .is_none()
                            .then_some("Load a Yocto workspace first"),
                        OperatorActionLocalRequirement::ImageRecipeAvailable => (!self
                            .workspace
                            .recipes
                            .iter()
                            .any(|recipe| recipe.name.contains("image")))
                        .then_some("No image recipes are available"),
                        OperatorActionLocalRequirement::SelectedRecipe => (self.screen
                            != Screen::Recipes
                            || self.workspace.recipes.get(self.recipe_selection).is_none())
                        .then_some("Open Recipes and select a recipe"),
                    });
                let compatibility =
                    compatibility_ui_command_action_availability(&self.workspace_compatibility, id);
                let compatibility_reason = compatibility.exact_reason();
                let disabled_reason = local_disabled_reason.map(str::to_owned).or_else(|| {
                    (!compatibility.enabled).then(|| {
                        compatibility_reason.clone().unwrap_or_else(|| {
                            "The connected environment does not enable this operation.".into()
                        })
                    })
                });
                PaletteCommand {
                    action_id: definition.id,
                    id,
                    label: definition.label,
                    description: definition.description,
                    shortcut: definition.shortcut,
                    menu_path: definition.menu_path,
                    aliases: definition.aliases,
                    palette_keywords: definition.palette_keywords,
                    safety: definition.safety,
                    footer_priority: definition.footer_priority,
                    help_group: definition.help_group,
                    disabled_reason,
                    compatibility_state: compatibility.state,
                    compatibility_reason,
                    compatibility_limitations: compatibility.limitations,
                    implementations: compatibility.implementations,
                }
            })
            .collect()
    }
    pub fn filtered_command_palette_commands(&self) -> Vec<PaletteCommand> {
        let query = self.command_palette_query.trim();
        if self.command_palette_mode == CommandPaletteMode::GlobalRegexSearch {
            return Vec::new();
        }
        let literal_query = query.to_lowercase();
        self.command_palette_commands()
            .into_iter()
            .filter(|command| {
                let values = std::iter::once(command.label)
                    .chain(std::iter::once(command.description.as_str()))
                    .chain(std::iter::once(command.shortcut))
                    .chain(std::iter::once(command.action_id.as_str()))
                    .chain(command.menu_path.iter().copied())
                    .chain(command.aliases.iter().copied())
                    .chain(command.palette_keywords.iter().copied());
                query.is_empty()
                    || values
                        .into_iter()
                        .any(|value| value.to_lowercase().contains(&literal_query))
            })
            .collect()
    }
    pub fn command_palette_regex_error(&self) -> Option<String> {
        let query = self.command_palette_query.trim();
        (self.command_palette_mode == CommandPaletteMode::GlobalRegexSearch && !query.is_empty())
            .then(|| {
                regex::RegexBuilder::new(query)
                    .case_insensitive(true)
                    .build()
                    .err()
                    .map(|error| error.to_string())
            })
            .flatten()
    }
    pub fn application_menu_items(&self, group: ApplicationMenuGroup) -> Vec<MenuItem> {
        let mut items = self
            .command_palette_commands()
            .into_iter()
            .filter(|command| ApplicationMenuGroup::for_command(command.id) == group)
            .map(|command| MenuItem {
                action_id: command.action_id,
                target: OperatorActionTarget::Command(command.id),
                label: command.label,
                description: command.description,
                shortcut: command.shortcut,
                disabled_reason: command.disabled_reason,
                safety: command.safety,
            })
            .collect::<Vec<_>>();
        if group == ApplicationMenuGroup::Build
            && let Some(definition) =
                workspace_operator_action_definitions(WorkspaceDestination::Tasks)
                    .into_iter()
                    .find(|definition| definition.id.as_str() == "tasks.cancel")
        {
            let availability = compatibility_ui_action_availability(
                &self.workspace_compatibility,
                &definition.requirement,
            );
            let disabled_reason = context_action_local_disabled_reason(
                self,
                WorkspaceDestination::Tasks,
                definition.id.as_str(),
            )
            .or_else(|| {
                (!availability.enabled).then(|| {
                    availability.exact_reason().unwrap_or_else(|| {
                        "The connected environment does not enable this operation.".into()
                    })
                })
            });
            items.push(MenuItem {
                action_id: definition.id,
                target: definition.target,
                label: definition.label,
                description: definition.description,
                shortcut: definition.shortcut,
                disabled_reason,
                safety: definition.safety,
            });
        }
        items
    }
    pub fn context_menu_items(&self, destination: WorkspaceDestination) -> Vec<MenuItem> {
        workspace_operator_action_definitions(destination)
            .into_iter()
            .map(|definition| {
                let availability = compatibility_ui_action_availability(
                    &self.workspace_compatibility,
                    &definition.requirement,
                );
                let disabled_reason =
                    context_action_local_disabled_reason(self, destination, definition.id.as_str())
                        .or_else(|| {
                            (!availability.enabled).then(|| {
                                availability.exact_reason().unwrap_or_else(|| {
                                    "The connected environment does not enable this operation."
                                        .into()
                                })
                            })
                        });
                MenuItem {
                    action_id: definition.id,
                    target: definition.target,
                    label: definition.label,
                    description: definition.description,
                    shortcut: definition.shortcut,
                    disabled_reason,
                    safety: definition.safety,
                }
            })
            .collect()
    }
    pub fn active_menu_items(&self) -> Vec<MenuItem> {
        match self.menu.kind {
            Some(MenuKind::Application) => self.application_menu_items(self.menu.group()),
            Some(MenuKind::Context(destination)) => self.context_menu_items(destination),
            None => Vec::new(),
        }
    }
    pub fn selected_menu_item(&self) -> Option<MenuItem> {
        self.active_menu_items()
            .get(self.menu.item_selection)
            .cloned()
    }
    pub fn pane_focus_label(&self) -> &'static str {
        pane_focus_label(self.focus, self.workspace_subfocus, self.inspector_subfocus)
    }
    pub fn zoom_label(&self) -> Option<&'static str> {
        self.zoomed_pane
            .map(|focus| pane_focus_label(focus, self.workspace_subfocus, self.inspector_subfocus))
    }

    pub fn selected_terminal_session(&self) -> Option<&ClientDaemonPtySummary> {
        self.daemon.pty_sessions.get(self.pty_selection)
    }

    pub fn selected_terminal_screen(&self) -> Option<&ClientDaemonPtyScreen> {
        let id = self.selected_terminal_session()?.id;
        self.daemon
            .pty_screens
            .iter()
            .find(|screen| screen.session_id == id)
    }

    pub fn selected_terminal_details(&self) -> Option<&ClientDaemonPtyDetails> {
        let id = self.selected_terminal_session()?.id;
        self.daemon
            .pty_details
            .iter()
            .find(|details| details.id == id)
    }

    pub fn selected_terminal_is_writer(&self) -> bool {
        self.daemon.status == ClientReplicaStatus::Current
            && self.selected_terminal_details().is_some_and(|details| {
                self.terminal.client_id.is_some() && details.writer == self.terminal.client_id
            })
    }

    pub fn selected_terminal_is_menuconfig(&self) -> bool {
        self.selected_terminal_details()
            .is_some_and(|details| details.kind == ClientDaemonPtyKind::Menuconfig)
    }
}

pub(crate) fn context_action_local_disabled_reason(
    app: &App,
    destination: WorkspaceDestination,
    action_id: &str,
) -> Option<String> {
    let workspace_required = matches!(
        destination,
        WorkspaceDestination::Dashboard
            | WorkspaceDestination::Recipes
            | WorkspaceDestination::Layers
            | WorkspaceDestination::Configuration
            | WorkspaceDestination::Tasks
            | WorkspaceDestination::Dependencies
            | WorkspaceDestination::Signatures
            | WorkspaceDestination::Packages
            | WorkspaceDestination::Images
            | WorkspaceDestination::Sdk
            | WorkspaceDestination::Testing
            | WorkspaceDestination::Security
            | WorkspaceDestination::Qa
            | WorkspaceDestination::RawMode
            | WorkspaceDestination::Devtool
            | WorkspaceDestination::QemuWic
            | WorkspaceDestination::Maintenance
    );
    if workspace_required && app.workspace.build_dir.is_none() {
        return Some("Load a Yocto workspace first.".into());
    }
    if destination == WorkspaceDestination::Recipes
        && action_id != "recipes.metadata"
        && app.workspace.recipes.get(app.recipe_selection).is_none()
    {
        return Some("Select a recipe first.".into());
    }
    if destination == WorkspaceDestination::Packages
        && action_id != "packages.inventory"
        && action_id != "packages.cancel"
        && app.selected_package().is_none()
    {
        return Some("Select a package first.".into());
    }
    if destination == WorkspaceDestination::Images
        && matches!(
            action_id,
            "images.build" | "images.qemu" | "images.wic" | "images.device_write"
        )
        && app.selected_image_artifact().is_none()
    {
        return Some("Select a deployed image artifact first.".into());
    }
    if matches!(action_id, "dashboard.cancel" | "tasks.cancel")
        && !matches!(
            app.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        )
    {
        return Some("No active build is available to cancel.".into());
    }
    None
}
pub(crate) fn contains_case_insensitive(value: &str, query: &str) -> bool {
    query.is_empty() || value.to_lowercase().contains(&query.to_lowercase())
}
pub(crate) fn task_state_order(state: TaskState) -> u8 {
    match state {
        TaskState::Active => 0,
        TaskState::Queued => 1,
        TaskState::Waiting => 2,
        TaskState::Failed | TaskState::Cancelled | TaskState::Lost => 3,
        TaskState::Completed => 4,
    }
}

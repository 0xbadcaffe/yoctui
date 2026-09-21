#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPtySessionMetadata {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub lifecycle: DaemonPtyLifecycle,
    pub restartable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonPtyLifecycle {
    Running,
    Exited,
    Lost,
}

/// Existing typed long-lived workflow state captured without UI selection,
/// focus, dialogs, editors, search queries, or other client presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonJobState {
    pub build: BuildState,
    pub background_jobs: BackgroundJobs,
    pub build_history: VecDeque<BuildRecord>,
    pub active_tasks: HashMap<TaskId, TaskInfo>,
    pub completed_tasks: VecDeque<CompletedTask>,
    pub logs: LogState,
    pub signature_dump: SignatureDumpState,
    pub signature_comparison: SignatureComparisonState,
    pub package_inventory: PackageInventoryState,
    pub package_details: HashMap<PackageIdentity, PackageDetailState>,
    pub image_artifacts: ImageArtifactInventoryState,
    pub rootfs_composition: RootfsCompositionState,
    pub sdk_artifacts: SdkArtifactInventoryState,
    pub sdk_tool_capability: SdkToolCapability,
    pub sdk_sessions: VecDeque<SdkSession>,
    pub test_capability: TestCapability,
    pub test_sessions: VecDeque<TestSession>,
    pub test_results: TestResultInventoryState,
    pub test_comparison: TestComparisonState,
    pub test_junit_export: TestJunitExportState,
    pub security: SecurityState,
    pub qa: QaState,
    pub maintenance: MaintenanceState,
    pub qemu_capability: QemuCapability,
    pub qemu_sessions: VecDeque<QemuSession>,
    pub wic_capability: WicCapability,
    pub wic_outputs: WicOutputInventoryState,
    pub wic_devices: WicDeviceInventoryState,
    pub wic_sessions: VecDeque<WicSession>,
    pub pty_sessions: VecDeque<DaemonPtySessionMetadata>,
}

impl DaemonJobState {
    pub fn capture(app: &App) -> Self {
        Self {
            build: app.build.clone(),
            background_jobs: app.background_jobs.clone(),
            build_history: app.build_history.clone(),
            active_tasks: app.tasks.clone(),
            completed_tasks: app.completed_tasks.clone(),
            logs: app.logs.clone(),
            signature_dump: app.signature_dump.clone(),
            signature_comparison: app.signature_comparison.clone(),
            package_inventory: app.package_inventory.clone(),
            package_details: app.package_details.clone(),
            image_artifacts: app.image_artifacts.clone(),
            rootfs_composition: app.rootfs_composition.clone(),
            sdk_artifacts: app.sdk_artifacts.clone(),
            sdk_tool_capability: app.sdk_tool_capability.clone(),
            sdk_sessions: app.sdk_sessions.clone(),
            test_capability: app.test_capability.clone(),
            test_sessions: app.test_sessions.clone(),
            test_results: app.test_results.clone(),
            test_comparison: app.test_comparison.clone(),
            test_junit_export: app.test_junit_export.clone(),
            security: app.security.clone(),
            qa: app.qa.clone(),
            maintenance: app.maintenance.clone(),
            qemu_capability: app.qemu_capability.clone(),
            qemu_sessions: app.qemu_sessions.clone(),
            wic_capability: app.wic_capability.clone(),
            wic_outputs: app.wic_outputs.clone(),
            wic_devices: app.wic_devices.clone(),
            wic_sessions: app.wic_sessions.clone(),
            pty_sessions: VecDeque::new(),
        }
    }

    pub fn install_replica(&self, app: &mut App) {
        app.build = self.build.clone();
        app.background_jobs = self.background_jobs.clone();
        app.build_history = self.build_history.clone();
        app.tasks = self.active_tasks.clone();
        app.completed_tasks = self.completed_tasks.clone();
        app.logs = self.logs.clone();
        app.signature_dump = self.signature_dump.clone();
        app.signature_comparison = self.signature_comparison.clone();
        app.package_inventory = self.package_inventory.clone();
        app.package_details = self.package_details.clone();
        app.image_artifacts = self.image_artifacts.clone();
        app.rootfs_composition = self.rootfs_composition.clone();
        app.sdk_artifacts = self.sdk_artifacts.clone();
        app.sdk_tool_capability = self.sdk_tool_capability.clone();
        app.sdk_sessions = self.sdk_sessions.clone();
        app.test_capability = self.test_capability.clone();
        app.test_sessions = self.test_sessions.clone();
        app.test_results = self.test_results.clone();
        app.test_comparison = self.test_comparison.clone();
        app.test_junit_export = self.test_junit_export.clone();
        app.security = self.security.clone();
        app.qa = self.qa.clone();
        app.maintenance = self.maintenance.clone();
        app.qemu_capability = self.qemu_capability.clone();
        app.qemu_sessions = self.qemu_sessions.clone();
        app.wic_capability = self.wic_capability.clone();
        app.wic_outputs = self.wic_outputs.clone();
        app.wic_devices = self.wic_devices.clone();
        app.wic_sessions = self.wic_sessions.clone();
    }
}

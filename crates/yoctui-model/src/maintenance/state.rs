#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceSessionStatus {
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl MaintenanceSessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::TimedOut | Self::Lost
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceEvidence {
    pub identity: MaintenanceFileIdentity,
    pub label: String,
}

impl MaintenanceEvidence {
    pub fn new(identity: MaintenanceFileIdentity, label: String) -> Result<Self, &'static str> {
        if !identity.is_valid() || !bounded_text(&label) {
            return Err("Maintenance evidence is invalid");
        }
        Ok(Self { identity, label })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceSession {
    pub id: MaintenanceSessionId,
    pub preview: MaintenanceOperationPreview,
    pub status: MaintenanceSessionStatus,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub output: VecDeque<MaintenanceOutputLine>,
    pub dropped_lines: usize,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
}

impl MaintenanceSession {
    fn append_output(&mut self, stream: MaintenanceOutputStream, text: String) {
        if !bounded_text(&text) {
            return;
        }
        if self.output.len() == MAX_MAINTENANCE_OUTPUT {
            self.output.pop_front();
            self.dropped_lines = self.dropped_lines.saturating_add(1);
        }
        self.output
            .push_back(MaintenanceOutputLine { stream, text });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceDialog {
    ReadinessToml {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    ReadinessForm(Box<MaintenanceReadinessDraft>),
    CleanupToml {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    CleanupForm(Box<MaintenanceCleanupDraft>),
    PrServiceToml {
        operation: PrServiceOperation,
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    PrServiceForm(Box<MaintenancePrServiceDraft>),
    LockedCacheToml {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    LockedCacheForm(Box<MaintenanceLockedCacheDraft>),
    BuildHistoryToml {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    BuildHistoryForm(Box<MaintenanceBuildHistoryDraft>),
    GitArchiveToml {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    GitArchiveForm(Box<MaintenanceGitArchiveDraft>),
    Confirm(MaintenanceOperationPreview),
    CleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmNetworkPush(MaintenanceOperationPreview),
    ConfirmCancellation(MaintenanceSessionId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceState {
    pub view: MaintenanceView,
    pub selections: [usize; 4],
    pub capability: MaintenanceCapability,
    pub services: MaintenanceServiceDiagnostics,
    pub integrations: MaintenanceIntegrationDiagnostics,
    pub pending: Option<MaintenanceOperationPreview>,
    pub sessions: VecDeque<MaintenanceSession>,
    pub evidence: Vec<MaintenanceEvidence>,
    pub evidence_selection: usize,
    pub capability_generation: u64,
    pub service_generation: u64,
}

impl Default for MaintenanceState {
    fn default() -> Self {
        Self {
            view: MaintenanceView::Sstate,
            selections: [0; 4],
            capability: MaintenanceCapability::NotInspected,
            services: MaintenanceServiceDiagnostics::NotInspected,
            integrations: MaintenanceIntegrationDiagnostics::NotInspected,
            pending: None,
            sessions: VecDeque::new(),
            evidence: Vec::new(),
            evidence_selection: 0,
            capability_generation: 0,
            service_generation: 0,
        }
    }
}

impl MaintenanceState {
    pub fn selection(&self) -> usize {
        self.selections[self.view.index()]
    }

    pub fn active_session(&self) -> Option<&MaintenanceSession> {
        self.sessions
            .iter()
            .rev()
            .find(|session| !session.status.is_terminal())
    }

    pub fn selected_evidence(&self) -> Option<&MaintenanceEvidence> {
        self.evidence.get(self.evidence_selection)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceAction {
    CycleView {
        backwards: bool,
    },
    Select {
        delta: isize,
        row_count: usize,
    },
    InspectCapability,
    CapabilityLoaded {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
        partial: bool,
    },
    CapabilityFailed {
        request: u64,
        message: String,
    },
    IntegrationsLoaded {
        request: u64,
        snapshot: Box<MaintenanceIntegrationsSnapshot>,
        partial: bool,
    },
    IntegrationsFailed {
        request: u64,
        message: String,
    },
    InspectServices,
    ServicesLoaded {
        request: u64,
        services: Vec<ServiceDiagnostic>,
        limitations: Vec<String>,
    },
    ServicesFailed {
        request: u64,
        message: String,
    },
    OpenReadinessForm,
    ConfirmReadinessToml(String),
    UpdateReadinessForm(Box<MaintenanceReadinessDraft>),
    ConfirmReadinessForm(Box<MaintenanceReadinessDraft>),
    OpenCleanupForm,
    ConfirmCleanupToml(String),
    UpdateCleanupForm(Box<MaintenanceCleanupDraft>),
    ConfirmCleanupForm(Box<MaintenanceCleanupDraft>),
    OpenPrServiceForm(PrServiceOperation),
    ConfirmPrServiceToml {
        operation: PrServiceOperation,
        document: String,
    },
    UpdatePrServiceForm(Box<MaintenancePrServiceDraft>),
    ConfirmPrServiceForm(Box<MaintenancePrServiceDraft>),
    OpenLockedCacheForm,
    ConfirmLockedCacheToml(String),
    UpdateLockedCacheForm(Box<MaintenanceLockedCacheDraft>),
    ConfirmLockedCacheForm(Box<MaintenanceLockedCacheDraft>),
    OpenBuildHistoryForm,
    ConfirmBuildHistoryToml(String),
    UpdateBuildHistoryForm(Box<MaintenanceBuildHistoryDraft>),
    ConfirmBuildHistoryForm(Box<MaintenanceBuildHistoryDraft>),
    OpenGitArchiveForm,
    ConfirmGitArchiveToml(String),
    UpdateGitArchiveForm(Box<MaintenanceGitArchiveDraft>),
    ConfirmGitArchiveForm(Box<MaintenanceGitArchiveDraft>),
    BeginOperation(MaintenanceOperationPreview),
    UpdateCleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmCleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmOperation(MaintenanceOperationPreview),
    ConfirmNetworkPush(MaintenanceOperationPreview),
    CancelDialog,
    SessionRunning {
        id: MaintenanceSessionId,
        started_at: SystemTime,
    },
    SessionOutput {
        id: MaintenanceSessionId,
        stream: MaintenanceOutputStream,
        text: String,
    },
    CompleteSession {
        id: MaintenanceSessionId,
        exit_code: i32,
        evidence: Vec<MaintenanceEvidence>,
        finished_at: SystemTime,
    },
    FailSession {
        id: MaintenanceSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    TimeoutSession {
        id: MaintenanceSessionId,
        finished_at: SystemTime,
    },
    LoseSession {
        id: MaintenanceSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginCancellation,
    ConfirmCancellation(MaintenanceSessionId),
    RejectCancellation {
        id: MaintenanceSessionId,
        message: String,
    },
    CancelSession {
        id: MaintenanceSessionId,
        finished_at: SystemTime,
    },
    SelectEvidence(isize),
    OpenSelectedEvidence,
    OpenSignatures,
    OpenSecurity,
    OpenQa,
    OpenRecipes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceEffect {
    InspectCapability {
        request: u64,
    },
    InspectServices {
        request: u64,
    },
    PreviewReadiness {
        capability_request: u64,
        request: SstateReadinessRequest,
    },
    PreviewCleanup {
        capability_request: u64,
        request: SstateCleanupRequest,
    },
    PreviewPrService {
        capability_request: u64,
        request: PrServiceRequest,
    },
    PreviewLockedSignatureCache {
        capability_request: u64,
        request: LockedSignatureCacheRequest,
    },
    PreviewBuildHistoryComparison {
        capability_request: u64,
        request: BuildComparisonRequest,
    },
    PreviewGitArchive {
        capability_request: u64,
        request: GitArchiveRequest,
    },
    StartOperation {
        id: MaintenanceSessionId,
        preview: Box<MaintenanceOperationPreview>,
    },
    CancelOperation(MaintenanceSessionId),
    OpenEvidence(MaintenanceFileIdentity),
    Navigate(Screen),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceDialogUpdate {
    None,
    Open(Box<MaintenanceDialog>),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceTransition {
    pub effect: Option<MaintenanceEffect>,
    pub dialog: MaintenanceDialogUpdate,
    pub notification: Option<String>,
}

impl MaintenanceTransition {
    fn none() -> Self {
        Self {
            effect: None,
            dialog: MaintenanceDialogUpdate::None,
            notification: None,
        }
    }

    fn effect(effect: MaintenanceEffect) -> Self {
        Self {
            effect: Some(effect),
            ..Self::none()
        }
    }
}

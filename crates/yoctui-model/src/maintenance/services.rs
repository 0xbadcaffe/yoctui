#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceKind {
    Pr,
    Hash,
    Worker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Disabled,
    Configured,
    Reachable,
    Unreachable,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceLocation {
    Local,
    Remote,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceEndpointRole {
    Primary,
    Upstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceReachability {
    NotProbed,
    Reachable,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEndpointDiagnostic {
    pub role: ServiceEndpointRole,
    pub value: String,
    pub location: ServiceLocation,
    pub reachability: ServiceReachability,
    pub limitation: Option<String>,
}

impl ServiceEndpointDiagnostic {
    pub fn new(
        role: ServiceEndpointRole,
        value: String,
        location: ServiceLocation,
        reachability: ServiceReachability,
        limitation: Option<String>,
    ) -> Result<Self, &'static str> {
        if !bounded_text(&value)
            || limitation
                .as_deref()
                .is_some_and(|message| !bounded_text(message))
        {
            return Err("Maintenance service endpoint diagnostic is invalid");
        }
        Ok(Self {
            role,
            value,
            location,
            reachability,
            limitation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceProcessEvidence {
    pub pid: u32,
    pub executable: String,
}

impl ServiceProcessEvidence {
    pub fn new(pid: u32, executable: String) -> Result<Self, &'static str> {
        if pid == 0 || !bounded_token(&executable) {
            return Err("Maintenance service process evidence is invalid");
        }
        Ok(Self { pid, executable })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDiagnostic {
    pub kind: ServiceKind,
    pub state: ServiceState,
    pub endpoints: Vec<ServiceEndpointDiagnostic>,
    pub process_evidence: Vec<ServiceProcessEvidence>,
    pub limitations: Vec<String>,
}

impl ServiceDiagnostic {
    pub fn new(
        kind: ServiceKind,
        state: ServiceState,
        mut endpoints: Vec<ServiceEndpointDiagnostic>,
        mut process_evidence: Vec<ServiceProcessEvidence>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if endpoints.iter().any(|endpoint| {
            ServiceEndpointDiagnostic::new(
                endpoint.role,
                endpoint.value.clone(),
                endpoint.location,
                endpoint.reachability,
                endpoint.limitation.clone(),
            )
            .as_ref()
                != Ok(endpoint)
        }) || process_evidence.iter().any(|process| {
            ServiceProcessEvidence::new(process.pid, process.executable.clone()).as_ref()
                != Ok(process)
        }) {
            return Err("Maintenance service diagnostic is invalid");
        }
        endpoints.sort_by(|left, right| {
            left.role
                .cmp(&right.role)
                .then(left.value.cmp(&right.value))
        });
        endpoints.dedup_by(|left, right| left.role == right.role && left.value == right.value);
        endpoints.truncate(MAX_MAINTENANCE_PATHS);
        process_evidence.sort();
        process_evidence.dedup();
        process_evidence.truncate(MAX_MAINTENANCE_OUTPUT);
        Ok(Self {
            kind,
            state,
            endpoints,
            process_evidence,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MaintenanceServiceDiagnostics {
    #[default]
    NotInspected,
    Loading(u64),
    Available {
        request: u64,
        services: Vec<ServiceDiagnostic>,
    },
    Partial {
        request: u64,
        services: Vec<ServiceDiagnostic>,
        limitations: Vec<String>,
    },
    Failed {
        request: u64,
        message: String,
    },
}

impl MaintenanceServiceDiagnostics {
    pub fn request(&self) -> Option<u64> {
        match self {
            Self::Loading(request)
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
            Self::NotInspected => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceDirectoryIdentity {
    pub path: PathBuf,
    pub modified_at: SystemTime,
}

impl MaintenanceDirectoryIdentity {
    pub fn new(path: PathBuf, modified_at: SystemTime) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) {
            return Err(
                "Maintenance directory identity must be a canonical absolute non-root path",
            );
        }
        Ok(Self { path, modified_at })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalIntegrationState {
    Available,
    Partial,
    Unavailable,
}

fn optional_state(parts: &[bool]) -> OptionalIntegrationState {
    if parts.iter().all(|present| *present) {
        OptionalIntegrationState::Available
    } else if parts.iter().any(|present| *present) {
        OptionalIntegrationState::Partial
    } else {
        OptionalIntegrationState::Unavailable
    }
}

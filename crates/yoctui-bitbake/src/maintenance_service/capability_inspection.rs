#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaintenanceServiceAdapterError {
    #[error("invalid Maintenance service input: {0}")]
    InvalidInput(String),
    #[error("unsafe Maintenance service path: {0}")]
    UnsafePath(PathBuf),
    #[error("Maintenance service process inspection failed: {0}")]
    ProcessInspection(String),
    #[error(transparent)]
    Runner(#[from] MaintenanceSstateAdapterError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceServiceCapabilityInput {
    pub build_dir: PathBuf,
    pub prserv_host: Option<String>,
    pub hashserve: Option<String>,
    pub hashserve_upstream: Option<String>,
    pub signature_handler: Option<String>,
    pub executable_search_path: Vec<PathBuf>,
    pub process_root: PathBuf,
    pub endpoint_probe_timeout: Duration,
    pub endpoint_observations: Vec<MaintenanceEndpointObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceEndpointObservation {
    pub endpoint: String,
    pub reachability: ServiceReachability,
}

impl MaintenanceEndpointObservation {
    pub fn new(
        endpoint: String,
        reachability: ServiceReachability,
    ) -> Result<Self, MaintenanceServiceAdapterError> {
        if endpoint.is_empty()
            || endpoint.len() > MAX_MAINTENANCE_TEXT_BYTES
            || endpoint.chars().any(char::is_control)
            || reachability == ServiceReachability::NotProbed
        {
            return Err(MaintenanceServiceAdapterError::InvalidInput(
                "endpoint observation is invalid".into(),
            ));
        }
        Ok(Self {
            endpoint,
            reachability,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceServiceInspection {
    pub capability: MaintenanceCapabilitySnapshot,
    pub services: Vec<ServiceDiagnostic>,
    pub limitations: Vec<String>,
}

pub struct MaintenanceServiceCapabilityInspector;

impl MaintenanceServiceCapabilityInspector {
    pub fn inspect(
        input: MaintenanceServiceCapabilityInput,
    ) -> Result<MaintenanceServiceInspection, MaintenanceServiceAdapterError> {
        if input.endpoint_probe_timeout.is_zero()
            || input.endpoint_probe_timeout > MAX_ENDPOINT_PROBE_TIMEOUT
        {
            return Err(MaintenanceServiceAdapterError::InvalidInput(
                "endpoint probe timeout must be between one nanosecond and five seconds".into(),
            ));
        }
        let build_dir = canonical_directory(&input.build_dir)?;
        let mut endpoint_observations = BTreeMap::new();
        for observation in input
            .endpoint_observations
            .into_iter()
            .take(MAX_MAINTENANCE_PATHS)
        {
            let observation = MaintenanceEndpointObservation::new(
                observation.endpoint,
                observation.reachability,
            )?;
            if endpoint_observations
                .insert(observation.endpoint, observation.reachability)
                .is_some()
            {
                return Err(MaintenanceServiceAdapterError::InvalidInput(
                    "duplicate endpoint observation".into(),
                ));
            }
        }
        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir),
            prserv_host: input.prserv_host.clone(),
            hashserve: input.hashserve.clone(),
            hashserve_upstream: input.hashserve_upstream.clone(),
            signature_handler: input.signature_handler.clone(),
            ..MaintenanceMetadata::default()
        })
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))?;

        let mut limitations = Vec::new();
        let pr_tool = discover_pr_service_tool(&input.executable_search_path, &mut limitations);
        let process_scan = scan_processes(&input.process_root);
        let (processes, process_limitations, process_available) = match process_scan {
            Ok(scan) => (scan.processes, scan.limitations, true),
            Err(error) => {
                push_limitation(&mut limitations, error.to_string());
                (BTreeMap::new(), vec![error.to_string()], false)
            }
        };
        for limitation in &process_limitations {
            push_limitation(&mut limitations, limitation.clone());
        }

        let pr_processes = processes.get(&ServiceKind::Pr).cloned().unwrap_or_default();
        let hash_processes = processes
            .get(&ServiceKind::Hash)
            .cloned()
            .unwrap_or_default();
        let worker_processes = processes
            .get(&ServiceKind::Worker)
            .cloned()
            .unwrap_or_default();
        let endpoint_context = EndpointInspectionContext {
            timeout: input.endpoint_probe_timeout,
            observations: &endpoint_observations,
        };

        let pr = configured_service(
            ServiceKind::Pr,
            input.prserv_host.as_deref(),
            None,
            pr_processes,
            process_available,
            false,
            endpoint_context,
        )?;
        let mut hash = configured_service(
            ServiceKind::Hash,
            input.hashserve.as_deref(),
            input.hashserve_upstream.as_deref(),
            hash_processes,
            process_available,
            true,
            endpoint_context,
        )?;
        if input.hashserve.is_some() && input.signature_handler.is_none() {
            push_limitation(
                &mut hash.limitations,
                "signature handler is unavailable; hash-equivalence use cannot be confirmed".into(),
            );
            if !matches!(
                hash.state,
                ServiceState::Disabled | ServiceState::Unavailable
            ) {
                hash.state = ServiceState::Partial;
            }
        }
        let worker_state = if process_available && !worker_processes.is_empty() {
            ServiceState::Configured
        } else {
            ServiceState::Unavailable
        };
        let worker_limitations = if !process_available {
            process_limitations.clone()
        } else if worker_processes.is_empty() {
            vec!["no bitbake-worker process was observed".into()]
        } else {
            vec!["bitbake-worker evidence is build context only".into()]
        };
        let worker = ServiceDiagnostic::new(
            ServiceKind::Worker,
            worker_state,
            Vec::new(),
            worker_processes,
            worker_limitations,
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))?;

        let services = vec![pr, hash, worker];
        let capability =
            MaintenanceCapabilitySnapshot::new(metadata, vec![pr_tool], limitations.clone())
                .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))?;
        Ok(MaintenanceServiceInspection {
            capability,
            services,
            limitations,
        })
    }
}

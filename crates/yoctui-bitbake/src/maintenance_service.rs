use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_OUTPUT, MAX_MAINTENANCE_PATHS,
    MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot, MaintenanceFileIdentity,
    MaintenanceMetadata, MaintenanceOperationPreview, MaintenanceSessionId, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, PrServiceRequest, ServiceDiagnostic,
    ServiceEndpointDiagnostic, ServiceEndpointRole, ServiceKind, ServiceLocation,
    ServiceProcessEvidence, ServiceReachability, ServiceState,
};

use crate::maintenance_sstate::{MaintenanceSstateAdapterError, MaintenanceSstateCommandSpec};

const MAX_PROCESS_ENTRIES: usize = 4_096;
const MAX_PROCESS_NAME_BYTES: usize = 256;
const MAX_ENDPOINT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

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

pub fn pr_service_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: PrServiceRequest,
) -> Result<
    (MaintenanceOperationPreview, MaintenanceSstateCommandSpec),
    MaintenanceServiceAdapterError,
> {
    MaintenanceSstateCommandSpec::pr_service(
        session,
        capability_request,
        snapshot,
        operation_id,
        request,
    )
    .map_err(Into::into)
}

fn discover_pr_service_tool(
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
    for directory in search_path.iter().take(MAX_MAINTENANCE_PATHS) {
        let Ok(directory) = canonical_directory(directory) else {
            push_limitation(
                limitations,
                format!("ignored unsafe tool directory {}", directory.display()),
            );
            continue;
        };
        let candidate = directory.join("bitbake-prserv-tool");
        match executable_identity(&candidate) {
            Ok(executable) => {
                return MaintenanceToolCapability::Available {
                    tool: MaintenanceTool::PrServiceTool,
                    executable,
                    interface: MaintenanceToolInterface::Native,
                };
            }
            Err(_) if candidate.exists() => push_limitation(
                limitations,
                format!("ignored unsafe executable {}", candidate.display()),
            ),
            Err(_) => {}
        }
    }
    MaintenanceToolCapability::Unavailable {
        tool: MaintenanceTool::PrServiceTool,
        reason: "bitbake-prserv-tool is unavailable in the configured child search path".into(),
    }
}

fn executable_identity(
    path: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceServiceAdapterError> {
    if path.file_name().and_then(|name| name.to_str()) != Some("bitbake-prserv-tool") {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
        }
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))
}

fn canonical_directory(path: &Path) -> Result<PathBuf, MaintenanceServiceAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

#[derive(Debug)]
struct ProcessScan {
    processes: BTreeMap<ServiceKind, Vec<ServiceProcessEvidence>>,
    limitations: Vec<String>,
}

fn scan_processes(process_root: &Path) -> Result<ProcessScan, MaintenanceServiceAdapterError> {
    let process_root = canonical_directory(process_root)?;
    let directory = fs::read_dir(&process_root)
        .map_err(|error| MaintenanceServiceAdapterError::ProcessInspection(error.to_string()))?;
    let mut entries = Vec::new();
    let mut limitations = Vec::new();
    for entry in directory.take(MAX_PROCESS_ENTRIES + 1) {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => push_limitation(
                &mut limitations,
                format!("one process entry could not be inspected: {error}"),
            ),
        }
    }
    if entries.len() > MAX_PROCESS_ENTRIES {
        entries.truncate(MAX_PROCESS_ENTRIES);
        push_limitation(
            &mut limitations,
            "process inspection reached the entry limit".into(),
        );
    }
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut processes = BTreeMap::<ServiceKind, Vec<ServiceProcessEvidence>>::new();
    for entry in entries {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => metadata,
            _ => continue,
        };
        let _ = metadata;
        let Some(executable) = read_process_name(&path) else {
            continue;
        };
        let kind = match executable.as_str() {
            "bitbake-prserv" => ServiceKind::Pr,
            "bitbake-hashserv" => ServiceKind::Hash,
            "bitbake-worker" => ServiceKind::Worker,
            _ => continue,
        };
        let evidence = ServiceProcessEvidence::new(pid, executable)
            .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))?;
        processes.entry(kind).or_default().push(evidence);
    }
    for values in processes.values_mut() {
        values.sort();
        values.dedup();
        values.truncate(MAX_MAINTENANCE_OUTPUT);
    }
    Ok(ProcessScan {
        processes,
        limitations,
    })
}

fn read_process_name(process_path: &Path) -> Option<String> {
    let comm = read_limited(&process_path.join("comm"), MAX_PROCESS_NAME_BYTES).ok()?;
    let comm = String::from_utf8_lossy(&comm).trim().to_string();
    if !comm.is_empty() {
        return Some(comm);
    }
    let command = read_limited(&process_path.join("cmdline"), MAX_PROCESS_NAME_BYTES).ok()?;
    let first = command.split(|byte| *byte == 0).next()?;
    Path::new(std::str::from_utf8(first).ok()?)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, std::io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "process evidence is not a regular non-symlink file",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "process evidence exceeded the byte limit",
        ));
    }
    Ok(bytes)
}

#[derive(Clone, Copy)]
struct EndpointInspectionContext<'a> {
    timeout: Duration,
    observations: &'a BTreeMap<String, ServiceReachability>,
}

fn configured_service(
    kind: ServiceKind,
    primary: Option<&str>,
    upstream: Option<&str>,
    process_evidence: Vec<ServiceProcessEvidence>,
    process_available: bool,
    allow_auto: bool,
    context: EndpointInspectionContext<'_>,
) -> Result<ServiceDiagnostic, MaintenanceServiceAdapterError> {
    let Some(primary) = primary else {
        let mut limitations = if process_available {
            Vec::new()
        } else {
            vec!["observational process evidence is unavailable".into()]
        };
        if !process_evidence.is_empty() {
            limitations.push(
                "the service is disabled in this build; observed processes may belong to another build"
                    .into(),
            );
        }
        return ServiceDiagnostic::new(
            kind,
            ServiceState::Disabled,
            Vec::new(),
            process_evidence,
            limitations,
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    };
    let mut endpoints = vec![inspect_endpoint(
        ServiceEndpointRole::Primary,
        primary,
        context.timeout,
        allow_auto,
        context.observations,
    )?];
    if let Some(upstream) = upstream {
        endpoints.push(inspect_endpoint(
            ServiceEndpointRole::Upstream,
            upstream,
            context.timeout,
            false,
            context.observations,
        )?);
    }
    let mut limitations = endpoints
        .iter()
        .filter_map(|endpoint| endpoint.limitation.clone())
        .collect::<Vec<_>>();
    if !process_available {
        push_limitation(
            &mut limitations,
            "observational process evidence is unavailable".into(),
        );
    } else if !process_evidence.is_empty() {
        push_limitation(
            &mut limitations,
            "process-name evidence is observational and does not prove endpoint health".into(),
        );
    }
    let primary_endpoint = &endpoints[0];
    let mut state = match primary_endpoint.reachability {
        ServiceReachability::Reachable => ServiceState::Reachable,
        ServiceReachability::Unreachable => ServiceState::Unreachable,
        ServiceReachability::NotProbed => {
            if primary_endpoint.location == ServiceLocation::Unknown {
                ServiceState::Partial
            } else {
                ServiceState::Configured
            }
        }
    };
    if endpoints.iter().skip(1).any(|endpoint| {
        endpoint.location == ServiceLocation::Unknown
            || endpoint.reachability == ServiceReachability::Unreachable
            || endpoint.limitation.is_some()
    }) || !process_available
    {
        state = ServiceState::Partial;
    }
    ServiceDiagnostic::new(kind, state, endpoints, process_evidence, limitations)
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))
}

fn inspect_endpoint(
    role: ServiceEndpointRole,
    value: &str,
    timeout: Duration,
    allow_auto: bool,
    observations: &BTreeMap<String, ServiceReachability>,
) -> Result<ServiceEndpointDiagnostic, MaintenanceServiceAdapterError> {
    if value.len() > MAX_MAINTENANCE_TEXT_BYTES
        || value.is_empty()
        || value.chars().any(char::is_control)
    {
        return Err(MaintenanceServiceAdapterError::InvalidInput(
            "service endpoint is invalid".into(),
        ));
    }
    if value.contains('@') {
        return ServiceEndpointDiagnostic::new(
            role,
            "<redacted endpoint>".into(),
            ServiceLocation::Unknown,
            ServiceReachability::NotProbed,
            Some("endpoint credentials were redacted and not probed".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    }
    if allow_auto && value == "auto" {
        return ServiceEndpointDiagnostic::new(
            role,
            value.into(),
            ServiceLocation::Local,
            ServiceReachability::NotProbed,
            Some("BitBake assigns the local auto-server endpoint at runtime".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    }
    if let Some(path) = value.strip_prefix("unix://") {
        return inspect_unix_endpoint(role, value, Path::new(path));
    }
    let authority = value
        .strip_prefix("ws://")
        .or_else(|| value.strip_prefix("wss://"))
        .map(|remainder| remainder.split('/').next().unwrap_or_default())
        .unwrap_or(value);
    let default_port = if value.starts_with("wss://") {
        Some(443)
    } else if value.starts_with("ws://") {
        Some(80)
    } else {
        None
    };
    let Some((host, port)) = split_host_port(authority, default_port) else {
        return ServiceEndpointDiagnostic::new(
            role,
            value.into(),
            ServiceLocation::Unknown,
            ServiceReachability::NotProbed,
            Some("endpoint format is unsupported".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    };
    let location = endpoint_location(host);
    if port == 0 {
        return ServiceEndpointDiagnostic::new(
            role,
            value.into(),
            location,
            ServiceReachability::NotProbed,
            Some("port 0 is assigned only when BitBake starts the local service".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    }
    if let Some(reachability) = observations.get(value) {
        return ServiceEndpointDiagnostic::new(role, value.into(), location, *reachability, None)
            .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    }
    let Some(address) = bounded_socket_address(host, port) else {
        return ServiceEndpointDiagnostic::new(
            role,
            value.into(),
            location,
            ServiceReachability::NotProbed,
            Some("hostname reachability was not probed without bounded name resolution".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    };
    let reachability = if TcpStream::connect_timeout(&address, timeout).is_ok() {
        ServiceReachability::Reachable
    } else {
        ServiceReachability::Unreachable
    };
    ServiceEndpointDiagnostic::new(role, value.into(), location, reachability, None)
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))
}

fn inspect_unix_endpoint(
    role: ServiceEndpointRole,
    value: &str,
    path: &Path,
) -> Result<ServiceEndpointDiagnostic, MaintenanceServiceAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return ServiceEndpointDiagnostic::new(
            role,
            value.into(),
            ServiceLocation::Unknown,
            ServiceReachability::NotProbed,
            Some("UNIX endpoint path is not a safe absolute path".into()),
        )
        .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()));
    }
    #[cfg(unix)]
    let reachability = if std::os::unix::net::UnixStream::connect(path).is_ok() {
        ServiceReachability::Reachable
    } else {
        ServiceReachability::Unreachable
    };
    #[cfg(not(unix))]
    let reachability = ServiceReachability::NotProbed;
    ServiceEndpointDiagnostic::new(
        role,
        value.into(),
        ServiceLocation::Local,
        reachability,
        None,
    )
    .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))
}

fn split_host_port(authority: &str, default_port: Option<u16>) -> Option<(&str, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        return Some((host, port.parse().ok()?));
    }
    if let Some((host, port)) = authority.rsplit_once(':')
        && !host.is_empty()
        && !port.is_empty()
    {
        return Some((host, port.parse().ok()?));
    }
    default_port.map(|port| (authority, port))
}

fn endpoint_location(host: &str) -> ServiceLocation {
    if host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        ServiceLocation::Local
    } else {
        ServiceLocation::Remote
    }
}

fn bounded_socket_address(host: &str, port: u16) -> Option<SocketAddr> {
    if host.eq_ignore_ascii_case("localhost") {
        return Some(SocketAddr::from(([127, 0, 0, 1], port)));
    }
    host.parse::<IpAddr>()
        .ok()
        .map(|address| SocketAddr::new(address, port))
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitation.is_empty()
        || limitation.len() > MAX_MAINTENANCE_TEXT_BYTES
        || limitations.len() >= MAX_MAINTENANCE_LIMITATIONS
        || limitations.contains(&limitation)
    {
        return;
    }
    limitations.push(limitation);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maintenance_sstate::{
        MaintenanceSstateCommandKind, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-maintenance-service-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn join(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn process(process_root: &Path, pid: u32, name: &str) {
        let directory = process_root.join(pid.to_string());
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("comm"), format!("{name}\n")).unwrap();
        fs::write(directory.join("cmdline"), format!("{name}\0")).unwrap();
    }

    fn fixture_input(
        fixture: &TestDirectory,
        prserv_host: Option<String>,
        hashserve: Option<String>,
        hashserve_upstream: Option<String>,
    ) -> MaintenanceServiceCapabilityInput {
        MaintenanceServiceCapabilityInput {
            build_dir: fixture.join("build"),
            prserv_host,
            hashserve,
            hashserve_upstream,
            signature_handler: Some("OEEquivHash".into()),
            executable_search_path: vec![fixture.join("tools")],
            process_root: fixture.join("proc"),
            endpoint_probe_timeout: Duration::from_millis(20),
            endpoint_observations: Vec::new(),
        }
    }

    fn prepare_fixture(fixture: &TestDirectory, tool_body: Option<&str>) {
        fs::create_dir_all(fixture.join("build")).unwrap();
        fs::create_dir_all(fixture.join("tools")).unwrap();
        fs::create_dir_all(fixture.join("proc")).unwrap();
        if let Some(body) = tool_body {
            executable(&fixture.join("tools/bitbake-prserv-tool"), body);
        }
    }

    fn service(inspection: &MaintenanceServiceInspection, kind: ServiceKind) -> &ServiceDiagnostic {
        inspection
            .services
            .iter()
            .find(|service| service.kind == kind)
            .unwrap()
    }

    fn export_request(
        inspection: &MaintenanceServiceInspection,
        fixture: &TestDirectory,
    ) -> PrServiceRequest {
        PrServiceRequest::new(
            yoctui_model::PrServiceOperation::Export,
            fixture.join("export.conf"),
            fixture.join("build"),
            inspection.capability.metadata.prserv_host.clone().unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn maintenance_service_diagnostics_keep_location_reachability_and_process_evidence_typed() {
        let fixture = TestDirectory::new("diagnostics");
        prepare_fixture(&fixture, Some("#!/bin/sh\nexit 0\n"));
        process(&fixture.join("proc"), 11, "bitbake-prserv");
        process(&fixture.join("proc"), 12, "bitbake-hashserv");
        process(&fixture.join("proc"), 13, "bitbake-worker");
        let mut input = fixture_input(
            &fixture,
            Some("localhost:8585".into()),
            Some("auto".into()),
            Some("hash.example.invalid:8686".into()),
        );
        input.endpoint_observations.push(
            MaintenanceEndpointObservation::new(
                "localhost:8585".into(),
                ServiceReachability::Reachable,
            )
            .unwrap(),
        );
        let inspection = MaintenanceServiceCapabilityInspector::inspect(input).unwrap();

        let pr = service(&inspection, ServiceKind::Pr);
        assert_eq!(pr.state, ServiceState::Reachable);
        assert_eq!(pr.endpoints[0].location, ServiceLocation::Local);
        assert_eq!(pr.endpoints[0].reachability, ServiceReachability::Reachable);
        assert_eq!(pr.process_evidence[0].pid, 11);
        assert!(
            pr.limitations
                .iter()
                .any(|line| line.contains("does not prove"))
        );

        let hash = service(&inspection, ServiceKind::Hash);
        assert_eq!(hash.state, ServiceState::Partial);
        assert_eq!(hash.endpoints.len(), 2);
        assert_eq!(hash.endpoints[0].location, ServiceLocation::Local);
        assert_eq!(hash.endpoints[1].location, ServiceLocation::Remote);
        assert_eq!(
            hash.endpoints[1].reachability,
            ServiceReachability::NotProbed
        );
        assert_eq!(
            service(&inspection, ServiceKind::Worker).process_evidence[0].pid,
            13
        );
    }

    #[test]
    fn maintenance_service_distinguishes_disabled_unreachable_missing_and_unsafe_capability() {
        let fixture = TestDirectory::new("states");
        prepare_fixture(&fixture, None);
        let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            None,
            Some("192.0.2.1:9".into()),
            None,
        ))
        .unwrap();
        assert_eq!(
            service(&inspection, ServiceKind::Pr).state,
            ServiceState::Disabled
        );
        assert_eq!(
            service(&inspection, ServiceKind::Hash).endpoints[0].location,
            ServiceLocation::Remote
        );
        assert_eq!(
            service(&inspection, ServiceKind::Hash).state,
            ServiceState::Unreachable
        );
        assert!(matches!(
            inspection
                .capability
                .capability(MaintenanceTool::PrServiceTool),
            Some(MaintenanceToolCapability::Unavailable { .. })
        ));

        let real = fixture.join("real-tool");
        executable(&real, "#!/bin/sh\nexit 0\n");
        #[cfg(unix)]
        symlink(&real, fixture.join("tools/bitbake-prserv-tool")).unwrap();
        let unsafe_tool = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        assert!(
            unsafe_tool
                .limitations
                .iter()
                .any(|line| line.contains("unsafe executable"))
        );

        let mut missing_process = fixture_input(&fixture, Some("localhost:0".into()), None, None);
        missing_process.process_root = fixture.join("missing-proc");
        let missing_process =
            MaintenanceServiceCapabilityInspector::inspect(missing_process).unwrap();
        assert_eq!(
            service(&missing_process, ServiceKind::Worker).state,
            ServiceState::Unavailable
        );
        assert_eq!(
            service(&missing_process, ServiceKind::Pr).state,
            ServiceState::Partial
        );
    }

    #[test]
    fn maintenance_service_constructs_only_exact_export_and_import_vectors_with_side_effects() {
        let fixture = TestDirectory::new("vectors");
        prepare_fixture(
            &fixture,
            Some("#!/bin/sh\nprintf '%s:%s\\n' \"$1\" \"$2\"\n"),
        );
        let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        let request = export_request(&inspection, &fixture);
        let (preview, command) = pr_service_command(
            MaintenanceSessionId(1),
            7,
            &inspection.capability,
            9,
            request.clone(),
        )
        .unwrap();
        assert_eq!(
            command.kind(),
            MaintenanceSstateCommandKind::PrServiceExport
        );
        assert_eq!(
            command.arguments(),
            [
                std::ffi::OsString::from("export"),
                request.file.as_os_str().to_owned(),
            ]
        );
        assert_eq!(preview.arguments[1], "1: export");
        assert!(
            preview
                .limitations
                .iter()
                .any(|line| line.contains("memory-resident"))
        );
        assert!(
            preview
                .limitations
                .iter()
                .any(|line| line.contains("configured PR endpoint"))
        );

        let import_file = fixture.join("locked.inc");
        fs::write(&import_file, "PRAUTO$example = 1\n").unwrap();
        let import = PrServiceRequest::new(
            yoctui_model::PrServiceOperation::Import,
            import_file.clone(),
            fixture.join("build"),
            "localhost:0".into(),
        )
        .unwrap();
        let (preview, command) = pr_service_command(
            MaintenanceSessionId(2),
            7,
            &inspection.capability,
            10,
            import,
        )
        .unwrap();
        assert_eq!(
            command.kind(),
            MaintenanceSstateCommandKind::PrServiceImport
        );
        assert_eq!(command.arguments()[0], "import");
        assert_eq!(command.arguments()[1], import_file.as_os_str());
        assert!(
            preview
                .limitations
                .iter()
                .any(|line| line.contains("changes PR service data"))
        );
    }

    #[tokio::test]
    async fn maintenance_service_revalidates_tool_file_parent_and_preview_before_spawn() {
        let fixture = TestDirectory::new("revalidate");
        prepare_fixture(&fixture, Some("#!/bin/sh\nexit 0\n"));
        let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        let import_file = fixture.join("locked.conf");
        fs::write(&import_file, "one\n").unwrap();
        let import = PrServiceRequest::new(
            yoctui_model::PrServiceOperation::Import,
            import_file.clone(),
            fixture.join("build"),
            "localhost:0".into(),
        )
        .unwrap();
        let (_, command) = pr_service_command(
            MaintenanceSessionId(1),
            1,
            &inspection.capability,
            1,
            import,
        )
        .unwrap();
        fs::write(&import_file, "changed identity\n").unwrap();
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(path)) if path == import_file
        ));

        let export = export_request(&inspection, &fixture);
        let (_, command) = pr_service_command(
            MaintenanceSessionId(2),
            1,
            &inspection.capability,
            2,
            export,
        )
        .unwrap();
        executable(
            &fixture.join("tools/bitbake-prserv-tool"),
            "#!/bin/sh\necho tampered\nexit 0\n",
        );
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(_))
        ));
    }

    async fn terminal_event(
        runner: &mut MaintenanceSstateJobRunner,
    ) -> MaintenanceSstateRunnerEvent {
        loop {
            let event = runner.next_event().await.unwrap();
            if matches!(
                event,
                MaintenanceSstateRunnerEvent::Completed { .. }
                    | MaintenanceSstateRunnerEvent::Failed { .. }
                    | MaintenanceSstateRunnerEvent::Cancelled { .. }
                    | MaintenanceSstateRunnerEvent::TimedOut { .. }
                    | MaintenanceSstateRunnerEvent::Lost { .. }
            ) {
                return event;
            }
        }
    }

    #[tokio::test]
    async fn maintenance_service_shared_runner_streams_success_and_nonzero_without_a_shell() {
        for (name, body, success) in [
            (
                "success",
                "#!/bin/sh\nprintf 'service:%s\\n' \"$1\"\nprintf 'warning\\n' >&2\nexit 0\n",
                true,
            ),
            ("nonzero", "#!/bin/sh\necho failed >&2\nexit 7\n", false),
        ] {
            let fixture = TestDirectory::new(name);
            prepare_fixture(&fixture, Some(body));
            let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
                &fixture,
                Some("localhost:0".into()),
                None,
                None,
            ))
            .unwrap();
            let (_, command) = pr_service_command(
                MaintenanceSessionId(1),
                1,
                &inspection.capability,
                1,
                export_request(&inspection, &fixture),
            )
            .unwrap();
            let mut runner = MaintenanceSstateJobRunner::new();
            runner.start(command).await.unwrap();
            assert!(matches!(
                runner.next_event().await.unwrap(),
                MaintenanceSstateRunnerEvent::Started { .. }
            ));
            let terminal = terminal_event(&mut runner).await;
            assert_eq!(
                matches!(terminal, MaintenanceSstateRunnerEvent::Completed { .. }),
                success
            );
            if !success {
                assert!(matches!(
                    terminal,
                    MaintenanceSstateRunnerEvent::Failed {
                        exit_code: Some(7),
                        ..
                    }
                ));
            }
        }
    }

    #[tokio::test]
    async fn maintenance_service_shared_runner_preserves_timeout_cancel_rejection_and_loss() {
        let forced_fixture = TestDirectory::new("runner-forced-timeout");
        prepare_fixture(
            &forced_fixture,
            Some("#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n"),
        );
        let forced_inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &forced_fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        let mut timed = MaintenanceSstateJobRunner::new()
            .with_operation_timeout(Duration::from_millis(200))
            .with_cancellation_timeout(Duration::from_millis(10));
        timed
            .start(
                pr_service_command(
                    MaintenanceSessionId(4),
                    1,
                    &forced_inspection.capability,
                    1,
                    export_request(&forced_inspection, &forced_fixture),
                )
                .unwrap()
                .1,
            )
            .await
            .unwrap();
        assert!(matches!(
            terminal_event(&mut timed).await,
            MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
        ));

        let fixture = TestDirectory::new("runner-terminal");
        prepare_fixture(
            &fixture,
            Some("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n"),
        );
        let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        let command = || {
            pr_service_command(
                MaintenanceSessionId(4),
                1,
                &inspection.capability,
                1,
                export_request(&inspection, &fixture),
            )
            .unwrap()
            .1
        };

        let mut cancelled =
            MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(100));
        cancelled.start(command()).await.unwrap();
        cancelled.next_event().await.unwrap();
        assert!(cancelled.cancel(MaintenanceSessionId(4)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { .. }
        ));
        assert!(!cancelled.cancel(MaintenanceSessionId(4)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRejected { .. }
        ));

        let mut lost = MaintenanceSstateJobRunner::new();
        lost.start(command()).await.unwrap();
        lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Lost { .. }
        ));
    }
}

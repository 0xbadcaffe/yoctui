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

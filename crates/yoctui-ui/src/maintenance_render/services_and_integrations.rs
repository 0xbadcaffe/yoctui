pub(crate) fn maintenance_service_records(
    request: u64,
    services: &[yoctui_model::ServiceDiagnostic],
    limitations: &[String],
) -> String {
    let details = services
        .iter()
        .map(|service| {
            let endpoints = service
                .endpoints
                .iter()
                .map(|endpoint| {
                    format!(
                        "  {:?} {} [{:?}, {:?}]{}",
                        endpoint.role,
                        endpoint.value,
                        endpoint.location,
                        endpoint.reachability,
                        endpoint
                            .limitation
                            .as_deref()
                            .map_or_else(String::new, |value| format!(" — {value}")),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let processes = service
                .process_evidence
                .iter()
                .map(|process| {
                    format!(
                        "  PID {} {} (observational)",
                        process.pid, process.executable
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{:?}: {:?}\nEndpoints:\n{}\nProcesses:\n{}\nLimitations:\n- {}",
                service.kind,
                service.state,
                if endpoints.is_empty() {
                    "  none"
                } else {
                    &endpoints
                },
                if processes.is_empty() {
                    "  none"
                } else {
                    &processes
                },
                if service.limitations.is_empty() {
                    "none".into()
                } else {
                    service.limitations.join("\n- ")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Service diagnostics request {request}\n{details}\nInspection limitations:\n- {}",
        if limitations.is_empty() {
            "none".into()
        } else {
            limitations.join("\n- ")
        }
    )
}

pub(crate) fn maintenance_integrations_text(state: &MaintenanceIntegrationDiagnostics) -> String {
    match state {
        MaintenanceIntegrationDiagnostics::NotInspected => {
            "Integration details: not inspected".into()
        }
        MaintenanceIntegrationDiagnostics::Loading(request) => {
            format!("Integration details: loading request {request}")
        }
        MaintenanceIntegrationDiagnostics::Failed { request, message } => {
            format!("Integration request {request} failed: {message}")
        }
        MaintenanceIntegrationDiagnostics::Available { request, snapshot }
        | MaintenanceIntegrationDiagnostics::Partial {
            request, snapshot, ..
        } => {
            let limitations = snapshot
                .limitations
                .iter()
                .chain(snapshot.pull_request.limitations.iter())
                .chain(snapshot.error_report.limitations.iter())
                .chain(snapshot.repo_manifest.limitations.iter())
                .chain(snapshot.toaster.limitations.iter())
                .cloned()
                .collect::<Vec<_>>();
            format!(
                "Integration request {request}\nPull request: {:?}\n  create: {}\n  send: {}\n  worktree: {}\n  HEAD: {}\nError report: {:?}\n  helper: {}\n  candidate: {}\nRepo manifest: {:?}\n  repo: {}\n  workspace: {}\n  manifest: {}\nToaster: {:?}\n  executable: {}\n  configurations: {}\n  observed processes: {}\n  process evidence is observational only",
                snapshot.pull_request.state,
                optional_file_path(snapshot.pull_request.create_helper.as_ref()),
                optional_file_path(snapshot.pull_request.send_helper.as_ref()),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.root.path.display().to_string()
                ),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.head.path.display().to_string()
                ),
                snapshot.error_report.state,
                optional_file_path(snapshot.error_report.helper.as_ref()),
                optional_file_path(snapshot.error_report.candidate_report.as_ref()),
                snapshot.repo_manifest.state,
                optional_file_path(snapshot.repo_manifest.repo_executable.as_ref()),
                snapshot.repo_manifest.workspace.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.path.display().to_string()
                ),
                optional_file_path(snapshot.repo_manifest.manifest.as_ref()),
                snapshot.toaster.state,
                optional_file_path(snapshot.toaster.executable.as_ref()),
                if snapshot.toaster.configurations.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .configurations
                        .iter()
                        .map(|value| value.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                if snapshot.toaster.observed_processes.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .observed_processes
                        .iter()
                        .map(|value| format!("{}:{}", value.pid, value.executable))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            ) + &format!(
                "\nLimitations:\n- {}",
                if limitations.is_empty() {
                    "none".into()
                } else {
                    limitations.join("\n- ")
                }
            )
        }
    }
}

pub(crate) fn optional_file_path(
    identity: Option<&yoctui_model::MaintenanceFileIdentity>,
) -> String {
    identity.map_or_else(
        || "unavailable".into(),
        |value| value.path.display().to_string(),
    )
}

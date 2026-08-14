use std::collections::HashMap;
use tokio::sync::mpsc;
use yoctui_bitbake::{
    MaintenanceReleaseCapabilityInput, MaintenanceReleaseCapabilityInspector,
    MaintenanceServiceCapabilityInput, MaintenanceServiceCapabilityInspector,
    MaintenanceSstateCapabilityInput, MaintenanceSstateCapabilityInspector,
    MaintenanceSstateCommandSpec, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
};
use yoctui_model::{
    MaintenanceOutputStream, MaintenanceSessionId, SstateReadinessMode, SstateReadinessRequest,
};
use yoctui_protocol::daemon::{DaemonMaintenanceSnapshot, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonMaintenanceEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: MaintenanceOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Cancelled {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        session_id: u64,
        message: String,
    },
}

pub struct DaemonMaintenanceSupervisor {
    next_job_id: u64,
    active: HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonMaintenanceEvent>,
    rx: mpsc::UnboundedReceiver<DaemonMaintenanceEvent>,
}

impl Default for DaemonMaintenanceSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: HashMap::new(),
            tx,
            rx,
        }
    }
}

impl DaemonMaintenanceSupervisor {
    #[allow(clippy::too_many_arguments)]
    pub fn start_readiness(
        &mut self,
        session_id: u64,
        capability_request: u64,
        operation_id: u64,
        build_directory: String,
        sstate_directory: Option<String>,
        tmp_directory: Option<String>,
        stamps_directories: Vec<String>,
        executable_search_path: Vec<String>,
        targets: Vec<String>,
        mode: String,
        output: Option<String>,
        log: Option<String>,
        timeout_seconds: u64,
    ) -> Result<JobId, String> {
        if session_id == 0 || self.active.contains_key(&session_id) {
            return Err("maintenance session is already active or invalid".into());
        }
        let input = MaintenanceSstateCapabilityInput {
            build_dir: build_directory.clone().into(),
            sstate_dir: sstate_directory.map(Into::into),
            tmp_dir: tmp_directory.map(Into::into),
            stamps_dirs: stamps_directories.into_iter().map(Into::into).collect(),
            executable_search_path: executable_search_path.into_iter().map(Into::into).collect(),
        };
        let snapshot = MaintenanceSstateCapabilityInspector::inspect(input)
            .map_err(|error| error.to_string())?;
        let mode = match mode.as_str() {
            "isolated_tmpdir" => SstateReadinessMode::IsolatedTmpdir,
            "same_tmpdir" => SstateReadinessMode::SameTmpdir,
            _ => return Err("invalid sstate readiness mode".into()),
        };
        let request = SstateReadinessRequest::new(
            targets,
            mode,
            output.map(Into::into),
            log.map(Into::into),
            timeout_seconds,
        )
        .map_err(str::to_owned)?;
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(session_id),
            capability_request,
            &snapshot,
            operation_id,
            request,
        )
        .map_err(|error| error.to_string())?;
        self.start_command(session_id, command)
    }

    pub fn start_external(
        &mut self,
        session_id: u64,
        executable: String,
        expected_name: String,
        arguments: Vec<String>,
        current_directory: String,
    ) -> Result<JobId, String> {
        if session_id == 0 || self.active.contains_key(&session_id) {
            return Err("maintenance session is already active or invalid".into());
        }
        let command = MaintenanceSstateCommandSpec::external_from_paths(
            MaintenanceSessionId(session_id),
            executable.into(),
            expected_name,
            arguments,
            current_directory.into(),
        )
        .map_err(|error| error.to_string())?;
        self.start_command(session_id, command)
    }

    fn start_command(
        &mut self,
        session_id: u64,
        command: MaintenanceSstateCommandSpec,
    ) -> Result<JobId, String> {
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = MaintenanceSstateJobRunner::new();
            if let Err(error) = runner.start(command).await {
                let _ = tx.send(DaemonMaintenanceEvent::Lost {
                    job_id,
                    session_id,
                    message: error.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {
                    cancel = cancel_rx.recv() => { if cancel.is_some() { let _ = runner.cancel(MaintenanceSessionId(session_id)).await; } }
                    event = runner.next_event() => { let mapped = match event { Ok(MaintenanceSstateRunnerEvent::Started { .. }) => DaemonMaintenanceEvent::Started { job_id, session_id }, Ok(MaintenanceSstateRunnerEvent::Output { stream, line, truncated, .. }) => DaemonMaintenanceEvent::Output { job_id, session_id, stream, line, truncated }, Ok(MaintenanceSstateRunnerEvent::Completed { exit_code, .. }) => DaemonMaintenanceEvent::Completed { job_id, session_id, exit_code }, Ok(MaintenanceSstateRunnerEvent::Failed { exit_code, .. }) | Ok(MaintenanceSstateRunnerEvent::TimedOut { exit_code, .. }) => DaemonMaintenanceEvent::Failed { job_id, session_id, exit_code }, Ok(MaintenanceSstateRunnerEvent::Cancelled { exit_code, .. }) => DaemonMaintenanceEvent::Cancelled { job_id, session_id, exit_code }, Ok(MaintenanceSstateRunnerEvent::CancellationRequested { .. }) => continue, Ok(MaintenanceSstateRunnerEvent::CancellationRejected { message, .. }) | Ok(MaintenanceSstateRunnerEvent::Lost { message, .. }) => DaemonMaintenanceEvent::Lost { job_id, session_id, message }, Err(error) => DaemonMaintenanceEvent::Lost { job_id, session_id, message: error.to_string() } }; let terminal = matches!(mapped, DaemonMaintenanceEvent::Completed { .. } | DaemonMaintenanceEvent::Failed { .. } | DaemonMaintenanceEvent::Cancelled { .. } | DaemonMaintenanceEvent::Lost { .. }); if tx.send(mapped).is_err() || terminal { break; } }
                }
            }
        });
        Ok(job_id)
    }
    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown maintenance session {session_id}"))?
            .send(())
            .map_err(|_| "maintenance session is no longer active".into())
    }
    pub fn try_event(&mut self) -> Option<DaemonMaintenanceEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonMaintenanceEvent::Completed { .. }
                | DaemonMaintenanceEvent::Failed { .. }
                | DaemonMaintenanceEvent::Cancelled { .. }
                | DaemonMaintenanceEvent::Lost { .. }
        ) {
            let id = match &event {
                DaemonMaintenanceEvent::Completed { session_id, .. }
                | DaemonMaintenanceEvent::Failed { session_id, .. }
                | DaemonMaintenanceEvent::Cancelled { session_id, .. }
                | DaemonMaintenanceEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(event)
    }
}

pub fn inspect(
    request: u64,
    build_directory: String,
    sstate_directory: Option<String>,
    tmp_directory: Option<String>,
    stamps_directories: Vec<String>,
    executable_search_path: Vec<String>,
) -> Result<DaemonMaintenanceSnapshot, String> {
    if request == 0 {
        return Err("maintenance capability request is invalid".into());
    }
    let input = MaintenanceSstateCapabilityInput {
        build_dir: build_directory.into(),
        sstate_dir: sstate_directory.map(Into::into),
        tmp_dir: tmp_directory.map(Into::into),
        stamps_dirs: stamps_directories.into_iter().map(Into::into).collect(),
        executable_search_path: executable_search_path.into_iter().map(Into::into).collect(),
    };
    let snapshot = MaintenanceSstateCapabilityInspector::inspect(input.clone())
        .map_err(|error| error.to_string())?;
    let release =
        MaintenanceReleaseCapabilityInspector::inspect(MaintenanceReleaseCapabilityInput {
            build_dir: input.build_dir,
            buildhistory_dir: None,
            native_lsb: None,
            executable_search_path: input.executable_search_path,
        })
        .map_err(|error| error.to_string())?;
    let mut tools = snapshot
        .tools
        .iter()
        .map(|tool| format!("{:?}", tool.tool()))
        .collect::<Vec<_>>();
    tools.extend(
        release
            .tools
            .iter()
            .map(|tool| format!("{:?}", tool.tool())),
    );
    let mut limitations = snapshot.limitations;
    limitations.extend(release.limitations);
    Ok(DaemonMaintenanceSnapshot {
        request,
        tools,
        limitations,
    })
}

pub fn inspect_services(
    request: u64,
    build_directory: String,
    prserv_host: Option<String>,
    hashserve: Option<String>,
    hashserve_upstream: Option<String>,
    signature_handler: Option<String>,
    executable_search_path: Vec<String>,
    process_root: String,
) -> Result<DaemonMaintenanceSnapshot, String> {
    if request == 0 {
        return Err("maintenance service request is invalid".into());
    }
    let inspection =
        MaintenanceServiceCapabilityInspector::inspect(MaintenanceServiceCapabilityInput {
            build_dir: build_directory.into(),
            prserv_host,
            hashserve,
            hashserve_upstream,
            signature_handler,
            executable_search_path: executable_search_path.into_iter().map(Into::into).collect(),
            process_root: process_root.into(),
            endpoint_probe_timeout: std::time::Duration::from_secs(1),
            endpoint_observations: Vec::new(),
        })
        .map_err(|error| error.to_string())?;
    Ok(DaemonMaintenanceSnapshot {
        request,
        tools: inspection
            .capability
            .tools
            .iter()
            .map(|tool| format!("{:?}", tool.tool()))
            .collect(),
        limitations: inspection.limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_runtime_maintenance_rejects_invalid_request() {
        assert!(inspect(0, "/build".into(), None, None, Vec::new(), Vec::new()).is_err());
    }

    #[test]
    fn client_runtime_maintenance_sstate_rejects_invalid_session() {
        assert!(
            DaemonMaintenanceSupervisor::default()
                .start_readiness(
                    0,
                    1,
                    1,
                    "/build".into(),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    vec!["core-image-minimal".into()],
                    "isolated_tmpdir".into(),
                    None,
                    None,
                    1,
                )
                .is_err()
        );
    }

    #[test]
    fn client_runtime_maintenance_service_rejects_invalid_request() {
        assert!(
            inspect_services(
                0,
                "/build".into(),
                None,
                None,
                None,
                None,
                Vec::new(),
                "/proc".into()
            )
            .is_err()
        );
    }

    #[test]
    fn client_runtime_maintenance_release_rejects_invalid_session() {
        assert!(DaemonMaintenanceSupervisor::default().start_external(0, "/missing/tool".into(), "tool".into(), vec!["--help".into()], "/build".into()).is_err());
    }
}

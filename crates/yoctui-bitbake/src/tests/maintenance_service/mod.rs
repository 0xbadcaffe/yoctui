use super::*;
use crate::maintenance_sstate::{
    MaintenanceSstateCommandKind, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

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
    crate::test_support::write_executable(path, body);
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

mod maintenance_service_diagnostics_keep_location_reachability_and_process_evidence_typed;

mod maintenance_service_distinguishes_disabled_unreachable_missing_and_unsafe_capability;

mod maintenance_service_constructs_only_exact_export_and_import_vectors_with_side_effects;

mod maintenance_service_revalidates_tool_file_parent_and_preview_before_spawn;

async fn terminal_event(runner: &mut MaintenanceSstateJobRunner) -> MaintenanceSstateRunnerEvent {
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

mod maintenance_service_shared_runner_streams_success_and_nonzero_without_a_shell;

mod maintenance_service_shared_runner_preserves_timeout_cancel_rejection_and_loss;

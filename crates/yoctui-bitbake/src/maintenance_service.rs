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

include!("maintenance_service/capability_inspection.rs");
include!("maintenance_service/command_and_process_scan.rs");
include!("maintenance_service/endpoint_inspection.rs");

#[cfg(test)]
mod tests {
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

//! Shared fixtures and regression modules.

use super::*;
use proptest::prelude::*;

pub(crate) fn log(message: &str) -> LogEntry {
    LogEntry {
        id: 0,
        severity: Severity::Info,
        message: message.into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
        build: None,
        protected: false,
        diagnostic: None,
    }
}
pub(crate) fn tagged_log(recipe: &str, task: &str, severity: Severity, message: &str) -> LogEntry {
    LogEntry {
        id: 0,
        severity,
        message: message.into(),
        recipe: Some(recipe.into()),
        task: Some(task.into()),
        path: None,
        timestamp: SystemTime::now(),
        build: None,
        protected: false,
        diagnostic: None,
    }
}
pub(crate) fn background_job_spec(id: u64, cancellation_supported: bool) -> BackgroundJobSpec {
    BackgroundJobSpec {
        id: BackgroundJobId(id),
        kind: BackgroundJobKind::Build,
        title: format!("Build job {id}"),
        context: BackgroundJobContext {
            workspace: Some(Screen::Tasks),
            target: Some("core-image-minimal".into()),
            ..BackgroundJobContext::default()
        },
        cancellation_supported,
        queued_at: SystemTime::UNIX_EPOCH,
    }
}
pub(crate) fn run_background_job(app: &mut App, id: u64) {
    let id = BackgroundJobId(id);
    let _ = update(
        app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        },
    );
    let _ = update(app, Action::RunBackgroundJob { id });
}
pub(crate) fn signature_record(
    recipe: &str,
    task: &str,
    hash: &str,
    path: &str,
) -> SignatureRecord {
    SignatureRecord {
        identity: SignatureIdentity {
            target: SignatureTarget {
                recipe: recipe.into(),
                task: task.into(),
            },
            hash: Some(hash.into()),
            path: Some(PathBuf::from(path)),
        },
        base_hash: Some(format!("base-{hash}")),
        task_hash: Some(format!("task-{hash}")),
        variables: Vec::new(),
        dependencies: Vec::new(),
    }
}
proptest! {
    #[test]
    fn retention_never_exceeds_count_or_bytes(messages in proptest::collection::vec(".{0,64}", 0..80), max_entries in 1usize..20, max_bytes in 1usize..256) {
        let mut logs = LogState::new(max_entries, max_bytes);
        for message in messages { logs.insert(log(&message)); }
        prop_assert!(logs.entries.len() <= max_entries);
        prop_assert!(logs.retained_bytes <= max_bytes || logs.entries.is_empty());
        prop_assert_eq!(logs.retained_bytes, logs.entries.iter().map(|entry| entry.message.len()).sum::<usize>());
    }
}

pub(crate) fn package_summary(name: &str, recipe: &str) -> PackageSummary {
    PackageSummary {
        identity: PackageIdentity::new(name),
        recipe: PackageField::Available(recipe.into()),
        provider: PackageField::Available(format!("/layers/meta/recipes/{recipe}.bb").into()),
        version: PackageField::Available("1.0".into()),
        installed_size_bytes: PackageField::Unavailable,
        license: PackageField::Unavailable,
        image_membership: PackageField::Available(Vec::new()),
    }
}

pub(crate) fn qemu_model_artifact() -> ImageArtifact {
    ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
        },
        kind: ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    }
}

pub(crate) fn qemu_model_app() -> App {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    let artifact = qemu_model_artifact();
    let request = ImageArtifactRequest {
        generation: 1,
        machine: artifact.identity.machine.clone(),
    };
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request,
        inventory: ImageArtifactInventory {
            machine: artifact.identity.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact.clone()],
        },
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![artifact.identity],
    };
    app.workspace.build_dir = Some("/build".into());
    app
}

pub(crate) fn wic_model_capability() -> WicCapability {
    WicCapability::Available {
        executable: "/opt/poky/scripts/wic".into(),
        kickstarts: vec![WicKickstart {
            identity: WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some("/layers/meta/wic/directdisk.wks".into()),
            },
            source: "part / --source rootfs --fstype=ext4 --size=64".into(),
            partitions: vec![WicPartitionSummary {
                mount_point: Some("/".into()),
                filesystem: Some("ext4".into()),
                source_plugin: Some("rootfs".into()),
                size_mib: Some(64),
                alignment_kib: None,
            }],
            limitations: Vec::new(),
        }],
        image_targets: vec!["core-image-minimal".into()],
    }
}

pub(crate) fn wic_model_device(path: &str, major_minor: &str) -> WicDevice {
    WicDevice {
        identity: WicDeviceIdentity {
            path: path.into(),
            major_minor: major_minor.into(),
            size_bytes: 2048,
            model: Some("test".into()),
            serial: Some(format!("serial-{major_minor}")),
            transport: Some("usb".into()),
        },
        removable: true,
        writable: true,
        read_only: false,
        descendant_mounts: Vec::new(),
        unavailable_reason: None,
    }
}

pub(crate) fn sdk_workflow_app() -> App {
    let mut app = App::new(20, 20_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.workspace
        .variables
        .insert("SDK_DEPLOY".into(), "/build/deploy/sdk".into());
    app.build.target = Some("core-image-minimal".into());
    app.sdk_tool_capability = SdkToolCapability::Available {
        publish: Some("/opt/poky/oe-publish-sdk".into()),
        find_sysroot: Some("/opt/poky/oe-find-native-sysroot".into()),
        run_native: Some("/opt/poky/oe-run-native".into()),
    };
    app
}

pub(crate) fn test_workflow_app() -> App {
    let mut app = App::new(20, 20_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.build.target = Some("core-image-minimal".into());
    app.test_capability = TestCapability {
        oe_selftest: TestExecutableCapability::Available("/workspace/oe-selftest".into()),
        bitbake_selftest: TestExecutableCapability::Available("/workspace/bitbake-selftest".into()),
        ptest: PtestCapability::Configured,
    };
    app
}

pub(crate) fn test_results_record(
    name: &str,
    fingerprint: &str,
    outcomes: &[(&str, TestCaseOutcome)],
) -> TestResultRecord {
    let cases = outcomes
        .iter()
        .map(|(case_name, outcome)| {
            TestCaseRecord::new(
                TestCaseIdentity::new("suite".into(), (*case_name).into()).unwrap(),
                *outcome,
                Some(Duration::from_millis(10)),
                Vec::new(),
                Some(format!("/build/logs/{name}-{case_name}.log").into()),
            )
            .unwrap()
            .0
        })
        .collect();
    let suite = TestSuiteRecord::new("suite".into(), None, Vec::new(), cases)
        .unwrap()
        .0;
    TestResultRecord::new(
        TestResultIdentity::new(
            format!("/build/results/{name}/testresults.json").into(),
            2_048,
            SystemTime::UNIX_EPOCH,
            fingerprint.into(),
        )
        .unwrap(),
        Some(TestFamily::TestImage),
        Some("qemux86-64".into()),
        Some("core-image-minimal".into()),
        Some("rev-1".into()),
        Some(Duration::from_secs(1)),
        Vec::new(),
        vec![suite],
        Some(TestSessionId(1)),
        Vec::new(),
    )
    .0
}

pub(crate) fn load_test_results(
    app: &mut App,
    records: Vec<TestResultRecord>,
    limitations: Vec<String>,
) -> TestResultImportRequest {
    let Some(Effect::ImportTestResults(request)) =
        begin_test_result_import(app, vec!["/build/results".into()])
    else {
        panic!("import effect");
    };
    let _ = update(
        app,
        Action::TestResultsLoaded {
            request: request.clone(),
            records,
            limitations,
        },
    );
    request
}

pub(crate) fn ux_terminal_fixture(
    lifecycle: ClientDaemonLifecycle,
    writer: Option<[u8; 16]>,
) -> App {
    let mut app = App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.terminal.client_id = Some([7; 16]);
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 41,
        name: "devshell:busybox".into(),
        lifecycle,
        viewers: 2,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 41,
        kind: ClientDaemonPtyKind::Devshell,
        cwd: "/work/build".into(),
        columns: 100,
        rows: 28,
        writer,
        writer_epoch: 9,
        exit_code: None,
        restartable: true,
    });
    app.daemon.pty_screens.push(ClientDaemonPtyScreen {
        session_id: 41,
        columns: 100,
        rows_count: 28,
        cursor_column: 0,
        cursor_row: 1,
        cursor_hidden: false,
        scrollback_offset: 0,
        rows: vec!["$ bitbake busybox -c devshell".into(), "ready".into()],
        cells: Vec::new(),
        scrollback_lines: 120,
        dropped_line_feeds_lower_bound: 17,
    });
    app
}
mod coexistence_diagnostic_distinguishes_nominal_busy_and_unknown;
mod completed_builds_are_retained_in_session_history;
mod config_compare_explains_scope_loading_and_missing_detail;
mod dependency_graph_normalization_reports_hard_bounds;
mod devwork_editor_detects_languages_and_supports_search_undo_and_redo;
mod image_artifact_model_correlates_states_search_and_stable_selection;
mod live_tasks_reducer_keeps_honest_counts_filters_and_bounded_selection;
mod navigator_workbench_order_keeps_build_and_validation_groups_contiguous;
mod qemu_workspace_availability_reasons_are_stable_and_cancellation_is_modal;
mod snapshot_timing_observed_reducer_does_not_fall_back_to_local_clock;
mod terminal_navigation_palette_and_writer_lease_are_typed;
mod test_workflow_model_attaches_managed_builds_and_rejects_stale_results;

use super::background_jobs::MAX_BACKGROUND_JOB_OUTPUT_ENTRIES;
use super::editor_types::source_structural_validation;
use super::session_updates::WIC_BACKGROUND_JOB_NAMESPACE;

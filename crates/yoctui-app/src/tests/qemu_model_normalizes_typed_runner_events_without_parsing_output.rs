//! Regression tests grouped around qemu_model_normalizes_typed_runner_events_without_parsing_output.
use super::*;

#[test]
fn qemu_model_normalizes_typed_runner_events_without_parsing_output() {
    let id = QemuSessionId(7);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        qemu_actions_for_runner_event(id, QemuRunnerEvent::Starting, timestamp),
        vec![Action::QemuSessionStarting {
            id,
            started_at: timestamp
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Output {
                stream: QemuRunnerOutputStream::Stderr,
                line: "verbatim runner output".into(),
                truncated: true,
            },
            timestamp,
        ),
        vec![Action::AppendQemuSessionOutput {
            id,
            stream: QemuOutputStream::Stderr,
            line: "verbatim runner output".into(),
            truncated: true,
            timestamp,
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Failed {
                message: "spawn failed".into(),
                exit_code: Some(127),
            },
            timestamp,
        ),
        vec![Action::FailQemuSession {
            id,
            message: "spawn failed".into(),
            exit_code: Some(127),
            finished_at: timestamp,
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::CancellationRejected {
                message: "not running".into(),
            },
            timestamp,
        ),
        vec![Action::RejectQemuSessionCancellation {
            id,
            message: "not running".into(),
        }]
    );
}
#[test]
fn qemu_adapter_normalizes_forced_cancellation_and_loss() {
    let id = QemuSessionId(11);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            timestamp,
        ),
        vec![
            Action::AppendQemuSessionOutput {
                id,
                stream: QemuOutputStream::Stderr,
                line: "runqemu cancellation required forced termination".into(),
                truncated: false,
                timestamp,
            },
            Action::CancelQemuSession {
                id,
                exit_code: Some(137),
                finished_at: timestamp,
            }
        ]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Lost {
                message: "event channel lost".into(),
            },
            timestamp,
        ),
        vec![Action::LoseQemuSession {
            id,
            message: "event channel lost".into(),
            finished_at: timestamp,
        }]
    );
}
#[test]
fn qemu_workspace_maps_launch_edit_preview_and_cancellation_keys() {
    assert_eq!(
        images_workspace_action(false, Input::Char('Q')),
        Some(Action::BeginSelectedQemuLaunch)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('x')),
        Some(Action::BeginActiveImageRuntimeCancellation)
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Down),
        Some(Action::SelectQemuLaunchField { delta: 1 })
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Left),
        Some(Action::CycleQemuLaunchChoice { backwards: true })
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Enter),
        Some(Action::ActivateQemuLaunchField)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Char('/')),
        Some(Action::AppendQemuLaunchField('/'))
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Backspace),
        Some(Action::BackspaceQemuLaunchField)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Enter),
        Some(Action::FinishQemuLaunchFieldEdit)
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewQemuLaunch)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Esc),
        Some(Action::CancelQemuLaunch)
    );
    assert_eq!(
        qemu_launch_confirmation_action(Input::Enter),
        Some(Action::ConfirmQemuLaunchInTerminal)
    );
    assert_eq!(
        qemu_launch_confirmation_action(Input::Esc),
        Some(Action::CancelQemuLaunchPreview)
    );
    assert_eq!(
        qemu_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmQemuSessionCancellation)
    );
    assert_eq!(
        qemu_cancellation_confirmation_action(Input::Esc),
        Some(Action::CancelQemuSessionCancellation)
    );
}
#[test]
fn wic_device_write_and_creation_map_distinct_modal_and_artifact_keys() {
    assert_eq!(
        images_workspace_action(false, Input::Char('W')),
        Some(Action::BeginSelectedWicCreate)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('w')),
        Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic
        ))
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Down),
        Some(Action::SelectWicCreateField { delta: 1 })
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Left),
        Some(Action::CycleWicCreateChoice { backwards: true })
    );
    assert_eq!(
        wic_create_dialog_action(true, Input::Char('/')),
        Some(Action::AppendWicCreateField('/'))
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewWicCreate)
    );
    assert_eq!(
        wic_create_confirmation_action(Input::Enter),
        Some(Action::ConfirmWicCreate)
    );
    assert_eq!(
        wic_cancellation_confirmation_action(WicSessionId(3), false, Input::Enter),
        Some(Action::ConfirmWicSessionCancellation {
            id: WicSessionId(3),
            acknowledge_incomplete_device: false,
        })
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('D')),
        Some(Action::BeginSelectedWicDeviceWrite)
    );
    assert_eq!(
        wic_device_picker_action(Input::Down),
        Some(Action::SelectWicDevice { delta: 1 })
    );
    assert_eq!(
        wic_device_picker_action(Input::Enter),
        Some(Action::ConfirmWicDeviceSelection)
    );
    assert_eq!(
        wic_write_phrase_action(Input::Char('W')),
        Some(Action::AppendWicWritePhrase('W'))
    );
    assert_eq!(
        wic_write_phrase_action(Input::Enter),
        Some(Action::PreviewWicDeviceWrite)
    );
    assert_eq!(
        wic_write_confirmation_action(Input::Enter),
        Some(Action::ConfirmWicDeviceWrite)
    );
    assert_eq!(
        wic_cancellation_confirmation_action(WicSessionId(4), true, Input::Enter),
        Some(Action::ConfirmWicSessionCancellation {
            id: WicSessionId(4),
            acknowledge_incomplete_device: true,
        })
    );
}
#[test]
fn wic_model_normalizes_typed_session_events_without_parsing_output() {
    let id = WicSessionId(7);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        wic_actions_for_session_event(id, WicSessionEvent::Starting, timestamp),
        vec![Action::WicSessionStarting {
            id,
            started_at: timestamp
        }]
    );
    assert_eq!(
        wic_actions_for_session_event(
            id,
            WicSessionEvent::Output {
                stream: WicOutputStream::Stderr,
                line: "raw adapter text".into(),
                truncated: true,
            },
            timestamp,
        ),
        vec![Action::AppendWicSessionOutput {
            id,
            stream: WicOutputStream::Stderr,
            line: "raw adapter text".into(),
            truncated: true,
            timestamp,
        }]
    );
    assert_eq!(
        wic_actions_for_session_event(
            id,
            WicSessionEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            timestamp,
        ),
        vec![
            Action::AppendWicSessionOutput {
                id,
                stream: WicOutputStream::Stderr,
                line: "Wic cancellation required forced termination".into(),
                truncated: false,
                timestamp,
            },
            Action::CancelWicSession {
                id,
                exit_code: Some(137),
                finished_at: timestamp,
            }
        ]
    );
}
#[test]
fn wic_adapter_capability_crosses_app_boundary_without_parsing() {
    let capability = WicCapability::MissingKickstarts {
        executable: "/usr/bin/wic".into(),
    };
    assert_eq!(
        wic_capability_action(capability.clone()),
        Action::WicCapabilityLoaded(capability)
    );
}
#[test]
fn wic_adapter_runner_normalizes_typed_terminal_events() {
    let id = WicSessionId(8);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        wic_actions_for_runner_event(
            id,
            WicRunnerEvent::Failed {
                message: "failed".into(),
                exit_code: Some(9),
            },
            timestamp,
        ),
        vec![Action::FailWicSession {
            id,
            message: "failed".into(),
            exit_code: Some(9),
            finished_at: timestamp,
        }]
    );
}
#[test]
fn wic_device_write_adapter_boundary_preserves_inventory_and_loss() {
    let request = yoctui_model::WicDeviceInventoryRequest {
        generation: 3,
        image: yoctui_model::WicOutputIdentity {
            path: "/build/output/image.wic".into(),
            size_bytes: 4_096,
            modified_unix_seconds: 7,
        },
    };
    let device = yoctui_model::WicDevice {
        identity: yoctui_model::WicDeviceIdentity {
            path: "/dev/sdz".into(),
            major_minor: "8:240".into(),
            size_bytes: 8_192,
            model: Some("fixture".into()),
            serial: Some("serial".into()),
            transport: Some("usb".into()),
        },
        removable: true,
        writable: true,
        read_only: false,
        descendant_mounts: Vec::new(),
        unavailable_reason: None,
    };
    assert_eq!(
        wic_device_inventory_action(WicDeviceInventoryResponse {
            request: request.clone(),
            devices: vec![device.clone()],
            limitations: vec!["excluded /dev/sda".into()],
        }),
        Action::WicDeviceInventoryLoaded {
            request,
            devices: vec![device],
            limitations: vec!["excluded /dev/sda".into()],
        }
    );
    let id = WicSessionId(11);
    assert_eq!(
        wic_actions_for_runner_event(
            id,
            WicRunnerEvent::Lost {
                message: "write output channel lost".into(),
            },
            SystemTime::UNIX_EPOCH,
        ),
        vec![Action::LoseWicSession {
            id,
            message: "write output channel lost".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        }]
    );
}
#[test]
fn maps_recipe_and_task_filter_controls() {
    assert_eq!(
        key_action(Input::Char('R')),
        Some(Action::CycleLogRecipeFilter)
    );
    assert_eq!(
        key_action(Input::Char('T')),
        Some(Action::CycleLogTaskFilter)
    );
}
#[test]
fn maps_log_match_navigation_controls() {
    assert_eq!(key_action(Input::Char('n')), Some(Action::NextLogMatch));
    assert_eq!(key_action(Input::Char('N')), Some(Action::PreviousLogMatch));
}

#[test]
fn sdk_workflow_maps_workspace_and_modal_keys_without_leakage() {
    assert_eq!(
        sdk_workspace_action(false, Input::Char('s')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Standard
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('E')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Extensible
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Standard
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('T')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Extensible
        )))
    );
    assert_eq!(
        sdk_workspace_action(true, Input::Char('x')),
        Some(Action::AppendSdkArtifactQuery('x'))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedSdkArtifact)
    );
    assert_eq!(
        sdk_build_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkBuild)
    );
    assert_eq!(
        sdk_publish_dialog_action(Input::Char('P')),
        Some(Action::AppendSdkPublishDestination('P')),
        "publication modal input must not leak to the SDK workspace"
    );
    assert_eq!(
        sdk_publish_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkPublish)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Esc),
        Some(Action::CancelSdkNative)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewSdkNative)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Down),
        Some(Action::SelectSdkNativeField { delta: 1 })
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Enter),
        Some(Action::ActivateSdkNativeField)
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Char('x')),
        Some(Action::AppendSdkNativeField('x'))
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Enter),
        Some(Action::FinishSdkNativeFieldEdit)
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Esc),
        Some(Action::CancelSdkNative),
        "Esc closes the dialog even while a field is being edited"
    );
    assert_eq!(
        sdk_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkSessionCancellation)
    );
    let id = SdkSessionId(7);
    assert_eq!(
        sdk_actions_for_runner_event(id, SdkToolRunnerEvent::Started, SystemTime::UNIX_EPOCH),
        vec![
            Action::SdkSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH
            },
            Action::SdkSessionRunning { id }
        ]
    );
    assert!(matches!(
        sdk_actions_for_runner_event(
            id,
            SdkToolRunnerEvent::TimedOut {
                forced: true,
                exit_code: None
            },
            SystemTime::UNIX_EPOCH
        )
        .as_slice(),
        [Action::FailSdkSession {
            id: SdkSessionId(7),
            message,
            exit_code: None,
            ..
        }] if message.contains("timed out") && message.contains("forced")
    ));
}

#[test]
fn test_workflow_test_runner_maps_keys_and_classifies_managed_bitbake_tests() {
    assert_eq!(
        testing_workspace_action(Input::Down),
        Some(Action::SelectTestFamily { delta: 1 })
    );
    assert_eq!(
        testing_workspace_action(Input::Char('r')),
        Some(Action::BeginSelectedTestLaunch)
    );
    assert_eq!(
        testing_workspace_action(Input::Char('x')),
        Some(Action::BeginActiveTestSessionCancellation)
    );
    assert_eq!(
        test_launch_dialog_action(false, Input::Down),
        Some(Action::SelectTestLaunchField { delta: 1 })
    );
    assert_eq!(
        test_launch_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewTestLaunch)
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Char('x')),
        Some(Action::AppendTestLaunchField('x')),
        "editable launch fields must trap printable input"
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Enter),
        Some(Action::FinishTestLaunchFieldEdit)
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Esc),
        Some(Action::CancelTestLaunch),
        "Esc closes the launch dialog while a field is edited"
    );
    assert_eq!(
        test_launch_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestLaunch)
    );
    assert_eq!(
        test_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestSessionCancellation)
    );

    let mut coordinator = BuildJobCoordinator::default();
    let actions = coordinator
        .queue_build(
            &BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: Some("testimage".into()),
                force: false,
            },
            SystemTime::UNIX_EPOCH,
        )
        .expect("valid testimage request");
    assert!(matches!(
        actions.first(),
        Some(Action::QueueBackgroundJob(BackgroundJobSpec {
            kind: BackgroundJobKind::Test,
            context: BackgroundJobContext {
                workspace: Some(Screen::Testing),
                task: Some(task),
                ..
            },
            ..
        })) if task == "testimage"
    ));
}

#[test]
fn test_results_map_workspace_search_and_modal_keys_without_leakage() {
    assert_eq!(
        test_results_workspace_action(false, false, Input::Tab),
        Some(Action::CycleTestView)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Enter),
        Some(Action::DrillIntoSelectedTestResult)
    );
    assert_eq!(
        test_results_workspace_action(false, true, Input::Down),
        Some(Action::SelectTestCase { delta: 1 })
    );
    assert_eq!(
        test_results_workspace_action(false, true, Input::Esc),
        Some(Action::LeaveTestResultCases)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('I')),
        Some(Action::BeginTestResultImport)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('c')),
        Some(Action::BeginTestComparison)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('J')),
        Some(Action::BeginTestJunitExport)
    );
    assert_eq!(
        test_results_workspace_action(true, false, Input::Char('x')),
        Some(Action::AppendTestResultQuery('x')),
        "search input must not leak to the Testing workspace"
    );
    assert_eq!(
        test_result_import_dialog_action(Input::Char('/')),
        Some(Action::AppendTestResultImport('/'))
    );
    assert_eq!(
        test_result_import_dialog_action(Input::Enter),
        Some(Action::ConfirmTestResultImport)
    );
    assert_eq!(
        test_comparison_dialog_action(Input::Right),
        Some(Action::CycleTestComparisonField)
    );
    assert_eq!(
        test_comparison_dialog_action(Input::Char('p')),
        Some(Action::PreviewTestComparison)
    );
    assert_eq!(
        test_comparison_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestComparison)
    );
    assert_eq!(
        test_junit_dialog_action(Input::Char('x')),
        Some(Action::AppendTestJunitDestination('x'))
    );
    assert_eq!(
        test_junit_dialog_action(Input::Enter),
        Some(Action::PreviewTestJunitExport)
    );
    assert_eq!(
        test_junit_confirmation_action(Input::Esc),
        Some(Action::CancelTestJunitExportPreview)
    );
    assert_eq!(
        test_comparison_workspace_action(Input::Char('l')),
        Some(Action::OpenSelectedTestTransitionLog)
    );
}

#[test]
fn test_runner_events_map_once_with_stream_and_terminal_meaning() {
    let id = yoctui_model::TestSessionId(9);
    assert_eq!(
        test_actions_for_runner_event(id, TestRunnerEvent::Started, SystemTime::UNIX_EPOCH),
        [
            Action::TestSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
            Action::TestSessionRunning { id },
        ]
    );
    assert_eq!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Output {
                stream: yoctui_model::TestOutputStream::Stderr,
                line: "warning".into(),
                truncated: true,
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::AppendTestSessionOutput {
            id,
            stream: yoctui_model::TestOutputStream::Stderr,
            line: "warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        }]
    );
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Completed {
                exit_code: Some(0),
                result_paths: vec!["/build/testresults.json".into()],
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::CompleteTestSession {
            id: yoctui_model::TestSessionId(9),
            exit_code: 0,
            result_paths,
            ..
        }] if result_paths == &[PathBuf::from("/build/testresults.json")]
    ));
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::TimedOut {
                forced: true,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::TimeoutTestSession {
            id: yoctui_model::TestSessionId(9),
            forced: true,
            exit_code: None,
            ..
        }]
    ));
    assert_eq!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            SystemTime::UNIX_EPOCH,
        )
        .len(),
        2,
        "forced cancellation retains both its warning and terminal action"
    );
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Lost {
                message: "channel closed".into(),
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::LoseTestSession { message, .. }] if message == "channel closed"
    ));
}

#[test]
fn test_results_adapter_responses_map_to_identity_correlated_actions() {
    let baseline = yoctui_model::TestResultIdentity::new(
        "/results/baseline/testresults.json".into(),
        10,
        SystemTime::UNIX_EPOCH,
        "baseline".into(),
    )
    .unwrap();
    let candidate = yoctui_model::TestResultIdentity::new(
        "/results/candidate/testresults.json".into(),
        11,
        SystemTime::UNIX_EPOCH,
        "candidate".into(),
    )
    .unwrap();
    let import_request =
        yoctui_model::TestResultImportRequest::new(3, vec![baseline.path.clone()]).unwrap();
    assert_eq!(
        test_results_import_action(TestResultImportResponse {
            request: import_request.clone(),
            records: Vec::new(),
            limitations: vec!["empty fixture".into()],
        }),
        Action::TestResultsLoaded {
            request: import_request,
            records: Vec::new(),
            limitations: vec!["empty fixture".into()],
        }
    );

    let request =
        yoctui_model::TestComparisonRequest::new(4, baseline.clone(), candidate.clone()).unwrap();
    let comparison = TestComparison {
        baseline,
        candidate,
        transitions: Vec::new(),
    };
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Completed {
                operation: TestResultOperation::Comparison(request.clone()),
                exit_code: Some(0),
            },
            Some(comparison.clone()),
            vec!["bounded import".into()],
        ),
        [Action::TestComparisonLoaded {
            request: request.clone(),
            comparison,
            limitations: vec!["bounded import".into()],
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Failed {
                operation: TestResultOperation::Comparison(request.clone()),
                exit_code: Some(6),
            },
            None,
            Vec::new(),
        ),
        [Action::TestComparisonFailed {
            request,
            message: "resulttool exited unsuccessfully with exit code 6".into(),
        }]
    );
}

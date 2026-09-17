//! Regression tests grouped around test_results_junit_events_keep_terminal_outcomes_distinct.
use super::*;

#[test]
fn test_results_junit_events_keep_terminal_outcomes_distinct() {
    let result = yoctui_model::TestResultIdentity::new(
        "/results/testresults.json".into(),
        10,
        SystemTime::UNIX_EPOCH,
        "result".into(),
    )
    .unwrap();
    let request = yoctui_model::TestJunitExportRequest {
        generation: 8,
        result,
        destination: "/exports/results.xml".into(),
    };
    let operation = TestResultOperation::Junit(request.clone());
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Completed {
                operation: operation.clone(),
                exit_code: Some(0),
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportSucceeded {
            request: request.clone()
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::TimedOut {
                operation: operation.clone(),
                forced: true,
                exit_code: None,
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportTimedOut {
            request: request.clone()
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Lost {
                operation: Some(operation),
                message: "worker channel closed".into(),
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportLost {
            request,
            message: "worker channel closed".into(),
        }]
    );
    assert!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::CancellationRejected {
                message: "not running".into(),
            },
            None,
            Vec::new(),
        )
        .is_empty()
    );
}

#[test]
fn security_workflow_maps_workspace_search_and_modal_keys_without_leakage() {
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, false, Input::Char('V')),
        Some(Action::Security(SecurityAction::BeginCveCheck))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Sbom, false, false, Input::Down),
        Some(Action::Security(SecurityAction::SelectReport(1)))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Sbom, true, false, Input::Down),
        Some(Action::Security(SecurityAction::SelectComponent(1)))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, true, Input::Char('x')),
        Some(Action::Security(SecurityAction::AppendQuery('x')))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, true, Input::Char('V')),
        Some(Action::Security(SecurityAction::AppendQuery('V'))),
        "search editing consumes workflow shortcuts"
    );

    let preview = yoctui_model::SecurityOperationPreview {
        id: yoctui_model::SecuritySessionId(7),
        scope: yoctui_model::SecurityScope::Image {
            target: "core-image-minimal".into(),
            machine: "qemux86-64".into(),
            distro: "poky".into(),
        },
        operation: yoctui_model::SecurityOperation::SbomBuild(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("create_recipe_sbom".into()),
            force: false,
        }),
        indexed_arguments: vec!["0: bitbake".into()],
        report_roots: vec!["/build/tmp/deploy/spdx".into()],
    };
    assert_eq!(
        security_dialog_action(&SecurityDialog::Operation(preview.clone()), Input::Enter),
        Some(Action::Security(SecurityAction::ConfirmOperation(preview)))
    );
    let mut import_editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
    import_editor.select_toml_value("root").unwrap();
    import_editor.editing = true;
    assert_eq!(
        security_dialog_action(
            &SecurityDialog::Import {
                editor: import_editor,
                validation_error: None,
            },
            Input::Char('V')
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('V'))),
        "modal text editing does not leak CVE launch"
    );
    assert_eq!(
        security_dialog_action(
            &SecurityDialog::Cancellation(yoctui_model::SecuritySessionId(7)),
            Input::Esc
        ),
        Some(Action::Security(SecurityAction::CancelDialog))
    );
}

#[test]
fn qa_workflow_maps_workspace_search_and_drill_keys_without_leakage() {
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('r')),
        Some(Action::Qa(QaAction::BeginSelectedCheck))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Down),
        Some(Action::Qa(QaAction::SelectCheck(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, true, false, Input::Down),
        Some(Action::Qa(QaAction::SelectFinding(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, true, Input::Char('r')),
        Some(Action::Qa(QaAction::AppendQuery('r'))),
        "search editing consumes QA run shortcuts"
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, true, false, Input::Esc),
        Some(Action::Qa(QaAction::LeaveDrill))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('l')),
        Some(Action::Qa(QaAction::OpenSelectedSource))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Tab),
        Some(Action::Qa(QaAction::CycleView))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Down),
        Some(Action::Qa(QaAction::SelectLayer(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('r')),
        Some(Action::Qa(QaAction::BeginSelectedLayerCheck))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('c')),
        Some(Action::Qa(QaAction::BeginLayerCancellation))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('e')),
        Some(Action::Qa(QaAction::OpenSelectedLayerRoot))
    );
}

#[test]
fn qa_workflow_maps_task_capability_response_without_reinterpreting_it() {
    let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    })
    .unwrap();
    let snapshot = yoctui_model::QaCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        scope.clone(),
        vec![scope],
        vec![],
        vec!["one optional report root was unsafe".into()],
    )
    .unwrap();
    assert_eq!(
        qa_task_capability_action(QaTaskCapabilityResponse::Available(snapshot.clone())),
        Action::Qa(QaAction::CapabilityLoaded(snapshot.clone()))
    );
    assert_eq!(
        qa_task_capability_action(QaTaskCapabilityResponse::Partial(snapshot.clone())),
        Action::Qa(QaAction::CapabilityPartial {
            snapshot,
            limitations: vec!["one optional report root was unsafe".into()],
        })
    );
}

#[test]
fn qa_workflow_maps_report_adapter_outcomes_without_parsing_them() {
    let request = yoctui_model::QaReportRequest::new(9, vec!["/build/reports".into()]).unwrap();
    assert_eq!(
        qa_report_response_action(QaReportResponse {
            request: request.clone(),
            outcome: QaReportScanOutcome::Empty,
        }),
        Action::Qa(QaAction::ReportsLoaded {
            request: request.clone(),
            reports: Vec::new(),
            limitations: Vec::new(),
        })
    );
    assert_eq!(
        qa_report_response_action(QaReportResponse {
            request: request.clone(),
            outcome: QaReportScanOutcome::Partial {
                reports: Vec::new(),
                limitations: vec!["one exact report was malformed".into()],
            },
        }),
        Action::Qa(QaAction::ReportsLoaded {
            request: request.clone(),
            reports: Vec::new(),
            limitations: vec!["one exact report was malformed".into()],
        })
    );
    assert_eq!(
        qa_report_error_action(request.clone(), QaReportAdapterError::Cancelled),
        Action::Qa(QaAction::ReportsCancelled(request.clone()))
    );
    assert_eq!(
        qa_report_error_action(request.clone(), QaReportAdapterError::Timeout(30)),
        Action::Qa(QaAction::ReportsTimedOut(request.clone()))
    );
    assert_eq!(
        qa_report_error_action(
            request.clone(),
            QaReportAdapterError::WorkerLost("channel closed".into())
        ),
        Action::Qa(QaAction::ReportsLost {
            request: request.clone(),
            message: "channel closed".into(),
        })
    );
    assert!(matches!(
        qa_report_error_action(
            request,
            QaReportAdapterError::PermissionDenied("/build/reports".into())
        ),
        Action::Qa(QaAction::ReportsFailed {
            kind: QaReportFailureKind::PermissionDenied,
            ..
        })
    ));
}

#[test]
fn qa_workflow_maps_layer_capability_and_runner_events_mechanically() {
    let layer =
        yoctui_model::QaLayerIdentity::new("meta-demo".into(), "/layers/meta-demo".into()).unwrap();
    let configured = yoctui_model::QaConfiguredLayerCapability::new(
        yoctui_model::QaCheckId::new("layer-meta-demo".into()).unwrap(),
        layer.clone(),
        vec!["walnascar".into()],
        yoctui_model::QaLayerRunCapability::Disabled("tool unavailable".into()),
        vec!["tool unavailable".into()],
    )
    .unwrap();
    let snapshot = yoctui_model::QaLayerCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        layer,
        vec![configured],
        vec!["tool unavailable".into()],
    )
    .unwrap();
    assert_eq!(
        qa_layer_capability_action(QaLayerCapabilityResponse::Partial(snapshot.clone())),
        Action::Qa(QaAction::LayerCapabilityPartial {
            snapshot,
            limitations: vec!["tool unavailable".into()],
        })
    );

    let timestamp = SystemTime::UNIX_EPOCH;
    let session = yoctui_model::QaLayerSessionId(7);
    assert_eq!(
        qa_layer_runner_action(QaLayerRunnerEvent::Started { id: session }, timestamp),
        Some(Action::Qa(QaAction::LayerSessionRunning(session)))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::Output {
                id: session,
                stream: yoctui_model::QaOutputStream::Stderr,
                line: "warning".into(),
                truncated: false,
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::LayerSessionOutput {
            session,
            stream: yoctui_model::QaOutputStream::Stderr,
            line: "warning".into(),
            truncated: false,
        }))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::TimedOut {
                id: session,
                forced: true,
                exit_code: None,
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::TimeoutLayerSession {
            session,
            forced: true,
            exit_code: None,
            finished_at: timestamp,
        }))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::Lost {
                id: session,
                message: "channel lost".into(),
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::LoseLayerSession {
            session,
            message: "channel lost".into(),
            finished_at: timestamp,
        }))
    );
}

#[test]
fn qa_workflow_dialogs_map_only_typed_confirmation_and_edit_actions() {
    let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
        name: "linux-yocto".into(),
        file: "/layers/meta/recipes-kernel/linux/linux-yocto.bb".into(),
    })
    .unwrap();
    let preview = yoctui_model::QaOperationPreview {
        id: yoctui_model::QaOperationId(7),
        check: yoctui_model::QaCheckId::new("kernel-config".into()).unwrap(),
        family: yoctui_model::QaCheckFamily::KernelConfiguration,
        scope,
        request: BuildRequest {
            targets: vec!["linux-yocto".into()],
            task: Some("kernel_configcheck".into()),
            force: false,
        },
        indexed_arguments: vec!["0: bitbake".into()],
        report_roots: vec!["/build/tmp/log/qa".into()],
        limitations: vec![],
    };
    assert_eq!(
        qa_dialog_action(&QaDialog::Operation(preview.clone()), Input::Enter),
        Some(Action::Qa(QaAction::ConfirmOperation(preview)))
    );
    assert_eq!(
        {
            let mut editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
            editor.select_toml_value("root").unwrap();
            editor.editing = true;
            qa_dialog_action(
                &QaDialog::Import {
                    editor,
                    validation_error: None,
                },
                Input::Char('r'),
            )
        },
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('r'))),
        "modal text editing does not leak QA run"
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::Cancellation {
                session: yoctui_model::QaSessionId(3),
                background_job: BackgroundJobId(9),
            },
            Input::Enter,
        ),
        Some(Action::Qa(QaAction::ConfirmCancellation(
            yoctui_model::QaSessionId(3)
        )))
    );
    let layer_preview = yoctui_model::QaLayerOperationPreview {
        id: yoctui_model::QaLayerOperationId(4),
        check: yoctui_model::QaCheckId::new("yocto-check-layer".into()).unwrap(),
        layer: yoctui_model::QaLayerIdentity::new("meta".into(), "/layers/meta".into()).unwrap(),
        executable: yoctui_model::QaExecutableIdentity::new(
            "/poky/scripts/yocto-check-layer".into(),
            10,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap(),
        arguments: vec!["--layer".into(), "/layers/meta".into()],
        indexed_arguments: vec!["0: /poky/scripts/yocto-check-layer".into()],
        report_roots: vec![],
        limitations: vec![],
    };
    assert_eq!(
        qa_dialog_action(
            &QaDialog::LayerOperation(layer_preview.clone()),
            Input::Enter
        ),
        Some(Action::Qa(QaAction::ConfirmLayerOperation(layer_preview)))
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::LayerCancellation(yoctui_model::QaLayerSessionId(4)),
            Input::Enter,
        ),
        Some(Action::Qa(QaAction::ConfirmLayerCancellation(
            yoctui_model::QaLayerSessionId(4)
        )))
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::Import {
                editor: yoctui_model::PopupEditor::new("root = \"\"\n".into()),
                validation_error: None,
            },
            Input::Tab
        ),
        None
    );
}

#[test]
fn cache_snapshot_progress_matches_live_batches_after_eviction_and_reset() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
        MAX_DAEMON_BUILD_EVENTS,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut journal =
        DaemonSnapshotJournal::new(initial.clone(), DaemonSnapshotLimits::default()).unwrap();
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, initial);
    for build in [
        DaemonBuildEvent::Reset {
            targets: vec!["image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::SstateSummary {
            summary: yoctui_model::SstateSummary {
                wanted: 10,
                local: 3,
                mirrors: 2,
                missed: 5,
                current: 8,
            },
        },
        DaemonBuildEvent::TaskQueued {
            recipe: "seed".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(yoctui_protocol::TaskStatsData {
                completed: 2_339,
                total: 6_812,
                active: 1,
                failed: 0,
            }),
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(build)).unwrap();
        replica.apply_event_to_app(&mut app, &event).unwrap();
    }
    let count = MAX_DAEMON_BUILD_EVENTS + 8;
    for index in 0..count {
        let event = journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                recipe: format!("recipe-{index}"),
                task: "do_fetch".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: None,
            }))
            .unwrap();
        replica
            .apply_task_events_to_app(&mut app, &[event])
            .unwrap();
    }
    let expected = (2_339 + count, Some(6_812));
    assert_eq!(app.build.cache.fetch_completed, count);
    let cache = app.build.cache;
    assert_eq!((app.build.completed, app.build.total), expected);
    replica.replace_app(&mut app, journal.snapshot().clone());
    assert_eq!(app.build.cache, cache);
    assert_eq!(app.build.cache.summary.unwrap().match_percent(), Some(50));
    assert_eq!((app.build.completed, app.build.total), expected);
    let event = journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
            targets: vec!["next".into()],
        }))
        .unwrap();
    replica.apply_event_to_app(&mut app, &event).unwrap();
    assert_eq!((app.build.completed, app.build.total), (0, None));
    assert!(app.completed_tasks.is_empty());
    assert_eq!(app.build.cache, yoctui_model::BuildCacheState::default());
}

#[test]
fn snapshot_progress_survives_completed_task_compaction() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let snapshot = daemon_protocol_snapshot(&state);
    let mut journal =
        DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default()).unwrap();
    let mut uninterrupted = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut uninterrupted, snapshot);
    for event in [
        DaemonBuildEvent::Reset {
            targets: vec!["obmc-phosphor-image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskQueued {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(yoctui_protocol::TaskStatsData {
                completed: 2_339,
                total: 6_812,
                active: 2,
                failed: 0,
            }),
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskCompleted {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica
            .apply_event_to_app(&mut uninterrupted, &event)
            .unwrap();
    }
    assert_eq!(
        (uninterrupted.build.completed, uninterrupted.build.total),
        (2_340, Some(6_812))
    );
    let mut attached = yoctui_model::App::new(64, 64 * 1024);
    attached.screen = Screen::Recipes;
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    assert_eq!(
        (attached.build.completed, attached.build.total),
        (2_340, Some(6_812))
    );
    assert_eq!(attached.screen, Screen::Recipes);
    assert_eq!(attached.completed_tasks.len(), 1);
    replica.replace_app(&mut uninterrupted, journal.snapshot().clone());
    assert_eq!(
        (uninterrupted.build.completed, uninterrupted.build.total),
        (2_340, Some(6_812))
    );
}

#[test]
fn worker_count_snapshot_live_batch_and_reconnect_share_identity_authority() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent as B, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut journal = DaemonSnapshotJournal::new(
        daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap();
    for event in [
        B::Reset {
            targets: vec!["image".into()],
        },
        B::Started {
            started_unix_ms: None,
        },
    ] {
        journal.publish(DaemonEvent::Build(event)).unwrap();
    }
    let mut live = yoctui_model::App::new(64, 64 * 1024);
    let mut batch = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    let mut batch_replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut live, journal.snapshot().clone());
    batch_replica.replace_app(&mut batch, journal.snapshot().clone());
    let mut events = Vec::new();
    for (recipe, pid) in [("rust-native", 41), ("tar", 42)] {
        let event = journal
            .publish(DaemonEvent::Build(B::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                pid: Some(pid),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: None,
            }))
            .unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
        events.push(event);
    }
    batch_replica
        .apply_task_events_to_app(&mut batch, &events)
        .unwrap();
    let mut attached = yoctui_model::App::new(64, 64 * 1024);
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    for app in [&live, &batch, &attached] {
        assert_eq!(app.active_worker_count(), Some(2));
    }
    let event = journal
        .publish(DaemonEvent::Build(B::TaskCompleted {
            recipe: "tar".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.active_worker_count(), Some(1));
    replica.disconnect_app(&mut live);
    assert_eq!(live.active_worker_count(), None);
    replica.replace_app(&mut live, journal.snapshot().clone());
    assert_eq!(live.active_worker_count(), Some(1));
    let event = journal
        .publish(DaemonEvent::Build(B::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: None,
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.active_worker_count(), Some(0));
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    assert_eq!(attached.active_worker_count(), Some(0));
    for event in [
        B::Reset {
            targets: vec!["next".into()],
        },
        B::Started {
            started_unix_ms: None,
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    assert_eq!(live.active_worker_count(), None);
}

#[test]
fn snapshot_timing_legacy_never_invents_attachment_start() {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "fixture-boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        DaemonBuildEvent::Reset {
            targets: vec!["image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "llvm-native".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
            started_unix_ms: None,
        },
    ];
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    DaemonClientSnapshot::default().replace_app(&mut app, snapshot);
    assert_eq!(
        app.build.started, None,
        "legacy snapshots carry no start authority"
    );
    assert_eq!(
        app.tasks[&TaskId("llvm-native:do_compile".into())].started,
        None
    );
}

#[test]
fn task_identity_unknown_statistics_and_known_lifecycle_never_create_ghost_rows() {
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    let _ = yoctui_model::update(
        &mut app,
        Action::BuildRequested {
            target: Some("image".into()),
        },
    );
    for event in [
        BackendEvent::BuildStarted,
        BackendEvent::TaskStats(yoctui_model::TaskStats {
            completed: 3,
            total: 10,
            active: 1,
            failed: 0,
        }),
    ] {
        let _ = yoctui_model::update(&mut app, model_action_from_backend_event(event).unwrap());
    }
    assert_eq!((app.build.completed, app.build.total), (3, Some(10)));
    assert!(app.tasks.is_empty());
    for recipe in ["llvm-native", "lib32-actual-name", "overridden-name"] {
        for event in [
            BackendEvent::TaskQueued {
                recipe: recipe.into(),
                task: "do_compile".into(),
                worker: None,
                stats: None,
            },
            BackendEvent::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                worker: None,
                stats: None,
                pid: Some(42),
                log_path: None,
            },
            BackendEvent::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
            },
        ] {
            let _ = yoctui_model::update(&mut app, model_action_from_backend_event(event).unwrap());
        }
        assert!(app.tasks.is_empty());
    }
    assert_eq!(app.completed_tasks.len(), 3);
    assert_eq!(app.build.completed, 6);
}

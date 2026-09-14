//! Regression tests grouped around ux_dependency_graph_maps_typed_navigation_filter_topology_and_open_actions.
use super::*;

#[test]
fn ux_dependency_graph_maps_typed_navigation_filter_topology_and_open_actions() {
    assert_eq!(
        dependency_workspace_action(false, Input::Up),
        Some(Action::SelectDependencyGraphNode { delta: -1 })
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('j')),
        Some(Action::SelectDependencyGraphNode { delta: 1 })
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Enter),
        Some(Action::OpenSelectedDependencyRecipe)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedDependencyProvider)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('L')),
        Some(Action::OpenSelectedDependencyTaskLog)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('r')),
        Some(Action::RefreshDependencyGraph)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('v')),
        Some(Action::ToggleDependencyGraphReverse)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Left),
        Some(Action::CollapseSelectedDependencyGraphNode)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Right),
        Some(Action::ExpandSelectedDependencyGraphNode)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('/')),
        Some(Action::BeginDependencyGraphSearch)
    );
    assert_eq!(
        dependency_workspace_action(true, Input::Char('b')),
        Some(Action::AppendDependencyGraphQuery('b'))
    );
    assert_eq!(
        dependency_workspace_action(true, Input::Esc),
        Some(Action::FinishDependencyGraphSearch)
    );
    assert_eq!(dependency_workspace_action(false, Input::Char('x')), None);
}
#[test]
fn ux_dependency_graph_mouse_click_selects_exact_projected_identity() {
    let root = DependencyNodeId::recipe("root");
    let child = DependencyNodeId::recipe("child");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        Vec::new(),
        vec![DependencyEdge {
            from: root,
            to: child.clone(),
            kind: DependencyEdgeKind::Build,
        }],
        10,
        10,
    );
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dependencies;
    app.focus = FocusTarget::Workspace;
    app.dependency_graph = DependencyGraphState::Available(graph);
    let action = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 24,
            row: 5,
        },
        &app,
        160,
        48,
    );
    assert_eq!(
        action,
        Some(Action::SelectDependencyGraphNodeAt { identity: child })
    );
}
#[test]
fn recipe_bitbake_action_maps_standard_and_forced_task_controls() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('f')),
        Some(Action::BeginSelectedRecipeForceTask)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('v')),
        Some(Action::BeginSelectedRecipeDevshell)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('K')),
        Some(Action::BeginSelectedRecipeDiffconfig)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('z')),
        Some(Action::BeginSelectedRecipeDiffsigs)
    );
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("compile".into()),
        force: true,
    };
    let mut coordinator = BuildJobCoordinator::default();
    let actions = coordinator
        .queue_build(&request, SystemTime::UNIX_EPOCH)
        .unwrap();
    assert!(matches!(
        &actions[0],
        Action::QueueBackgroundJob(spec)
            if spec.context.target.as_deref() == Some("busybox")
                && spec.context.task.as_deref() == Some("compile")
    ));
    assert!(
        coordinator
            .queue_build(&request, SystemTime::UNIX_EPOCH)
            .is_none()
    );
}
#[test]
fn recipe_navigation_maps_files_logs_patches_and_devtool_routes() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('e')),
        Some(Action::OpenSelectedRecipeProvider)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('o')),
        Some(Action::BeginSelectedRecipeTaskLog)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('p')),
        Some(Action::BeginSelectedRecipePatchReview)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('d')),
        Some(Action::BeginSelectedRecipeDevtoolModify)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('u')),
        Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('F')),
        Some(Action::BeginSelectedRecipeDevtoolFinish)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('P')),
        Some(Action::BeginSelectedRecipeDevtoolDeploy)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('D')),
        Some(Action::BeginSelectedRecipeDevtoolReset)
    );
}

#[test]
fn devtool_metadata_shortcut_requests_typed_status() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
}

#[test]
fn config_workspace_maps_search_selection_and_lazy_detail() {
    assert_eq!(
        config_workspace_action(false, Input::Down),
        Some(Action::SelectConfigVariable { delta: 1 })
    );
    assert_eq!(
        config_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedConfigDetail)
    );
    assert_eq!(
        config_workspace_action(false, Input::Char('/')),
        Some(Action::BeginMetadataSearch)
    );
    assert_eq!(
        config_workspace_action(true, Input::Char('M')),
        Some(Action::AppendMetadataQuery('M'))
    );
}

#[test]
fn config_copy_shortcuts_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('C')),
        Some(Action::CopySelectedConfigEffective)
    );
    assert_eq!(
        config_workspace_action(false, Input::Char('U')),
        Some(Action::CopySelectedConfigUnexpanded)
    );
}

#[test]
fn config_source_picker_keys_are_modal_and_typed() {
    assert_eq!(
        config_source_picker_action(Input::Down),
        Some(Action::SelectConfigSource { delta: 1 })
    );
    assert_eq!(
        config_source_picker_action(Input::Enter),
        Some(Action::OpenSelectedConfigSourceChoice)
    );
    assert_eq!(
        config_source_picker_action(Input::Esc),
        Some(Action::CancelConfigSourcePicker)
    );
}

#[test]
fn config_scope_shortcut_and_picker_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('s')),
        Some(Action::OpenConfigScopePicker)
    );
    assert_eq!(
        config_scope_picker_action(Input::Down),
        Some(Action::SelectConfigScope { delta: 1 })
    );
    assert_eq!(
        config_scope_picker_action(Input::Enter),
        Some(Action::ConfirmConfigScope)
    );
    assert_eq!(
        config_scope_picker_action(Input::Esc),
        Some(Action::CancelConfigScopePicker)
    );
}

#[test]
fn config_compare_shortcut_and_close_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('c')),
        Some(Action::OpenConfigComparison)
    );
    assert_eq!(
        config_compare_dialog_action(Input::Enter),
        Some(Action::CloseConfigComparison)
    );
    assert_eq!(
        config_compare_dialog_action(Input::Esc),
        Some(Action::CloseConfigComparison)
    );
}

#[test]
fn config_edit_preview_shortcut_and_dialog_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('E')),
        Some(Action::BeginConfigEdit)
    );
    assert_eq!(
        config_edit_dialog_action(Input::Char('x')),
        Some(Action::AppendConfigEdit('x'))
    );
    assert_eq!(
        config_edit_dialog_action(Input::Enter),
        Some(Action::PreviewConfigEdit)
    );
    assert_eq!(
        config_edit_confirmation_action(Input::Enter),
        Some(Action::ConfirmConfigEdit)
    );
}

#[test]
fn config_edit_write_confirmation_is_modal_and_cancellable() {
    assert_eq!(
        config_edit_confirmation_action(Input::Enter),
        Some(Action::ConfirmConfigEdit)
    );
    assert_eq!(
        config_edit_confirmation_action(Input::Esc),
        Some(Action::CancelConfigEditConfirmation)
    );
    assert_eq!(config_edit_confirmation_action(Input::Char('E')), None);
}

#[test]
fn devwork_editor_routes_confirmation_and_workspace_editor_build_keys() {
    let mut editor = yoctui_model::RecipeEditor {
        recipe: "busybox".into(),
        root: "/workspace/busybox".into(),
        files: vec!["main.c".into()],
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main() {}".into()),
        searching: false,
    };
    assert_eq!(
        devtool_modify_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolModify)
    );
    assert_eq!(
        devtool_modify_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolModify)
    );
    assert_eq!(devtool_modify_confirmation_action(Input::Char('b')), None);
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlB),
        Some(Action::BeginRecipeEditorBuild)
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::Enter),
        Some(Action::FocusRecipeEditor(
            yoctui_model::RecipeEditorFocus::Document
        ))
    );
    editor.focus = yoctui_model::RecipeEditorFocus::Document;
    editor.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert_eq!(
        recipe_editor_action(&editor, Input::Enter),
        Some(Action::EditRecipeEditor(PopupEditorCommand::Newline))
    );
}

#[test]
fn devtool_publish_update_routes_only_confirmation_keys() {
    assert_eq!(
        devtool_update_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolUpdateRecipe)
    );
    assert_eq!(
        devtool_update_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolUpdateRecipe)
    );
    assert_eq!(devtool_update_confirmation_action(Input::Char('u')), None);
}

#[test]
fn devtool_publish_finish_routes_picker_and_confirmation_keys() {
    assert_eq!(
        devtool_finish_picker_action(Input::Up),
        Some(Action::SelectDevtoolFinishLayer { delta: -1 })
    );
    assert_eq!(
        devtool_finish_picker_action(Input::Down),
        Some(Action::SelectDevtoolFinishLayer { delta: 1 })
    );
    assert_eq!(
        devtool_finish_picker_action(Input::Enter),
        Some(Action::PreviewDevtoolFinish)
    );
    assert_eq!(
        devtool_finish_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolFinish)
    );
    assert_eq!(
        devtool_finish_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolFinishConfirmation)
    );
}

#[test]
fn devtool_target_deploy_routes_entry_and_confirmation_keys() {
    assert_eq!(
        devtool_deploy_dialog_action(Input::Char('q')),
        Some(Action::AppendDevtoolDeployTarget('q'))
    );
    assert_eq!(
        devtool_deploy_dialog_action(Input::Backspace),
        Some(Action::BackspaceDevtoolDeployTarget)
    );
    assert_eq!(
        devtool_deploy_dialog_action(Input::Enter),
        Some(Action::PreviewDevtoolDeploy)
    );
    assert_eq!(
        devtool_deploy_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolDeploy)
    );
    assert_eq!(
        devtool_deploy_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolDeployConfirmation)
    );
}

#[test]
fn devtool_target_reset_routes_only_destructive_confirmation_keys() {
    assert_eq!(
        devtool_reset_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolReset)
    );
    assert_eq!(
        devtool_reset_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolReset)
    );
    assert_eq!(devtool_reset_confirmation_action(Input::Char('D')), None);
}

#[test]
fn devtool_job_lifecycle_maps_runner_events_and_stays_independent_from_bitbake() {
    let now = SystemTime::UNIX_EPOCH;
    let mut devtool = DevtoolJobCoordinator::default();
    let operation = DevtoolOperation::Reset {
        recipe: "busybox".into(),
    };
    let actions = devtool.queue(operation.clone(), now).unwrap();
    let id = devtool.active_job_id().unwrap();
    assert_eq!(id, BackgroundJobId(1_u64 << 63));
    assert_eq!(devtool.active_operation(), Some(&operation));
    assert!(devtool.queue(operation, now).is_none());

    let mut build = BuildJobCoordinator::default();
    let build_actions = build
        .queue_build(
            &BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            },
            now,
        )
        .unwrap();
    assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
    assert_ne!(build.active_job_id(), devtool.active_job_id());

    let mut app = yoctui_model::App::new(10, 1_000);
    for action in actions.into_iter().chain(build_actions) {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in devtool.actions_for_event(DevtoolRunnerEvent::Started, now) {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in devtool.actions_for_event(
        DevtoolRunnerEvent::Output {
            stream: DevtoolOutputStream::Stderr,
            line: "progress".into(),
            truncated: true,
        },
        now,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    app.screen = Screen::Dashboard;
    for action in
        devtool.actions_for_event(DevtoolRunnerEvent::Completed { exit_code: Some(0) }, now)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
    assert!(job.output[0].truncated);
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
}

#[test]
fn devtool_job_lifecycle_maps_start_failure_cancel_failure_cancel_and_loss() {
    let now = SystemTime::UNIX_EPOCH;
    let operation = DevtoolOperation::Reset {
        recipe: "busybox".into(),
    };

    let mut coordinator = DevtoolJobCoordinator::default();
    let id = {
        let _ = coordinator.queue(operation.clone(), now);
        coordinator.active_job_id().unwrap()
    };
    assert!(matches!(
        coordinator.start_failed("missing".into(), now).as_slice(),
        [Action::FailBackgroundJob { id: failed, .. }] if *failed == id
    ));

    let mut coordinator = DevtoolJobCoordinator::default();
    let _ = coordinator.queue(operation.clone(), now);
    assert!(matches!(
        coordinator.request_cancellation(),
        Some(Action::RequestBackgroundJobCancellation { .. })
    ));
    assert!(coordinator.request_cancellation().is_none());
    let rejected = coordinator.cancellation_failed("signal".into(), now);
    assert!(matches!(
        rejected.last(),
        Some(Action::RejectBackgroundJobCancellation { .. })
    ));
    assert!(matches!(
        coordinator
            .actions_for_event(
                DevtoolRunnerEvent::Cancelled {
                    forced: true,
                    exit_code: None,
                },
                now,
            )
            .last(),
        Some(Action::CancelBackgroundJob { .. })
    ));

    let mut coordinator = DevtoolJobCoordinator::default();
    let _ = coordinator.queue(operation, now);
    assert!(matches!(
        coordinator
            .actions_for_event(
                DevtoolRunnerEvent::Lost {
                    message: "channel".into(),
                },
                now,
            )
            .as_slice(),
        [Action::LoseBackgroundJob { .. }]
    ));
}

#[test]
fn recipe_qa_action_maps_capabilities_and_persists_terminal_job_outcomes() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('V')),
        Some(Action::BeginSelectedRecipeCveCheck)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('X')),
        Some(Action::BeginSelectedRecipeSpdx)
    );

    let cve = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("cve_check".into()),
        force: false,
    };
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(20, 4_000);
    let queued = coordinator
        .queue_build(&cve, SystemTime::UNIX_EPOCH)
        .unwrap();
    assert!(matches!(
        &queued[0],
        Action::QueueBackgroundJob(spec)
            if spec.kind == BackgroundJobKind::CveCheck
                && spec.context.workspace == Some(Screen::Recipes)
                && spec.context.recipe.as_deref() == Some("busybox")
                && spec.context.task.as_deref() == Some("cve_check")
    ));
    for action in queued {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in
        coordinator.actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
        SystemTime::UNIX_EPOCH,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    let cve_job = app.background_jobs.jobs.back().unwrap();
    assert_eq!(cve_job.status, BackgroundJobStatus::Succeeded);
    assert!(
        cve_job
            .result
            .as_ref()
            .unwrap()
            .summary
            .contains("no result path")
    );
    assert!(cve_job.result.as_ref().unwrap().artifacts.is_empty());

    let spdx = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("create_spdx".into()),
        force: false,
    };
    for action in coordinator
        .queue_build(&spdx, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert!(
        coordinator
            .queue_build(&spdx, SystemTime::UNIX_EPOCH)
            .is_none()
    );
    let cancellation = coordinator.request_cancellation().unwrap();
    let _ = yoctui_model::update(&mut app, cancellation);
    for action in coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(130),
        },
        SystemTime::UNIX_EPOCH,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert_eq!(
        app.background_jobs.jobs.back().unwrap().status,
        BackgroundJobStatus::Cancelled
    );

    for action in coordinator
        .queue_build(&cve, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in
        coordinator.actions_for_backend_event(BackendEvent::Disconnected, SystemTime::UNIX_EPOCH)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert_eq!(
        app.background_jobs.jobs.back().unwrap().status,
        BackgroundJobStatus::Lost
    );
}

#[test]
fn signature_workspace_maps_recipe_entry_picker_and_workspace_keys() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('Z')),
        Some(Action::BeginSelectedRecipeSignatures)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('z')),
        Some(Action::BeginSelectedRecipeDiffsigs)
    );
    assert_eq!(
        signature_task_picker_action(Input::Down),
        Some(Action::SelectSignatureTask { delta: 1 })
    );
    assert_eq!(
        signature_task_picker_action(Input::Enter),
        Some(Action::ConfirmSignatureTask)
    );
    assert_eq!(
        signature_task_picker_action(Input::Esc),
        Some(Action::CancelSignatureTaskPicker)
    );
    assert_eq!(
        signature_workspace_action(Input::Up),
        Some(Action::SelectSignatureRecord { delta: -1 })
    );
    assert_eq!(
        signature_workspace_action(Input::Char('1')),
        Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left
        ))
    );
    assert_eq!(
        signature_workspace_action(Input::Char('2')),
        Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right
        ))
    );
    assert_eq!(
        signature_workspace_action(Input::Char('c')),
        Some(Action::BeginSignatureComparison)
    );
    assert_eq!(
        signature_workspace_action(Input::Char('r')),
        Some(Action::RefreshSignatureDump)
    );
    assert_eq!(
        signature_workspace_action(Input::Char('e')),
        Some(Action::OpenSignatureProvider)
    );
    assert_eq!(
        signature_workspace_action(Input::Esc),
        Some(Action::LeaveSignatureWorkspace)
    );
    assert_eq!(signature_workspace_action(Input::Char('x')), None);
}
#[test]
fn images_workspace_image_action_maps_search_refresh_build_cancel_and_open_actions() {
    assert_eq!(
        images_workspace_action(false, Input::Up),
        Some(Action::SelectImageArtifact { delta: -1 })
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('R')),
        Some(Action::RefreshImageArtifactInventory)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('b')),
        Some(Action::BeginSelectedImageArtifactBuild)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('T')),
        Some(Action::BeginSelectedImageConsole)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('c')),
        Some(Action::CancelImageArtifactOperation)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedImageArtifact)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('m')),
        Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest
        ))
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('/')),
        Some(Action::BeginImageArtifactSearch)
    );
    assert_eq!(
        images_workspace_action(true, Input::Char('w')),
        Some(Action::AppendImageArtifactQuery('w'))
    );
    assert_eq!(
        images_workspace_action(true, Input::Esc),
        Some(Action::FinishImageArtifactSearch)
    );
}

#[test]
fn image_console_input_keeps_text_typing_separate_from_choice_navigation() {
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/image.wic".into(),
    };
    let mut dialog =
        yoctui_model::ImageConsoleDialog::new(yoctui_model::ImageConsoleDraft::for_artifact(
            identity,
            yoctui_model::ImageArtifactKind::Wic,
        ));
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Right),
        Some(Action::CycleImageConsoleChoice { backwards: false })
    );
    dialog.draft.mode = yoctui_model::ImageConsoleMode::Ssh;
    dialog.selected_field = yoctui_model::ImageConsoleField::Host;
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Char('j')),
        Some(Action::AppendImageConsoleField('j'))
    );
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Enter),
        Some(Action::ConfirmImageConsole)
    );
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Esc),
        Some(Action::CancelImageConsole)
    );
}

#[test]
fn ux_rootfs_images_tabs_keyboard_and_scroll_map_to_typed_drilldown_actions() {
    use yoctui_model::ImagesView;

    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Enter),
        Some(Action::BeginSelectedRootfsComposition)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Tab),
        Some(Action::ShiftImagesView { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsPackages, Input::Left),
        Some(Action::SelectRootfsGroup { delta: -1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsPackages, Input::PageDown),
        Some(Action::SelectRootfsPackage { delta: 10 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Down),
        Some(Action::SelectRootfsEntry { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Right),
        Some(Action::BrowseRootfsFilesystem)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::SystemdServices, Input::Char('e')),
        Some(Action::EditSelectedRootfsSystemFile)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::SystemDbus, Input::Down),
        Some(Action::SelectRootfsDbusService { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::BackTab),
        Some(Action::ShiftImagesView { delta: -1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('3')),
        Some(Action::ShiftImagesView { delta: 2 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Char('1')),
        Some(Action::ShiftImagesView { delta: -2 })
    );
    assert_eq!(
        images_workspace_action_for_view(true, ImagesView::Artifacts, Input::Char('2')),
        Some(Action::AppendImageArtifactQuery('2'))
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('5')),
        Some(Action::ShiftImagesView { delta: 4 })
    );

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Images;
    app.images_view = ImagesView::RootfsFilesystem;
    assert_eq!(
        workspace_collection_action(&app, Input::PageUp),
        Some(Action::SelectRootfsEntry { delta: -10 })
    );
}

#[test]
fn udev_keys_select_sixth_tab_and_scroll_without_spawning() {
    use yoctui_model::ImagesView;
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('6')),
        Some(Action::ShiftImagesView { delta: 5 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::End),
        Some(Action::SelectRootfsUdevRule { delta: isize::MAX })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::PageDown),
        Some(Action::SelectRootfsUdevRule { delta: 10 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::Char(']')),
        Some(Action::ScrollRootfsUdevPreview { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::Char('e')),
        None
    );
}

//! Regression tests grouped around live_tasks_reducer_keeps_honest_counts_filters_and_bounded_selection.
use super::*;

#[test]
fn live_tasks_reducer_keeps_honest_counts_filters_and_bounded_selection() {
    let mut app = App::new(20, 2_000);
    let first = TaskId("busybox:do_compile".into());
    let second = TaskId("openssl:do_install".into());
    let mut busybox = TaskInfo::active(first.clone(), "busybox".into(), "do_compile".into());
    busybox.worker = Some("worker-1".into());
    busybox.stats = Some(TaskStats {
        completed: 1,
        total: 5,
        active: 1,
        failed: 0,
    });
    let _ = update(&mut app, Action::TaskStarted(busybox));
    let mut openssl = TaskInfo::active(second.clone(), "openssl".into(), "do_install".into());
    openssl.worker = Some("worker-2".into());
    let _ = update(&mut app, Action::TaskStarted(openssl));
    assert_eq!(app.build.completed, 1);
    assert_eq!(app.build.total, Some(5));
    assert_eq!(app.waiting_task_count(), 2);
    assert!(matches!(
        app.visible_task_rows().last(),
        Some(TaskRow::WaitingSummary(2))
    ));

    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: second,
            success: false,
        },
    );
    let _ = update(&mut app, Action::CycleTaskStateFilter);
    assert_eq!(app.task_filters.state, TaskStateFilter::Active);
    assert_eq!(app.visible_task_rows().len(), 1);
    for _ in 0..3 {
        let _ = update(&mut app, Action::CycleTaskStateFilter);
    }
    assert_eq!(app.task_filters.state, TaskStateFilter::Failed);
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task)] if task.recipe == "openssl" && task.state == TaskState::Failed
    ));

    app.task_progress_scroll = 99;
    let _ = update(&mut app, Action::CycleTaskDurationFilter);
    assert_eq!(app.task_progress_scroll, 0);
    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: first,
            success: true,
        },
    );
    assert!(app.task_progress_scroll <= app.visible_task_rows().len().saturating_sub(1));
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(app.build.completed, 5);
    assert_eq!(app.waiting_task_count(), 0);
}

#[test]
fn task_state_distinguishes_identified_queue_entries_from_aggregate_waiting_work() {
    let mut app = App::new(20, 2_000);
    app.build.status = BuildStatus::Running;
    app.build.total = Some(4);
    let id = TaskId("busybox:do_compile".into());
    let mut queued = TaskInfo::active(id.clone(), "busybox".into(), "do_compile".into());
    queued.pid = Some(4242);

    let _ = update(&mut app, Action::TaskQueued(queued));
    let queued = app.tasks.get(&id).expect("queued task retained");
    assert_eq!(queued.state, TaskState::Queued);
    assert_eq!(queued.started, None, "queued work has not started");
    assert_eq!(queued.pid, None, "queued work cannot retain a running PID");
    assert_eq!(app.waiting_task_count(), 3);
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task), TaskRow::WaitingSummary(3)]
            if task.state == TaskState::Queued
    ));

    app.task_filters.state = TaskStateFilter::Waiting;
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task), TaskRow::WaitingSummary(3)]
            if task.state == TaskState::Queued
    ));

    let started = TaskInfo::active(id.clone(), "busybox".into(), "do_compile".into());
    let _ = update(&mut app, Action::TaskStarted(started));
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::WaitingSummary(3)]
    ));
    app.task_filters.state = TaskStateFilter::Active;
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task)] if task.state == TaskState::Active
    ));
}

#[test]
fn live_tasks_filter_supports_recipe_task_worker_and_duration() {
    let mut app = App::new(20, 2_000);
    let mut task = TaskInfo::active(
        TaskId("linux-yocto:do_compile_kernel".into()),
        "linux-yocto".into(),
        "do_compile_kernel".into(),
    );
    task.worker = Some("remote-7".into());
    task.started = Some(SystemTime::UNIX_EPOCH);
    task.finished = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(20));
    task.state = TaskState::Completed;
    app.completed_tasks.push_back(CompletedTask {
        task,
        success: true,
    });
    app.task_filters.recipe = "LINUX".into();
    app.task_filters.task = "kernel".into();
    app.task_filters.worker = "REMOTE".into();
    app.task_filters.minimum_duration = Some(Duration::from_secs(10));
    assert_eq!(app.visible_task_rows().len(), 1);
    app.task_filters.minimum_duration = Some(Duration::from_secs(60));
    assert!(app.visible_task_rows().is_empty());
}

#[test]
fn task_rows_preserve_authoritative_optional_fields_without_derivation() {
    let mut app = App::new(20, 2_000);
    let mut task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.worker = Some("worker-7".into());
    task.pid = Some(4242);
    task.progress = None;
    app.tasks.insert(task.id.clone(), task);

    let rows = app.visible_task_row_refs_at(SystemTime::UNIX_EPOCH);
    assert!(matches!(
        rows.as_slice(),
        [TaskRowRef::Task { task, state: TaskState::Active }]
            if task.worker.as_deref() == Some("worker-7")
                && task.pid == Some(4242)
                && task.progress.is_none()
    ));
}

#[test]
fn build_summary_uses_typed_counts_and_freezes_terminal_elapsed_time() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let mut app = App::new(20, 2_000);
    app.build.status = BuildStatus::Running;
    app.build.started = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(40));
    app.build.completed = 3;
    app.build.total = Some(10);
    app.build.warnings = 2;
    app.build.errors = 1;
    let active = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    app.tasks.insert(active.id.clone(), active);

    let summary = app.build_summary_at(now);
    assert_eq!(summary.completed, 3);
    assert_eq!(summary.total, Some(10));
    assert_eq!(summary.progress_percent(), Some(30));
    assert_eq!(summary.active, 1);
    assert_eq!(summary.waiting, 6);
    assert_eq!(summary.warnings, 2);
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.elapsed, Some(Duration::from_secs(60)));

    app.build.status = BuildStatus::Completed;
    app.build_history.push_back(BuildRecord {
        target: None,
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(75)),
        completed_tasks: 10,
        warnings: 2,
        errors: 1,
    });
    assert_eq!(
        app.build_summary_at(now + Duration::from_secs(900)).elapsed,
        Some(Duration::from_secs(75))
    );

    app.build.total = None;
    assert_eq!(app.build_summary_at(now).progress_percent(), None);
    app.build.total = Some(0);
    assert_eq!(app.build_summary_at(now).progress_percent(), None);
}

#[test]
fn recipe_navigation_uses_authoritative_provider_logs_and_local_patches() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()),
        ..Recipe::default()
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            patches: Some(vec![
                "/layers/meta/recipes-core/busybox/files/a.patch".into(),
                "https://example.invalid/remote.diff".into(),
                "/layers/meta/recipes-core/busybox/files/b.patch".into(),
            ]),
            ..RecipeMetadata::default()
        },
    );
    let mut active = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    active.log_path = Some("/tmp/log.do_compile".into());
    app.tasks.insert(active.id.clone(), active);
    let mut completed = TaskInfo::active(
        TaskId("busybox:do_install".into()),
        "busybox".into(),
        "do_install".into(),
    );
    completed.state = TaskState::Completed;
    completed.log_path = Some("/tmp/log.do_install".into());
    app.completed_tasks.push_back(CompletedTask {
        task: completed,
        success: true,
    });

    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipeProvider),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()
        ))
    );
    assert_eq!(update(&mut app, Action::BeginSelectedRecipeTaskLog), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskLogPicker(picker)) if picker.logs.len() == 2
    ));
    let _ = update(&mut app, Action::SelectRecipeTaskLog { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipeTaskLog),
        Some(Effect::OpenInEditor("/tmp/log.do_install".into()))
    );

    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipePatchReview),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipePatchPicker(picker)) if picker.patches.len() == 2
    ));
    let _ = update(&mut app, Action::SelectRecipePatch { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipePatch),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes-core/busybox/files/b.patch".into()
        ))
    );
}

#[test]
fn recipe_navigation_explains_missing_and_remote_only_paths() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::OpenSelectedRecipeProvider);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("provider path")
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeTaskLog);
    assert!(app.notification.as_deref().unwrap().contains("evicted"));
    app.recipe_metadata.insert(
        "demo".into(),
        RecipeMetadata {
            recipe: "demo".into(),
            patches: Some(vec!["file://unresolved.patch".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipePatchReview);
    assert!(app.notification.as_deref().unwrap().contains("unresolved"));
}

#[test]
fn recipe_qa_action_requires_authoritative_tasks_and_exact_confirmation() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Load selected recipe metadata")
    );
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets,
            task: Some(task),
            force: false,
        })) if targets == &["busybox"] && task == "cve_check"
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cve_check".into()),
            force: false,
        }))
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            task: Some(task),
            ..
        })) if task == "create_spdx"
    ));
    let _ = update(&mut app, Action::CancelRecipeTask);
    app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
    let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Task create_spdx is not reported")
    );
}

#[test]
fn recipe_qa_action_reducer_retains_output_and_honest_empty_artifacts() {
    let mut app = App::new(20, 4_000);
    let id = BackgroundJobId(9);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::CveCheck,
            title: "CVE check busybox".into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("busybox".into()),
                task: Some("cve_check".into()),
                ..BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id,
            entry: BackgroundJobOutputEntry {
                severity: Severity::Warning,
                message: "CVE-2026-0001 requires review".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "CVE check completed; BitBake reported no result path".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.warnings, 1);
    assert_eq!(
        job.output.back().unwrap().message,
        "CVE-2026-0001 requires review"
    );
    assert!(job.result.as_ref().unwrap().artifacts.is_empty());
}

#[test]
fn config_metadata_keeps_global_and_recipe_scopes_typed_and_independent() {
    let mut app = App::new(20, 4_000);
    let global = VariableDetail {
        identity: VariableIdentity {
            name: "PACKAGE_ARCH".into(),
            recipe: None,
        },
        effective_value: Some("qemux86_64".into()),
        unexpanded_value: Some("${MACHINE_ARCH}".into()),
        provenance: Some("/build/conf/local.conf:8".into()),
        operations: vec![VariableOperation {
            operation: "set".into(),
            file: Some("/build/conf/local.conf".into()),
            line: Some(8),
            value: Some("${MACHINE_ARCH}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    };
    let _ = update(&mut app, Action::VariableLoaded(global.clone()));
    assert_eq!(app.workspace.variables["PACKAGE_ARCH"], "qemux86_64");
    assert_eq!(
        app.workspace.variable_provenance_chain["PACKAGE_ARCH"],
        ["/build/conf/local.conf:8"]
    );

    let recipe = VariableDetail {
        identity: VariableIdentity {
            name: "PACKAGE_ARCH".into(),
            recipe: Some("base-files".into()),
        },
        effective_value: Some("all".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    let _ = update(&mut app, Action::VariableLoaded(recipe.clone()));
    assert_eq!(
        app.workspace.variables["PACKAGE_ARCH"], "qemux86_64",
        "a scoped response must not overwrite global summary state"
    );
    assert_eq!(
        app.variable_details
            .get(&recipe.identity)
            .unwrap()
            .effective_value
            .as_deref(),
        Some("all")
    );
    assert_eq!(app.variable_details.get(&global.identity), Some(&global));
}

#[test]
fn devtool_metadata_uses_absolute_identity_and_ignores_other_recipe_status() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "busybox".into(),
            file: Some("/layers/core/recipes-core/busybox/busybox_1.0.bb".into()),
            ..Recipe::default()
        },
        Recipe {
            name: "bash".into(),
            file: Some("/layers/core/recipes-extended/bash/bash_5.0.bb".into()),
            ..Recipe::default()
        },
    ];
    let busybox = match update(&mut app, Action::BeginSelectedRecipeDevtoolStatus) {
        Some(Effect::InspectDevtoolStatus(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert!(app.devtool_status_loading.contains(&busybox));

    let bash = RecipeIdentity {
        name: "bash".into(),
        file: "/layers/core/recipes-extended/bash/bash_5.0.bb".into(),
    };
    let _ = update(
        &mut app,
        Action::DevtoolStatusLoaded(DevtoolStatus {
            identity: bash.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        }),
    );
    assert!(
        app.devtool_status_loading.contains(&busybox),
        "a response for another absolute recipe identity is stale for the selection"
    );
    assert_eq!(app.devtool_statuses[&bash].identity, bash);
    assert!(
        app.devtool_statuses[&bash]
            .disabled_reason(DevtoolAction::ModifyOrEdit)
            .is_none()
    );
    assert_eq!(
        app.devtool_statuses[&bash]
            .disabled_reason(DevtoolAction::UpdateRecipe)
            .as_deref(),
        Some("Recipe is not in the Devtool workspace.")
    );
}

#[test]
fn devtool_metadata_rejects_missing_or_relative_provider_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("recipes-core/busybox.bb".into()),
        ..Recipe::default()
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolStatus),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("The selected recipe provider path is not absolute.")
    );
    assert!(app.devtool_status_loading.is_empty());
}

#[test]
fn config_workspace_lazy_detail_is_identity_correlated_and_search_bounded() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.metadata_query = "machine".into();
    let identity = match update(&mut app, Action::BeginSelectedConfigDetail) {
        Some(Effect::GetVariable(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert_eq!(identity.name, "MACHINE");
    assert_eq!(identity.recipe, None);
    assert!(app.variable_detail_loading.contains(&identity));

    let scoped = VariableDetail {
        identity: VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        },
        effective_value: Some("qemux86-64".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    let _ = update(&mut app, Action::VariableLoaded(scoped.clone()));
    assert!(
        app.variable_detail_loading.contains(&identity),
        "a scoped response must not complete the selected global request"
    );
    assert_eq!(app.variable_details.get(&scoped.identity), Some(&scoped));

    let _ = update(&mut app, Action::SelectConfigVariable { delta: 99 });
    assert_eq!(app.config_selection, 0);
    let _ = update(
        &mut app,
        Action::VariableDetailFailed {
            identity: identity.clone(),
            message: "server unavailable".into(),
        },
    );
    assert!(!app.variable_detail_loading.contains(&identity));
    assert_eq!(
        app.variable_detail_errors
            .get(&identity)
            .map(String::as_str),
        Some("server unavailable")
    );
}

#[test]
fn config_workspace_refresh_preserves_selected_variable_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace.variables.insert("A".into(), "one".into());
    app.workspace.variables.insert("B".into(), "two".into());
    app.config_selection = 1;
    let mut workspace = Workspace::default();
    workspace.variables.insert("B".into(), "updated".into());
    workspace.variables.insert("C".into(), "three".into());
    let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
    assert_eq!(app.config_selection, 0);
    assert_eq!(
        selected_config_identity(&app).map(|identity| identity.name),
        Some("B".into())
    );
}

#[test]
fn config_copy_uses_only_loaded_detail_for_the_exact_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "summary-value".into());
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigEffective),
        None,
        "the summary value must not be copied as authoritative detail"
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("with Enter"))
    );
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigEffective),
        Some(Effect::CopyToClipboard("qemux86-64".into()))
    );
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigUnexpanded),
        Some(Effect::CopyToClipboard("${DEFAULT_MACHINE}".into()))
    );
}

#[test]
fn config_copy_explains_loading_failure_and_absent_unexpanded_value() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_detail_loading.insert(identity.clone());
    assert_eq!(update(&mut app, Action::CopySelectedConfigEffective), None);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("still loading")
    );
    app.variable_detail_loading.clear();
    app.variable_detail_errors
        .insert(identity.clone(), "Tinfoil unavailable".into());
    let _ = update(&mut app, Action::CopySelectedConfigEffective);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Tinfoil unavailable")
    );
    app.variable_detail_errors.clear();
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::CopySelectedConfigUnexpanded);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("unexpanded value")
    );
}

#[test]
fn config_scope_keeps_global_and_recipe_detail_independent() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    let global = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        global.clone(),
        VariableDetail {
            identity: global.clone(),
            effective_value: Some("global-machine".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::OpenConfigScopePicker);
    let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() else {
        panic!("scope picker was not opened");
    };
    assert_eq!(picker.scopes, [None, Some("base-files".into())]);
    let _ = update(&mut app, Action::SelectConfigScope { delta: 1 });
    let scoped = match update(&mut app, Action::ConfirmConfigScope) {
        Some(Effect::GetVariable(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert_eq!(scoped.recipe.as_deref(), Some("base-files"));
    assert!(app.variable_detail_loading.contains(&scoped));
    assert_eq!(
        app.variable_details[&global].effective_value.as_deref(),
        Some("global-machine")
    );
    let _ = update(
        &mut app,
        Action::VariableLoaded(VariableDetail {
            identity: scoped.clone(),
            effective_value: Some("recipe-machine".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        }),
    );
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigEffective),
        Some(Effect::CopyToClipboard("recipe-machine".into()))
    );
}

#[test]
fn config_scope_falls_back_to_global_when_recipe_disappears() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    let _ = update(&mut app, Action::RecipesLoaded(vec![]));
    assert_eq!(app.config_scope, None);
    let _ = update(&mut app, Action::OpenConfigScopePicker);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigScopePicker(picker)) if picker.scopes == [None]
    ));
}

#[test]
fn config_compare_reports_equal_different_and_unavailable_fields() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    for (recipe, effective, unexpanded) in [
        (None, Some("qemux86-64"), Some("${DEFAULT_MACHINE}")),
        (Some("base-files"), Some("qemux86-64"), None),
    ] {
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: recipe.map(str::to_owned),
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: effective.map(str::to_owned),
                unexpanded_value: unexpanded.map(str::to_owned),
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
    }
    let comparison = config_comparison(&app).unwrap();
    assert_eq!(comparison.effective.outcome, ConfigComparisonOutcome::Equal);
    assert_eq!(
        comparison.unexpanded.outcome,
        ConfigComparisonOutcome::Unavailable
    );
    let _ = update(&mut app, Action::OpenConfigComparison);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigComparison(value)) if value == &comparison
    ));
    let _ = update(&mut app, Action::CloseConfigComparison);
    assert!(app.active_dialog().is_none());

    app.variable_details
        .get_mut(&VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        })
        .unwrap()
        .effective_value = Some("qemuarm".into());
    assert_eq!(
        config_comparison(&app).unwrap().effective.outcome,
        ConfigComparisonOutcome::Different
    );
}

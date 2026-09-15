//! Regression tests grouped around config_compare_explains_scope_loading_and_missing_detail.
use super::*;

#[test]
fn config_compare_explains_scope_loading_and_missing_detail() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    assert_eq!(
        config_comparison(&app),
        Err("Select a recipe scope with s before comparing.".into())
    );
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    let scoped = VariableIdentity {
        name: "MACHINE".into(),
        recipe: Some("base-files".into()),
    };
    app.variable_detail_loading.insert(scoped);
    assert!(
        config_comparison(&app)
            .unwrap_err()
            .contains("still loading")
    );
}

#[test]
fn config_edit_preview_requires_allowlisted_loaded_global_detail() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity: identity.clone(),
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::BeginConfigEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigEdit { identity: selected, editor })
            if selected == &identity
                && !editor.editing
                && editor.text.contains("value = \"qemux86-64\"")
                && editor.selected_text() == Some("qemux86-64")
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value == "qemux86-64"
    ));
    if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut() {
        editor.text = "# MACHINE\nvalue = \"qemux86-64\\\"\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewConfigEdit);
    let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() else {
        panic!("confirmation was not opened");
    };
    assert_eq!(request.destination, PathBuf::from("/build/conf/local.conf"));
    assert_eq!(request.assignment, "MACHINE = \"qemux86-64\\\"\"");
    let expected = request.clone();
    assert_eq!(
        update(&mut app, Action::ConfirmConfigEdit),
        Some(Effect::WriteConfigAssignment(expected))
    );
    assert_eq!(app.focus, FocusTarget::Navigator);
}

#[test]
fn config_edit_preview_rejects_read_only_scope_and_control_injection() {
    let mut app = App::new(20, 4_000);
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("BB_NUMBER_THREADS".into(), "8".into());
    let identity = VariableIdentity {
        name: "BB_NUMBER_THREADS".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("8".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::BeginConfigEdit);
    assert!(app.notification.as_deref().unwrap().contains("read-only"));
    assert!(app.active_dialog().is_none());

    assert_eq!(
        config_edit_assignment("MACHINE", "qemu\nMALICIOUS = \"1\""),
        Err("Configuration values cannot contain newlines or control characters.".into())
    );
    app.config_scope = Some("base-files".into());
    assert!(
        config_edit_disabled_reason(&app)
            .unwrap()
            .contains("Recipe-scoped")
    );
}

#[test]
fn config_edit_write_revalidates_request_and_preserves_detail_on_failures() {
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    let request = ConfigEditRequest {
        identity: identity.clone(),
        value: "qemux86-64".into(),
        destination: "/build/conf/local.conf".into(),
        assignment: "MACHINE = \"qemux86-64\"".into(),
    };
    assert_eq!(
        validate_config_edit_request(&request, Path::new("/build")),
        Ok(())
    );

    let mut tampered = request.clone();
    tampered.assignment = "MACHINE = \"injected\"".into();
    assert!(
        validate_config_edit_request(&tampered, Path::new("/build"))
            .unwrap_err()
            .contains("does not match")
    );
    let mut scoped = request.clone();
    scoped.identity.recipe = Some("base-files".into());
    assert!(
        validate_config_edit_request(&scoped, Path::new("/build"))
            .unwrap_err()
            .contains("Recipe-scoped")
    );

    let mut app = App::new(10, 1_000);
    let detail = VariableDetail {
        identity: identity.clone(),
        effective_value: Some("old".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    app.variable_details
        .insert(identity.clone(), detail.clone());
    assert_eq!(
        update(
            &mut app,
            Action::ConfigEditWriteSucceeded {
                identity: identity.clone(),
            },
        ),
        Some(Effect::GetVariable(identity.clone()))
    );
    assert!(app.variable_detail_loading.contains(&identity));
    let _ = update(
        &mut app,
        Action::ConfigEditRefreshFailed {
            identity: identity.clone(),
            message: "bridge unavailable".into(),
        },
    );
    assert_eq!(app.variable_details.get(&identity), Some(&detail));
    assert!(!app.variable_detail_loading.contains(&identity));

    let _ = update(
        &mut app,
        Action::ConfigEditWriteFailed {
            identity: identity.clone(),
            message: "permission denied".into(),
        },
    );
    assert_eq!(app.variable_details.get(&identity), Some(&detail));
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("permission denied")
    );
}

#[test]
fn devtool_job_spec_validates_every_typed_operation() {
    let operations = [
        DevtoolOperation::Modify {
            recipe: "busybox".into(),
        },
        DevtoolOperation::UpdateRecipe {
            recipe: "busybox".into(),
        },
        DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta-custom".into(),
        },
        DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1:/opt".into(),
        },
        DevtoolOperation::UndeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1".into(),
        },
        DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
    ];
    for operation in operations {
        assert_eq!(operation.recipe(), "busybox");
        assert_eq!(operation.validate(), Ok(()));
    }
}

#[test]
fn devtool_job_spec_rejects_ambiguous_tokens_and_relative_finish_destinations() {
    for recipe in ["", "busy box", "busy\nbox", "--help"] {
        assert_eq!(
            DevtoolOperation::Modify {
                recipe: recipe.into(),
            }
            .validate(),
            Err(DevtoolOperationError::InvalidRecipe)
        );
    }
    for target in ["", "root@host /opt", "root@host\n--help", "--help"] {
        assert_eq!(
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: target.into(),
            }
            .validate(),
            Err(DevtoolOperationError::InvalidTarget)
        );
    }
    assert_eq!(
        DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "meta-custom".into(),
        }
        .validate(),
        Err(DevtoolOperationError::RelativeFinishDestination)
    );
}

#[test]
fn devtool_job_lifecycle_retains_typed_output_and_outcome_across_navigation() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1_u64 << 63);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::Devtool,
            title: "Devtool reset busybox".into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("busybox".into()),
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
                severity: Severity::Info,
                message: "workspace reset".into(),
                source: BackgroundJobOutputSource::Stderr,
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(&mut app, Action::Open(Screen::Dashboard));
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "Devtool completed successfully".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.context.recipe.as_deref(), Some("busybox"));
    assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
    assert!(job.output[0].truncated);
    assert_eq!(app.screen, Screen::Dashboard);
}

#[test]
fn signature_workspace_uses_authoritative_task_picker_and_exact_provider_identity() {
    let provider = PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb");
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some(provider.clone()),
        ..Recipe::default()
    });

    let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
    assert_eq!(
        app.notification.as_deref(),
        Some("Load authoritative recipe tasks with Enter before inspecting signatures.")
    );
    app.notification = None;
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec![
                "do_fetch".into(),
                "bad task".into(),
                "do_compile".into(),
                "do_compile".into(),
            ]),
            ..RecipeMetadata::default()
        },
    );

    let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
    let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() else {
        panic!("signature task picker was not opened");
    };
    assert_eq!(picker.recipe.name, "busybox");
    assert_eq!(picker.recipe.file, provider);
    assert_eq!(picker.tasks, ["do_compile", "do_fetch"]);
    assert_eq!(app.focus, FocusTarget::Dialog);

    let _ = update(&mut app, Action::SelectSignatureTask { delta: 1 });
    assert_eq!(
        update(&mut app, Action::ConfirmSignatureTask),
        Some(Effect::GetSignatureDump(SignatureTarget {
            recipe: "busybox".into(),
            task: "do_fetch".into(),
        }))
    );
    assert_eq!(app.screen, Screen::Signatures);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        update(&mut app, Action::LeaveSignatureWorkspace),
        Some(Effect::CancelSignatureOperation)
    );

    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_fetch".into(),
    };
    let record = signature_record(
        "busybox",
        "do_fetch",
        "aaa",
        "/build/tmp/stamps/busybox/do_fetch.sigdata.aaa",
    );
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target,
            records: vec![record],
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSignatureProvider),
        Some(Effect::OpenInEditor(provider))
    );
    let _ = update(&mut app, Action::LeaveSignatureWorkspace);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 0);
}

#[test]
fn signature_workspace_refresh_comparison_and_stale_results_remain_correlated() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = signature_record(
        "busybox",
        "do_compile",
        "aaa",
        "/build/tmp/stamps/busybox/do_compile.sigdata.aaa",
    );
    let right = signature_record(
        "busybox",
        "do_compile",
        "bbb",
        "/build/tmp/stamps/busybox/do_compile.sigdata.bbb",
    );
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Signatures;
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![left.clone(), right.clone()],
        },
    );
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
    );
    let request = SignatureComparisonRequest {
        left: left.identity,
        right: right.identity,
    };
    assert_eq!(
        update(&mut app, Action::BeginSignatureComparison),
        Some(Effect::CompareSignatures(request.clone()))
    );
    assert_eq!(
        update(&mut app, Action::RefreshSignatureDump),
        None,
        "refresh is inert while a comparison is loading"
    );
    let stale = SignatureComparisonRequest {
        left: request.right.clone(),
        right: request.left.clone(),
    };
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: stale,
            differences: Vec::new(),
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request,
            differences: Vec::new(),
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::AvailableEmpty { .. }
    ));
    assert_eq!(
        update(&mut app, Action::RefreshSignatureDump),
        Some(Effect::GetSignatureDump(target))
    );
}

#[test]
fn pkgdata_model_reducer_correlates_inventory_states_search_and_selection() {
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::BeginPackageInventory),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 1
        }))
    );
    let request = PackageInventoryRequest { generation: 1 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request: PackageInventoryRequest { generation: 99 },
            packages: vec![package_summary("stale", "stale")],
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::Loading { request }
    );
    let mut invalid_field = package_summary("libc6", "glibc");
    invalid_field.provider = PackageField::Available("relative.bb".into());
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![
                package_summary("busybox", "busybox"),
                invalid_field,
                package_summary("busybox", "zzz"),
            ],
        },
    );
    assert!(matches!(
        app.package_inventory,
        PackageInventoryState::Partial { .. }
    ));
    assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

    let _ = update(&mut app, Action::BeginPackageSearch);
    let _ = update(&mut app, Action::AppendPackageQuery('G'));
    let _ = update(&mut app, Action::AppendPackageQuery('L'));
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
    assert_eq!(app.filtered_packages().len(), 1);
    let _ = update(&mut app, Action::BackspacePackageQuery);
    let _ = update(&mut app, Action::FinishPackageSearch);
    assert!(!app.package_searching);

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 2 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![package_summary("libc6", "glibc")],
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 3 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: Vec::new(),
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::AvailableEmpty { request }
    );
    assert_eq!(app.package_selection, None);

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 4 };
    let _ = update(
        &mut app,
        Action::PackageInventoryFailed {
            request,
            message: "pkgdata missing".into(),
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::Failed {
            request,
            message: "pkgdata missing".into()
        }
    );
}

#[test]
fn pkgdata_model_detail_states_and_dependency_navigation_are_exact() {
    let mut app = App::new(10, 1_000);
    let inventory_request = PackageInventoryRequest { generation: 1 };
    app.package_inventory = PackageInventoryState::Loading {
        request: inventory_request,
    };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request: inventory_request,
            packages: vec![
                package_summary("busybox", "busybox"),
                package_summary("libc6", "glibc"),
                package_summary("init", "init"),
            ],
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedPackageDetail),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 1,
        }))
    );
    let request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 1,
    };
    let detail = PackageDetail {
        identity: request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Available(vec![PackageIdentity::new("init")]),
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: PackageDetailRequest {
                generation: 99,
                ..request.clone()
            },
            detail: detail.clone(),
        },
    );
    assert!(matches!(
        app.selected_package_detail(),
        Some(PackageDetailState::Loading { .. })
    ));
    let _ = update(
        &mut app,
        Action::PackageDetailPartial {
            request: request.clone(),
            detail,
            limitations: vec!["license unavailable".into()],
        },
    );
    assert!(matches!(
        app.selected_package_detail(),
        Some(PackageDetailState::Partial { .. })
    ));

    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("libc6"),
            reverse: false,
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
    app.package_selection = Some(PackageIdentity::new("busybox"));
    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("init"),
            reverse: true,
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
    app.package_selection = Some(PackageIdentity::new("busybox"));
    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("not-present"),
            reverse: false,
        },
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("not in the current typed detail"))
    );

    app.package_selection = Some(PackageIdentity::new("libc6"));
    let effect = update(&mut app, Action::BeginSelectedPackageDetail).unwrap();
    let Effect::GetPackageDetail(empty_request) = effect else {
        panic!("expected package detail effect");
    };
    let empty = PackageDetail {
        identity: empty_request.identity.clone(),
        files: PackageField::Available(Vec::new()),
        runtime_dependencies: PackageField::Available(Vec::new()),
        reverse_dependencies: PackageField::Available(Vec::new()),
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: empty_request.clone(),
            detail: empty,
        },
    );
    assert_eq!(
        app.package_details.get(&empty_request.identity),
        Some(&PackageDetailState::AvailableEmpty {
            request: empty_request.clone()
        })
    );

    app.package_selection = Some(PackageIdentity::new("init"));
    let Effect::GetPackageDetail(failed_request) =
        update(&mut app, Action::BeginSelectedPackageDetail).unwrap()
    else {
        panic!("expected package detail effect");
    };
    let _ = update(
        &mut app,
        Action::PackageDetailFailed {
            request: failed_request.clone(),
            message: "tool failed".into(),
        },
    );
    assert_eq!(
        app.package_details.get(&failed_request.identity),
        Some(&PackageDetailState::Failed {
            request: failed_request,
            message: "tool failed".into()
        })
    );
}

#[test]
fn pkgdata_workspace_routes_navigation_refresh_detail_and_contextual_actions() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox.bb".into()),
        ..Recipe::default()
    });
    assert_eq!(
        update(&mut app, Action::Open(Screen::Packages)),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 1
        }))
    );
    assert_eq!(app.screen, Screen::Packages);
    assert_eq!(NAVIGATOR_SCREENS[app.navigator_selection], Screen::Packages);
    assert_eq!(
        update(&mut app, Action::CancelPackageOperation),
        Some(Effect::CancelPackageOperation)
    );
    let request = PackageInventoryRequest { generation: 1 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![
                package_summary("busybox", "busybox"),
                package_summary("init", "init"),
                package_summary("libc6", "glibc"),
            ],
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedPackageDetail),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 2,
        }))
    );
    let detail_request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 2,
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: detail_request.clone(),
            detail: PackageDetail {
                identity: detail_request.identity,
                files: PackageField::Available(vec!["/bin/busybox".into()]),
                runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
                reverse_dependencies: PackageField::Available(vec![PackageIdentity::new("init")]),
            },
        },
    );
    assert_eq!(
        app.selected_package_dependency(),
        Some(&PackageIdentity::new("libc6"))
    );
    let _ = update(&mut app, Action::TogglePackageDependencyKind);
    assert_eq!(
        app.selected_package_dependency(),
        Some(&PackageIdentity::new("init"))
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedPackageDependency),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("init"),
            generation: 3,
        }))
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
    let _ = update(&mut app, Action::BackPackageNavigation);
    assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

    assert_eq!(
        update(&mut app, Action::OpenSelectedPackageProvider),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes/busybox.bb".into()
        ))
    );
    let _ = update(&mut app, Action::OpenSelectedPackageRecipe);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 0);

    app.package_details.clear();
    app.screen = Screen::Packages;
    assert_eq!(
        update(&mut app, Action::RefreshPackageInventory),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 4
        }))
    );
}

//! Regression tests grouped around image_artifact_model_correlates_states_search_and_stable_selection.
use super::*;

#[test]
fn image_artifact_model_correlates_states_search_and_stable_selection() {
    let make_artifact = |image: &str, suffix: &str, kind| ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: image.into(),
            path: format!("/build/tmp/deploy/images/qemux86-64/{image}.{suffix}").into(),
        },
        kind,
        size_bytes: ImageArtifactField::Available(4_096),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Available(Vec::new()),
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    let inventory = |artifacts| ImageArtifactInventory {
        machine: "qemux86-64".into(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts,
    };

    let mut app = App::new(20, 20_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let request = ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    assert_eq!(
        update(&mut app, Action::BeginImageArtifactInventory),
        Some(Effect::GetImageArtifacts(request.clone()))
    );
    let minimal = make_artifact(
        "core-image-minimal",
        "rootfs.ext4",
        ImageArtifactKind::RootFilesystem,
    );
    let sato = make_artifact("core-image-sato", "wic", ImageArtifactKind::Wic);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: request.clone(),
            inventory: inventory(vec![sato.clone(), minimal.clone()]),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Available { .. }
    ));
    assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
    let _ = update(&mut app, Action::SelectImageArtifact { delta: 1 });
    assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

    assert_eq!(
        update(&mut app, Action::RefreshImageArtifactInventory),
        Some(Effect::GetImageArtifacts(ImageArtifactRequest {
            generation: 2,
            machine: "qemux86-64".into(),
        }))
    );
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request,
            inventory: inventory(vec![minimal.clone()]),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Loading { .. }
    ));
    let request = ImageArtifactRequest {
        generation: 2,
        machine: "qemux86-64".into(),
    };
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryPartial {
            request: request.clone(),
            inventory: inventory(vec![minimal.clone(), sato.clone()]),
            limitations: vec!["checksum metadata unavailable".into()],
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Partial { .. }
    ));
    assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

    let _ = update(&mut app, Action::BeginImageArtifactSearch);
    let _ = update(&mut app, Action::AppendImageArtifactQuery('m'));
    let _ = update(&mut app, Action::AppendImageArtifactQuery('i'));
    let _ = update(&mut app, Action::AppendImageArtifactQuery('n'));
    assert_eq!(app.filtered_image_artifacts(), vec![&minimal]);
    assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
    let _ = update(&mut app, Action::FinishImageArtifactSearch);

    app.image_artifact_query.clear();
    let failed_request = ImageArtifactRequest {
        generation: 3,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryFailed {
            request: failed_request.clone(),
            message: "deploy directory is unavailable".into(),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Failed { ref request, .. } if request == &failed_request
    ));

    let empty_request = ImageArtifactRequest {
        generation: 4,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: empty_request,
            inventory: inventory(Vec::new()),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::AvailableEmpty { .. }
    ));
    assert_eq!(app.image_artifact_selection, None);

    let invalid_request = ImageArtifactRequest {
        generation: 5,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: invalid_request,
            inventory: ImageArtifactInventory {
                machine: "qemuarm64".into(),
                deploy_directory: ImageArtifactField::Unavailable,
                artifacts: Vec::new(),
            },
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Failed { .. }
    ));
}

#[test]
fn images_workspace_preserves_build_and_routes_exact_typed_paths() {
    let mut app = App::new(20, 20_000);
    app.workspace.recipes.push(Recipe {
        name: "core-image-minimal".into(),
        ..Recipe::default()
    });
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let request = ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    assert_eq!(
        update(&mut app, Action::Open(Screen::Images)),
        Some(Effect::GetImageArtifacts(request.clone()))
    );
    let artifact_path = PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic");
    let manifest_path =
        PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest");
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: artifact_path.clone(),
        },
        kind: ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Available(vec![manifest_path.clone()]),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(vec![artifact_path.clone()]),
    };
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request,
            inventory: ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available(
                    "/build/tmp/deploy/images/qemux86-64".into(),
                ),
                artifacts: vec![artifact],
            },
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedImageArtifact),
        Some(Effect::OpenInEditor(artifact_path.clone()))
    );
    assert_eq!(
        update(
            &mut app,
            Action::OpenSelectedImageArtifactAssociation(ImageArtifactAssociation::Manifest)
        ),
        Some(Effect::OpenInEditor(manifest_path))
    );
    let _ = update(&mut app, Action::BeginSelectedImageArtifactBuild);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest { targets, .. }))
            if targets == &vec!["core-image-minimal".to_owned()]
    ));
}

#[test]
fn image_artifact_build_rejects_non_recipe_deploy_outputs() {
    let mut app = App::new(20, 20_000);
    app.workspace.recipes.push(Recipe {
        name: "core-image-minimal".into(),
        ..Recipe::default()
    });
    app.build.target = Some("core-image-minimal".into());
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "bzImage--6.18.24-r0".into(),
            path: "/build/tmp/deploy/images/qemux86-64/bzImage--6.18.24-r0.bin".into(),
        },
        kind: ImageArtifactKind::Kernel,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: artifact.identity.machine.clone(),
        },
        inventory: ImageArtifactInventory {
            machine: artifact.identity.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact],
        },
    };

    assert_eq!(
        update(&mut app, Action::BeginSelectedImageArtifactBuild),
        None
    );
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(app.active_dialog().is_none());
    assert!(app.notification.as_deref().is_some_and(|message| {
        message.contains("deployed artifact, not a buildable image recipe")
            && message.contains("Select an image recipe with i")
    }));
}

#[test]
fn ux_rootfs_reducer_correlates_generation_lifecycle_and_stable_drilldown_selection() {
    let mut app = App::new(20, 20_000);
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4".into(),
        },
        kind: ImageArtifactKind::RootFilesystem,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    };
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: artifact.identity.machine.clone(),
        },
        inventory: ImageArtifactInventory {
            machine: artifact.identity.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact.clone()],
        },
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    let request = RootfsCompositionRequest {
        generation: 1,
        image: artifact.identity.clone(),
    };
    assert_eq!(
        update(&mut app, Action::BeginSelectedRootfsComposition),
        Some(Effect::GetRootfsComposition(request.clone()))
    );
    assert_eq!(app.images_view, ImagesView::RootfsPackages);
    let package = |name: &str, category: &str| RootfsInstalledPackage {
        identity: PackageIdentity::new(name),
        recipe: Some(name.into()),
        category: category.into(),
        installed_size_bytes: 100,
        file_count: 2,
    };
    let composition = RootfsComposition {
        image: artifact.identity.clone(),
        installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
            packages: vec![package("busybox", "base"), package("glibc", "runtime")],
        }),
        filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree {
            entries: vec![
                RootfsEntry {
                    identity: RootfsPathIdentity("/".into()),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
                RootfsEntry {
                    identity: RootfsPathIdentity("/usr".into()),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
            ],
        }),
        system_inventory: RootfsAuthority::Available(RootfsSystemInventory::default()),
        root_directory: Some("/build/tmp/rootfs".into()),
    };
    let stale = RootfsCompositionRequest {
        generation: 99,
        image: artifact.identity.clone(),
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: stale,
            composition: composition.clone(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: request.clone(),
            composition: composition.clone(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Available { .. }
    ));
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("base".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("busybox"))
    );
    let _ = update(&mut app, Action::SelectRootfsEntry { delta: 1 });
    assert_eq!(
        app.rootfs_entry_selection,
        Some(RootfsPathIdentity("/usr".into()))
    );
    let _ = update(&mut app, Action::SelectRootfsGroup { delta: 1 });
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("runtime".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("glibc"))
    );

    let Effect::GetRootfsComposition(refresh) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    assert_eq!(refresh.generation, 2);
    assert_eq!(refresh.image, artifact.identity);
    let _ = update(
        &mut app,
        Action::RootfsCompositionPartial {
            request: refresh,
            composition,
            limitations: vec!["pkgdata sizes partial".into()],
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Partial { .. }
    ));
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("runtime".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("glibc"))
    );
    assert_eq!(
        app.rootfs_entry_selection,
        Some(RootfsPathIdentity("/usr".into()))
    );

    let Effect::GetRootfsComposition(empty_request) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: empty_request,
            composition: RootfsComposition {
                image: artifact.identity.clone(),
                installed_packages: RootfsAuthority::Available(RootfsPackageInventory::default()),
                filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree::default()),
                system_inventory: RootfsAuthority::Available(RootfsSystemInventory::default()),
                root_directory: None,
            },
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::AvailableEmpty { .. }
    ));
    assert_eq!(app.rootfs_group_selection, None);
    assert_eq!(app.rootfs_entry_selection, None);

    let Effect::GetRootfsComposition(unavailable) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionUnavailable {
            request: unavailable,
            reason: "manifest absent".into(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Unavailable { .. }
    ));

    let Effect::GetRootfsComposition(failed) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionFailed {
            request: failed,
            message: "adapter failed".into(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Failed { .. }
    ));
}

#[test]
fn image_console_reducer_launches_qemu_or_ssh_through_typed_terminal_effects() {
    let mut qemu = qemu_model_app();
    let _ = update(&mut qemu, Action::BeginSelectedImageConsole);
    assert!(matches!(
        qemu.active_dialog(),
        Some(Dialog::ImageConsole(dialog)) if dialog.draft.mode == ImageConsoleMode::Qemu
    ));
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        ..
    })) = update(&mut qemu, Action::ConfirmImageConsole)
    else {
        panic!("expected QEMU terminal creation");
    };
    assert_eq!(kind, TerminalCreationKind::QemuConsole);
    assert_eq!(program, PathBuf::from("/opt/poky/scripts/runqemu"));
    assert!(arguments.iter().any(|argument| argument == "nographic"));
    assert!(arguments.iter().any(|argument| argument == "serialstdio"));
    assert_eq!(qemu.screen, Screen::TerminalSessions);

    let mut ssh = qemu_model_app();
    ssh.qemu_capability = QemuCapability::MissingTool;
    ssh.ssh_client_capability = SshClientCapability::Available {
        executable: "/usr/bin/ssh".into(),
    };
    let _ = update(&mut ssh, Action::BeginSelectedImageConsole);
    let Some(Dialog::ImageConsole(dialog)) = ssh.active_dialog_mut() else {
        panic!("expected Image Console dialog");
    };
    assert_eq!(dialog.draft.mode, ImageConsoleMode::Ssh);
    dialog.draft.host = "target.example".into();
    dialog.draft.user = "root".into();
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        ..
    })) = update(&mut ssh, Action::ConfirmImageConsole)
    else {
        panic!("expected SSH terminal creation");
    };
    assert_eq!(kind, TerminalCreationKind::SshConsole);
    assert_eq!(program, PathBuf::from("/usr/bin/ssh"));
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("root@target.example")
    );
}

#[test]
fn image_console_advanced_qemu_preview_preserves_argv_and_rejects_stale_authority() {
    let mut app = qemu_model_app();
    update(&mut app, Action::BeginSelectedQemuLaunch);
    update(&mut app, Action::PreviewQemuLaunch);
    let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
        panic!("missing preview");
    };
    let expected = preview.argv.clone();
    let mut stale = app.clone();
    stale.qemu_capability = QemuCapability::MissingTool;
    assert_eq!(
        update(&mut stale, Action::ConfirmQemuLaunchInTerminal),
        None
    );
    assert!(stale.active_dialog().is_some());
    let mut changed = app.clone();
    if let Some(Dialog::QemuLaunchConfirmation(value)) = changed.active_dialog_mut() {
        value.argv.push("unapproved".into());
    }
    assert_eq!(
        update(&mut changed, Action::ConfirmQemuLaunchInTerminal),
        None
    );
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        cwd,
        ..
    })) = update(&mut app, Action::ConfirmQemuLaunchInTerminal)
    else {
        panic!("missing terminal effect");
    };
    assert_eq!(kind, TerminalCreationKind::QemuConsole);
    assert_eq!(cwd, PathBuf::from("/build"));
    assert_eq!(program, expected[0]);
    assert_eq!(
        arguments,
        expected[1..]
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert!(app.qemu_sessions.is_empty());
}

#[test]
fn image_console_reducer_keeps_invalid_input_open_and_cancel_is_no_spawn() {
    let mut app = qemu_model_app();
    app.qemu_capability = QemuCapability::MissingTool;
    app.ssh_client_capability = SshClientCapability::Available {
        executable: "/usr/bin/ssh".into(),
    };
    let _ = update(&mut app, Action::BeginSelectedImageConsole);
    assert_eq!(update(&mut app, Action::ConfirmImageConsole), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ImageConsole(dialog)) if dialog.validation_error.as_deref().is_some_and(|message| message.contains("SSH host"))
    ));
    assert_eq!(update(&mut app, Action::CancelImageConsole), None);
    assert!(app.active_dialog().is_none());
}

#[test]
fn qemu_model_validates_exact_launch_identity_paths_and_options() {
    let artifact = qemu_model_artifact();
    let capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![artifact.identity.clone()],
    };
    let draft = QemuLaunchDraft::for_artifact(artifact.identity.clone(), artifact.kind);
    let first = draft.preview(&capability).expect("valid preview");
    let second = draft.preview(&capability).expect("deterministic preview");
    assert_eq!(first, second);
    assert_eq!(first.request.memory_mib, 1024);

    let mut invalid = draft.clone();
    invalid.machine = "other-machine".into();
    assert_eq!(
        invalid.preview(&capability),
        Err("runqemu machine and image identities must match")
    );
    invalid = draft.clone();
    invalid.rootfs = "relative/rootfs.ext4".into();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft.clone();
    invalid.memory_mib = (MAX_QEMU_MEMORY_MIB + 1).to_string();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft.clone();
    invalid.extra_arguments = "-- -display none".into();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft;
    invalid.artifact_kind = ImageArtifactKind::Manifest;
    assert!(invalid.preview(&capability).is_err());
}

#[test]
fn qemu_model_reducer_previews_confirms_and_bounds_session_output() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert!(matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunchConfirmation(_))
    ));
    let effect = update(&mut app, Action::ConfirmQemuLaunch);
    let Some(Effect::StartQemuSession { id, request }) = effect else {
        panic!("expected typed runqemu start effect");
    };
    assert_eq!(request.image, qemu_model_artifact().identity);
    let session = app.qemu_session(id).expect("session");
    let job_id = session.background_job_id;
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Queued)
    );

    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert_eq!(
        app.notification.as_deref(),
        Some("A managed runqemu session is already active.")
    );
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    for index in 0..600 {
        let _ = update(
            &mut app,
            Action::AppendQemuSessionOutput {
                id,
                stream: if index % 2 == 0 {
                    QemuOutputStream::Stdout
                } else {
                    QemuOutputStream::Stderr
                },
                line: format!("line {index}"),
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
    }
    let job = app.background_jobs.get(job_id).expect("job");
    assert_eq!(job.status, BackgroundJobStatus::Running);
    assert_eq!(job.output.len(), MAX_BACKGROUND_JOB_OUTPUT_ENTRIES);
    assert_eq!(job.dropped_output_entries, 88);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.source == BackgroundJobOutputSource::Stderr)
    );
}

#[test]
fn qemu_model_requires_cancellation_confirmation_and_rejects_stale_events() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
    else {
        panic!("expected start");
    };
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmQemuSessionCancellation),
        Some(Effect::CancelQemuSession(id))
    );
    let job_id = app.qemu_session(id).expect("session").background_job_id;
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Cancelling)
    );
    let _ = update(
        &mut app,
        Action::RejectQemuSessionCancellation {
            id,
            message: "signal failed".into(),
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
    let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
    let _ = update(&mut app, Action::ConfirmQemuSessionCancellation);
    let _ = update(
        &mut app,
        Action::CancelQemuSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Cancelled)
    );
    assert_eq!(
        app.qemu_session(id).and_then(|session| session.exit_code),
        Some(130)
    );

    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::QemuSessionRunning {
            id: QemuSessionId(99_999),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}

#[test]
fn qemu_workspace_dialog_fields_are_bounded_modal_and_validation_aware() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            selected_field: QemuLaunchField::Machine,
            editing: false,
            ..
        }))
    ));
    let _ = update(&mut app, Action::ActivateQemuLaunchField);
    assert_eq!(
        app.notification.as_deref(),
        Some("Image and machine identity are read-only.")
    );
    let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
    let _ = update(&mut app, Action::ActivateQemuLaunchField);
    for _ in 0..(MAX_QEMU_PATH_INPUT_BYTES + 10) {
        let _ = update(&mut app, Action::AppendQemuLaunchField('x'));
    }
    let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() else {
        panic!("launch dialog");
    };
    assert_eq!(dialog.draft.kernel.len(), MAX_QEMU_PATH_INPUT_BYTES);
    assert!(dialog.editing);
    let _ = update(&mut app, Action::FinishQemuLaunchFieldEdit);
    let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
    let _ = update(&mut app, Action::CycleQemuLaunchChoice { backwards: false });
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            draft: QemuLaunchDraft {
                networking: QemuNetworkingMode::Tap,
                ..
            },
            ..
        }))
    ));
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            validation_error: Some(message),
            ..
        })) if message.contains("paths must be normalized")
    ));
    let _ = update(&mut app, Action::CancelQemuLaunch);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);
}

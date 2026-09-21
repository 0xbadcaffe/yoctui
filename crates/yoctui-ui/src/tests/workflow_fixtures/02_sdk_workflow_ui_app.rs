pub(crate) fn sdk_workflow_ui_app() -> App {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Sdk;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.workspace
        .variables
        .insert("SDK_DEPLOY".into(), "/deploy/sdk".into());
    app.build.target = Some("core-image-minimal".into());
    let installer = yoctui_model::SdkArtifact {
        identity: yoctui_model::SdkArtifactIdentity {
            path: "/deploy/sdk/poky-core-image-minimal-toolchain.sh".into(),
            size_bytes: 8_192,
            modified_unix_seconds: 1_700_000_000,
        },
        kind: SdkArtifactKind::Installer,
        sdk_kind: Some(SdkKind::Standard),
        machine: Some("qemux86-64".into()),
        host_tuple: Some("x86_64-pokysdk-linux".into()),
        target_tuple: Some("x86_64-poky-linux".into()),
        checksums: vec!["/deploy/sdk/poky-core-image-minimal-toolchain.sh.sha256".into()],
        manifests: vec!["/deploy/sdk/poky-core-image-minimal-toolchain.target.manifest".into()],
        published: None,
    };
    app.sdk_artifact_selection = Some(installer.identity.clone());
    app.sdk_artifacts = SdkArtifactInventoryState::Available {
        request: yoctui_model::SdkArtifactInventoryRequest {
            generation: 1,
            root: "/deploy/sdk".into(),
            machine: "qemux86-64".into(),
        },
        artifacts: vec![installer],
    };
    app.sdk_tool_capability = SdkToolCapability::Available {
        publish: Some("/workspace/scripts/oe-publish-sdk".into()),
        find_sysroot: Some("/workspace/scripts/oe-find-native-sysroot".into()),
        run_native: Some("/workspace/scripts/oe-run-native".into()),
    };
    app
}

pub(crate) fn sdk_workflow_running_ui_app() -> (App, SdkSessionId) {
    let mut app = sdk_workflow_ui_app();
    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "destination = \"/srv/sdk-publish\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let Some(yoctui_model::Effect::StartSdkSession { id, .. }) =
        update(&mut app, Action::ConfirmSdkPublish)
    else {
        panic!("expected managed SDK session");
    };
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id });
    app.focus = FocusTarget::Workspace;
    (app, id)
}

pub(crate) fn test_workflow_result(
    suffix: &str,
    outcome: yoctui_model::TestCaseOutcome,
) -> yoctui_model::TestResultRecord {
    let path = PathBuf::from(format!("/results/{suffix}/testresults.json"));
    let identity = yoctui_model::TestResultIdentity::new(
        path,
        128,
        SystemTime::UNIX_EPOCH,
        format!("{suffix}fingerprint"),
    )
    .unwrap();
    let case_identity =
        yoctui_model::TestCaseIdentity::new("runtime".into(), "Case.test_one".into()).unwrap();
    let (case, _) = yoctui_model::TestCaseRecord::new(
        case_identity,
        outcome,
        Some(std::time::Duration::from_millis(1250)),
        vec![yoctui_model::TestMetadata::new("result".into(), suffix.into()).unwrap()],
        Some(PathBuf::from(format!("/logs/{suffix}.log"))),
    )
    .unwrap();
    let (suite, _) =
        yoctui_model::TestSuiteRecord::new("runtime".into(), None, Vec::new(), vec![case]).unwrap();
    yoctui_model::TestResultRecord::new(
        identity,
        Some(yoctui_model::TestFamily::TestImage),
        Some("qemux86-64".into()),
        Some("core-image-minimal".into()),
        Some("revision-1".into()),
        Some(std::time::Duration::from_secs(2)),
        vec![yoctui_model::TestMetadata::new("DISTRO".into(), "poky".into()).unwrap()],
        vec![suite],
        Some(yoctui_model::TestSessionId(3)),
        vec!["fixture limitation".into()],
    )
    .0
}

pub(crate) fn test_workflow_results_app() -> (
    App,
    yoctui_model::TestResultRecord,
    yoctui_model::TestResultRecord,
) {
    let baseline = test_workflow_result("baseline", yoctui_model::TestCaseOutcome::Passed);
    let candidate = test_workflow_result("candidate", yoctui_model::TestCaseOutcome::Failed);
    let request = yoctui_model::TestResultImportRequest::new(
        1,
        vec![
            baseline.identity.path.clone(),
            candidate.identity.path.clone(),
        ],
    )
    .unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Testing;
    app.focus = FocusTarget::Workspace;
    app.test_view = TestWorkspaceView::Results;
    app.result_tool_capability =
        yoctui_model::ResultToolCapability::Available("/workspace/resulttool".into());
    app.test_result_selection = Some(candidate.identity.clone());
    app.test_results = TestResultInventoryState::Partial {
        request,
        records: vec![baseline.clone(), candidate.clone()],
        limitations: vec!["one malformed result was skipped".into()],
    };
    (app, baseline, candidate)
}

pub(crate) fn qemu_workspace_app() -> App {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/qemux86-64/core-image-minimal.wic".into(),
    };
    let artifact = yoctui_model::ImageArtifact {
        identity: identity.clone(),
        kind: yoctui_model::ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(8_192),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(vec![identity.path.clone()]),
    };
    app.image_artifact_selection = Some(identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
            artifacts: vec![artifact],
        },
    };
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![identity],
    };
    app
}

pub(crate) fn qemu_running_workspace_app() -> (App, QemuSessionId) {
    let mut app = qemu_workspace_app();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
    let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
        yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
    else {
        panic!("expected session");
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
    app.focus = FocusTarget::Workspace;
    (app, id)
}

pub(crate) fn wic_workspace_app() -> App {
    let mut app = qemu_workspace_app();
    app.wic_capability = WicCapability::Available {
        executable: "/opt/poky/scripts/wic".into(),
        kickstarts: vec![yoctui_model::WicKickstart {
            identity: yoctui_model::WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some("/layers/meta/wic/directdisk.wks".into()),
            },
            source: "part / --source=rootfs --fstype=ext4 --size=64".into(),
            partitions: vec![yoctui_model::WicPartitionSummary {
                mount_point: Some("/".into()),
                filesystem: Some("ext4".into()),
                source_plugin: Some("rootfs".into()),
                size_mib: Some(64),
                alignment_kib: None,
            }],
            limitations: vec!["dynamic boot size".into()],
        }],
        image_targets: vec!["core-image-minimal".into()],
    };
    app
}

pub(crate) fn wic_running_workspace_app() -> (App, WicSessionId) {
    let mut app = wic_workspace_app();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
    let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
        yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicCreate)
    else {
        panic!("expected Wic session");
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
    (app, id)
}

pub(crate) fn qa_workflow_ui_app() -> App {
    let scope = yoctui_model::QaScope::new(RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
    })
    .unwrap();
    let check = yoctui_model::QaCheckId::new("recipe-package-busybox".into()).unwrap();
    let report_identity = yoctui_model::QaReportIdentity::new(
        "/build/tmp/log/qa/busybox.json".into(),
        512,
        SystemTime::UNIX_EPOCH,
        "reportfingerprint".into(),
        yoctui_model::QaReportFormat::Json,
        Some(check.clone()),
        Some(yoctui_model::QaFindingScope::Recipe(scope.clone())),
    )
    .unwrap();
    let finding_identity =
        yoctui_model::QaFindingIdentity::new(check.clone(), "findingfingerprint".into()).unwrap();
    let finding = yoctui_model::QaFinding {
        identity: finding_identity.clone(),
        status: yoctui_model::QaFindingStatus::Failed,
        severity: Some("error".into()),
        message: "installed-vs-shipped mismatch".into(),
        scope: yoctui_model::QaFindingScope::Recipe(scope.clone()),
        task: Some("do_package_qa".into()),
        test_name: None,
        source: Some(
            yoctui_model::QaSourceLocation::new(
                "/layers/meta/classes-global/insane.bbclass".into(),
                Some(42),
                None,
            )
            .unwrap(),
        ),
        rule: Some("installed-vs-shipped".into()),
        suggestion: Some("add the installed file to FILES".into()),
        metadata: vec![yoctui_model::QaMetadata::new("package".into(), "busybox".into()).unwrap()],
    };
    let report = yoctui_model::QaReport {
        identity: report_identity.clone(),
        findings: vec![finding],
        metadata: Vec::new(),
        limitations: vec!["one unsupported record was retained".into()],
    };
    let request =
        yoctui_model::QaReportRequest::new(3, vec![report_identity.path.clone()]).unwrap();
    let available = yoctui_model::QaCheckCapability::new(
        check.clone(),
        yoctui_model::QaCheckFamily::RecipePackage,
        "Recipe and package QA".into(),
        scope.clone(),
        Some("do_package_qa".into()),
        vec!["/build/tmp/log/qa".into()],
        yoctui_model::QaCheckAvailability::Available,
        Vec::new(),
    )
    .unwrap();
    let disabled = yoctui_model::QaCheckCapability::new(
        yoctui_model::QaCheckId::new("kernel-configuration-busybox".into()).unwrap(),
        yoctui_model::QaCheckFamily::KernelConfiguration,
        "Kernel configuration".into(),
        scope.clone(),
        None,
        Vec::new(),
        yoctui_model::QaCheckAvailability::Disabled(
            "selected recipe is not a kernel provider".into(),
        ),
        Vec::new(),
    )
    .unwrap();
    let capability = yoctui_model::QaCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        scope.clone(),
        vec![scope.clone()],
        vec![available, disabled],
        vec!["one optional report root was unavailable".into()],
    )
    .unwrap();
    let layer_identity =
        yoctui_model::QaLayerIdentity::new("meta-demo".into(), "/layers/meta-demo".into()).unwrap();
    let executable = yoctui_model::QaExecutableIdentity::new(
        "/workspace/scripts/yocto-check-layer".into(),
        128,
        SystemTime::UNIX_EPOCH,
    )
    .unwrap();
    let arguments = vec![layer_identity.root.display().to_string()];
    let layer = yoctui_model::QaConfiguredLayerCapability::new(
        yoctui_model::QaCheckId::new("layer-meta-demo".into()).unwrap(),
        layer_identity.clone(),
        vec!["walnascar".into()],
        yoctui_model::QaLayerRunCapability::Available {
            executable,
            arguments,
            report_roots: vec!["/build/tmp/log/qa-layer".into()],
        },
        vec!["live compatibility not validated".into()],
    )
    .unwrap();
    let layer_capability = yoctui_model::QaLayerCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        layer_identity.clone(),
        vec![layer],
        Vec::new(),
    )
    .unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Qa;
    app.focus = FocusTarget::Workspace;
    app.qa.scope = Some(scope);
    app.qa.check_selection = Some(check);
    app.qa.capability = yoctui_model::QaCapability::Partial {
        snapshot: Box::new(capability),
        limitations: vec!["one optional report root was unavailable".into()],
    };
    app.qa.inventory = yoctui_model::QaReportInventoryState::Partial {
        request,
        reports: vec![report],
        limitations: vec!["one exact record was malformed".into()],
    };
    app.qa.report_selection = Some(report_identity);
    app.qa.finding_selection = Some(finding_identity);
    app.qa.layer_capability =
        yoctui_model::QaLayerCapability::Available(Box::new(layer_capability));
    app.qa.layer_selection = Some(layer_identity);
    app
}

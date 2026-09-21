#[test]
fn signature_workspace_renders_typed_records_differences_limitations_and_footer() {
    let target = yoctui_model::SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = yoctui_model::SignatureIdentity {
        target: target.clone(),
        hash: Some("aaa".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
    };
    let right = yoctui_model::SignatureIdentity {
        target: target.clone(),
        hash: Some("bbb".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.bbb".into()),
    };
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Signatures;
    app.signature_selection = Some(left.clone());
    app.signature_dump = SignatureDumpState::Partial {
        target,
        records: vec![
            yoctui_model::SignatureRecord {
                identity: left.clone(),
                base_hash: Some("base-aaa".into()),
                task_hash: Some("aaa".into()),
                variables: vec![yoctui_model::SignatureValue {
                    name: "CC".into(),
                    value: Some("gcc".into()),
                }],
                dependencies: vec!["busybox:do_configure=dep-a".into()],
            },
            yoctui_model::SignatureRecord {
                identity: right.clone(),
                base_hash: Some("base-bbb".into()),
                task_hash: Some("bbb".into()),
                variables: Vec::new(),
                dependencies: Vec::new(),
            },
        ],
        limitations: vec!["one malformed artifact was omitted".into()],
    };
    app.signature_comparison = SignatureComparisonState::Partial {
        request: yoctui_model::SignatureComparisonRequest { left, right },
        differences: vec![yoctui_model::SignatureDifference {
            category: SignatureDifferenceCategory::ChangedValue,
            key: "CC".into(),
            left: Some("gcc".into()),
            right: Some("clang".into()),
        }],
        limitations: vec!["recursive detail unavailable".into()],
    };

    let wide = rendered_text(&app, 160, 34);
    assert!(wide.contains("Signatures"), "{wide}");
    assert!(wide.contains("busybox:do_compile"), "{wide}");
    assert!(wide.contains("base-aaa"), "{wide}");
    assert!(wide.contains("CC = gcc"), "{wide}");
    assert!(wide.contains("[value] CC: gcc"), "{wide}");
    assert!(wide.contains("one malformed artifact"), "{wide}");
    assert!(wide.contains("recursive detail unavailable"), "{wide}");
    assert!(wide.contains("F12 Menu"), "{wide}");
    let contextual_footer = rendered_text(&app, 120, 34);
    assert!(
        contextual_footer.contains("1/2 sides"),
        "{contextual_footer}"
    );

    let narrow = rendered_text(&app, 90, 30);
    assert!(narrow.contains("Signatures"), "{narrow}");
    assert!(narrow.contains("Selected record"), "{narrow}");
    let tiny = rendered_text(&app, 50, 16);
    assert!(tiny.contains("needs at least 80x24"), "{tiny}");
}

#[test]
fn signature_workspace_renders_explicit_loading_empty_failure_and_picker_states() {
    let target = yoctui_model::SignatureTarget {
        recipe: "busybox".into(),
        task: "do_fetch".into(),
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Signatures;
    app.signature_dump = SignatureDumpState::Loading {
        target: target.clone(),
    };
    assert!(rendered_text(&app, 100, 24).contains("Loading authoritative signature artifacts"));
    app.signature_dump = SignatureDumpState::AvailableEmpty {
        target: target.clone(),
    };
    assert!(rendered_text(&app, 100, 24).contains("no signature artifacts"));
    app.signature_dump = SignatureDumpState::Failed {
        target,
        message: "tool missing".into(),
    };
    assert!(rendered_text(&app, 100, 24).contains("tool missing"));

    app.screen = Screen::Recipes;
    app.dialogs.push_back(Dialog::SignatureTaskPicker(
        yoctui_model::SignatureTaskPicker {
            recipe: RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/busybox.bb".into(),
            },
            tasks: vec!["do_fetch".into(), "do_compile".into()],
            selection: 1,
        },
    ));
    let picker = rendered_text(&app, 80, 24);
    assert!(picker.contains("Inspect signatures: busybox"), "{picker}");
    assert!(picker.contains("Authoritative signature tasks"), "{picker}");
}

#[test]
fn pkgdata_workspace_renders_typed_partial_details_footer_and_responsive_modes() {
    let request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let identity = PackageIdentity::new("busybox");
    let package = yoctui_model::PackageSummary {
        identity: identity.clone(),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Available("/layers/meta/recipes-core/busybox.bb".into()),
        version: PackageField::Available("1.37.0-r0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Unavailable,
    };
    let detail_request = yoctui_model::PackageDetailRequest {
        identity: identity.clone(),
        generation: 2,
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.focus = FocusTarget::Workspace;
    app.package_selection = Some(identity.clone());
    app.package_inventory = PackageInventoryState::Partial {
        request,
        packages: vec![package],
        limitations: vec!["image membership unavailable".into()],
    };
    app.package_details.insert(
        identity.clone(),
        PackageDetailState::Partial {
            request: detail_request,
            detail: yoctui_model::PackageDetail {
                identity,
                files: PackageField::Available(vec!["/bin/busybox".into()]),
                runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
                reverse_dependencies: PackageField::Available(Vec::new()),
            },
            limitations: vec!["reverse scan bounded".into()],
        },
    );

    let wide = rendered_text(&app, 160, 34);
    assert!(wide.contains("Packages"), "{wide}");
    assert!(wide.contains("busybox"), "{wide}");
    assert!(wide.contains("1.37.0-r0"), "{wide}");
    assert!(wide.contains("GPL-2.0-only"), "{wide}");
    assert!(wide.contains("/bin/busybox"), "{wide}");
    assert!(wide.contains("libc6"), "{wide}");
    assert!(wide.contains("Image membership: unavailable"), "{wide}");
    assert!(wide.contains("image membership unavailable"), "{wide}");
    assert!(wide.contains("F12 Menu"), "{wide}");
    let contextual_footer = rendered_text(&app, 120, 30);
    assert!(
        contextual_footer.contains("Enter detail"),
        "{contextual_footer}"
    );

    for (width, height) in [(120, 30), (90, 28)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Packages"), "{width}: {output}");
        assert!(output.contains("busybox"), "{width}: {output}");
    }
    for (theme, color) in [(Theme::WhiteClassic, true), (Theme::DarkPro, false)] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, 140, 30);
        assert!(output.contains("busybox"), "{output}");
    }
    assert!(rendered_text(&app, 50, 16).contains("needs at least 80x24"));
}

#[test]
fn ux_scrollable_collection_matrix_keeps_the_last_highlighted_row_visible() {
    let package_request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let packages = (0..40)
        .map(|index| yoctui_model::PackageSummary {
            identity: PackageIdentity::new(format!("package-{index:02}")),
            recipe: PackageField::Available(format!("recipe-{index:02}")),
            provider: PackageField::Unavailable,
            version: PackageField::Available("1.0".into()),
            installed_size_bytes: PackageField::Available(index),
            license: PackageField::Available("MIT".into()),
            image_membership: PackageField::Unavailable,
        })
        .collect::<Vec<_>>();
    let mut packages_app = App::new(10, 1_000);
    packages_app.package_selection = Some(PackageIdentity::new("package-39"));
    packages_app.package_inventory = PackageInventoryState::Available {
        request: package_request,
        packages,
    };
    let package_rows = rendered_region_rows(90, 12, |frame, area| {
        packages_workspace(frame, &packages_app, area)
    });
    assert!(
        package_rows.iter().any(|row| row.contains("package-39")),
        "{}",
        package_rows.join("\n")
    );
    assert!(!package_rows.iter().any(|row| row.contains("package-00")));

    let artifacts = (0..40)
        .map(|index| {
            let identity = yoctui_model::ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: format!("scroll-image-{index:02}"),
                path: format!("/deploy/scroll-image-{index:02}.ext4").into(),
            };
            yoctui_model::ImageArtifact {
                identity,
                kind: yoctui_model::ImageArtifactKind::RootFilesystem,
                size_bytes: ImageArtifactField::Available(4_096),
                modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
                checksums: ImageArtifactField::Available(Vec::new()),
                manifests: ImageArtifactField::Available(Vec::new()),
                licenses: ImageArtifactField::Available(Vec::new()),
                spdx: ImageArtifactField::Available(Vec::new()),
                wic_files: ImageArtifactField::Available(Vec::new()),
            }
        })
        .collect::<Vec<_>>();
    let mut images_app = App::new(10, 1_000);
    images_app.image_artifact_selection = Some(artifacts.last().unwrap().identity.clone());
    images_app.image_artifacts = ImageArtifactInventoryState::Available {
        request: yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy".into()),
            artifacts,
        },
    };
    let image_rows = rendered_region_rows(100, 14, |frame, area| {
        image_artifacts_workspace(frame, &images_app, area)
    });
    assert!(
        image_rows.iter().any(|row| row.contains("scroll-image-39")),
        "{}",
        image_rows.join("\n")
    );
    assert!(!image_rows.iter().any(|row| row.contains("scroll-image-00")));

    let mut config_app = App::new(10, 1_000);
    for index in 0..40 {
        config_app.workspace.variables.insert(
            format!("SCROLL_VARIABLE_{index:02}"),
            format!("value-{index:02}"),
        );
    }
    config_app.config_selection = 39;
    let config_rows = rendered_region_rows(100, 20, |frame, area| config(frame, &config_app, area));
    assert!(
        config_rows
            .iter()
            .take(8)
            .any(|row| row.contains("SCROLL_VARIABLE_39")),
        "{}",
        config_rows.join("\n")
    );

    let signature_rows_data = (0..40)
        .map(|index| yoctui_model::SignatureRecord {
            identity: yoctui_model::SignatureIdentity {
                target: yoctui_model::SignatureTarget {
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                },
                hash: Some(format!("scroll-signature-{index:02}")),
                path: Some(format!("/build/scroll-signature-{index:02}").into()),
            },
            base_hash: None,
            task_hash: None,
            variables: Vec::new(),
            dependencies: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut signatures_app = App::new(10, 1_000);
    signatures_app.signature_selection = Some(signature_rows_data.last().unwrap().identity.clone());
    signatures_app.signature_dump = SignatureDumpState::Available {
        target: signature_rows_data[0].identity.target.clone(),
        records: signature_rows_data,
    };
    let signature_rows = rendered_region_rows(90, 12, |frame, area| {
        signature_records(frame, &signatures_app, area)
    });
    assert!(
        signature_rows
            .iter()
            .any(|row| row.contains("scroll-signature-39")),
        "{}",
        signature_rows.join("\n")
    );

    let mut picker_app = App::new(10, 1_000);
    picker_app
        .dialogs
        .push_back(Dialog::RecipeTaskPicker(yoctui_model::RecipeTaskPicker {
            recipe: "busybox".into(),
            tasks: (0..40)
                .map(|index| format!("do_scroll_{index:02}"))
                .collect(),
            selection: 39,
            force: false,
        }));
    let picker = rendered_text(&picker_app, 100, 30);
    assert!(picker.contains("do_scroll_39"), "{picker}");
    assert!(!picker.contains("do_scroll_00"), "{picker}");

    let mut compatibility_app = compatibility_ui_inspector_app();
    let mut authority = compatibility_app
        .workspace_compatibility
        .authority()
        .expect("compatibility fixture authority")
        .clone();
    authority.snapshot.generation += 1;
    authority.snapshot.capabilities = yoctui_model::CapabilityId::ALL
        .into_iter()
        .map(|id| yoctui_model::CapabilityRecord {
            id,
            state: yoctui_model::CapabilityState::Unknown {
                reason: yoctui_model::CapabilityReason::new(
                    "test.inconclusive",
                    "Bounded viewport test evidence is intentionally inconclusive.",
                    None,
                )
                .unwrap(),
            },
            evidence: Vec::new(),
        })
        .collect();
    authority.implementations.clear();
    let authority = authority.normalize().unwrap();
    yoctui_model::install_workspace_compatibility(&mut compatibility_app, authority).unwrap();
    let authority = compatibility_app
        .workspace_compatibility
        .authority()
        .cloned();
    compatibility_app
        .compatibility_ui
        .select(isize::MAX, authority.as_ref());
    let last_capability = yoctui_model::CapabilityId::ALL
        .last()
        .expect("capability inventory")
        .as_str();
    let first_capability = yoctui_model::CapabilityId::ALL
        .first()
        .expect("capability inventory")
        .as_str();
    let compatibility_rows = rendered_region_rows(100, 24, |frame, area| {
        compatibility_workspace(frame, &compatibility_app, area)
    });
    assert!(
        compatibility_rows
            .iter()
            .any(|row| row.contains(last_capability)),
        "{}",
        compatibility_rows.join("\n")
    );
    assert!(
        !compatibility_rows
            .iter()
            .any(|row| row.contains(first_capability)),
        "{}",
        compatibility_rows.join("\n")
    );
    assert!(
        compatibility_rows.join("\n").contains(&format!(
            "{0}/{0} · ↑ · rows",
            yoctui_model::CapabilityId::ALL.len()
        )),
        "{}",
        compatibility_rows.join("\n")
    );
}

#[test]
fn pkgdata_workspace_renders_loading_empty_failed_and_unavailable_states() {
    let request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.package_inventory = PackageInventoryState::Loading { request };
    assert!(rendered_text(&app, 100, 25).contains("Loading authoritative package inventory"));
    app.package_inventory = PackageInventoryState::AvailableEmpty { request };
    assert!(rendered_text(&app, 100, 25).contains("No built runtime packages"));
    app.package_inventory = PackageInventoryState::Failed {
        request,
        message: "generated pkgdata is unavailable".into(),
    };
    let failed = rendered_text(&app, 100, 25);
    assert!(
        failed.contains("generated pkgdata is unavailable"),
        "{failed}"
    );
    assert!(failed.contains("do_package"), "{failed}");
}

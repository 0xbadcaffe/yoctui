#[test]
fn ux_accessibility_invariants_survive_color_motion_and_responsive_modes() {
    let mut app = literal_reference_app();
    app.focus = FocusTarget::Workspace;
    app.reduced_motion = true;
    app.color_enabled = false;
    app.theme = Theme::HighContrast;

    for (width, height) in [(160, 48), (100, 30), (80, 24)] {
        let output = rendered_text_at(&app, width, height, literal_now());
        for expected in ["Tasks", "▶ Running", "72%"] {
            assert!(
                output.contains(expected),
                "{width}x{height} missing {expected}: {output}"
            );
        }
    }

    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert!(
        buffer
            .content
            .iter()
            .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset),
        "--no-color must leave no semantic RGB/ANSI color in the terminal buffer"
    );
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::REVERSED)),
        "the selected task row must remain visible through reverse video"
    );
    assert!(
        buffer.content.iter().any(|cell| {
            matches!(cell.symbol(), "─" | "│" | "┌" | "┐" | "└" | "┘")
                && cell.modifier.contains(Modifier::BOLD)
        }),
        "the focused pane border must remain visible without color"
    );

    app.animation_frame = 0;
    let stable = rendered_text_at(&app, 160, 48, literal_now());
    app.animation_frame = u64::MAX;
    assert_eq!(
        rendered_text_at(&app, 160, 48, literal_now()),
        stable,
        "reduced motion must make presentation independent of animation frame"
    );

    app.dialogs.push_back(Dialog::BuildOptions);
    let dialog = rendered_text_at(&app, 80, 24, literal_now());
    for expected in [
        "Image build options",
        "Machine:",
        "b  Build image",
        "Esc closes",
    ] {
        assert!(
            dialog.contains(expected),
            "dialog missing {expected}: {dialog}"
        );
    }

    app.dialogs.clear();
    app.screen = Screen::BuildHistory;
    let history = rendered_text_at(&app, 160, 48, literal_now());
    for expected in ["Job History", "✓ Succeeded", "✕ Failed"] {
        assert!(
            history.contains(expected),
            "history missing {expected}: {history}"
        );
    }

    app.screen = Screen::Logs;
    let _ = update(
        &mut app,
        Action::Log(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Warning,
            message: "warning remains textual".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: literal_now(),
            build: None,
            protected: false,
            diagnostic: None,
        }),
    );
    let _ = update(
        &mut app,
        Action::Log(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "error remains textual".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: literal_now(),
            build: None,
            protected: true,
            diagnostic: None,
        }),
    );
    let logs = rendered_text_at(&app, 160, 48, literal_now());
    for expected in ["Logs", "! Warning", "✕ Error", "▶ Follow"] {
        assert!(logs.contains(expected), "logs missing {expected}: {logs}");
    }

    let palette = ThemePalette::for_app(&App {
        theme: Theme::HighContrast,
        color_enabled: true,
        ..App::new(16, 4096)
    });
    assert_ne!(palette.focused_border, palette.inactive_border);
    assert_ne!(palette.success, palette.error);
    assert_ne!(palette.selection_background, palette.background);
}

#[test]
fn ux_accessibility_m21_surfaces_never_require_color_glyph_shape_or_motion() {
    let modes = [
        (false, Theme::HighContrast, SymbolPreference::Ascii, true),
        (true, Theme::HighContrast, SymbolPreference::Unicode, true),
        (true, Theme::Monochrome, SymbolPreference::Ascii, false),
    ];
    for (color, theme, symbols, reduced_motion) in modes {
        let configure = |app: &mut App| {
            app.color_enabled = color;
            app.theme = theme;
            app.preferences.symbols = symbols;
            app.preferences.charts = if symbols == SymbolPreference::Ascii {
                yoctui_model::ChartPreference::AccessibleText
            } else {
                yoctui_model::ChartPreference::Automatic
            };
            app.reduced_motion = reduced_motion;
        };

        let mut tasks = literal_reference_app();
        tasks.screen = Screen::Tasks;
        tasks.focus = FocusTarget::Workspace;
        configure(&mut tasks);
        let task_text = rendered_text_at(&tasks, 100, 30, literal_now());
        for expected in ["Tasks", "Running", "72%", "do_compile"] {
            assert!(
                task_text.contains(expected),
                "missing {expected}: {task_text}"
            );
        }

        let mut rootfs = ux_rootfs_ui_app();
        rootfs.images_view = ImagesView::RootfsPackages;
        configure(&mut rootfs);
        let rootfs_text = rendered_text_at(&rootfs, 200, 60, literal_now());
        for expected in ["Installed packages:", "Exact bytes", "Other"] {
            assert!(
                rootfs_text.contains(expected),
                "missing {expected}: {rootfs_text}"
            );
        }
        if symbols == SymbolPreference::Ascii {
            assert!(!rootfs_text.contains("visual summary"), "{rootfs_text}");
        }

        let mut menu = App::new(10, 1_000);
        configure(&mut menu);
        let _ = update(&mut menu, Action::OpenApplicationMenu);
        let menu_text = rendered_text_at(&menu, 80, 24, literal_now());
        for expected in [
            "Application menu",
            "Load a Yocto workspace first",
            "Esc/F12 close",
        ] {
            assert!(
                menu_text.contains(expected),
                "missing {expected}: {menu_text}"
            );
        }

        let mut terminal = concept_terminal_sessions_app();
        configure(&mut terminal);
        let terminal_text = rendered_text_at(&terminal, 160, 50, literal_now());
        for expected in ["BuildShell", "writer", "Devshell", "read-only", "viewer(s)"] {
            assert!(
                terminal_text.contains(expected),
                "missing terminal ownership {expected}: {terminal_text}"
            );
        }

        for output in [task_text, rootfs_text, menu_text, terminal_text] {
            assert!(!output.contains('\u{fffd}'), "{output}");
        }
    }

    let mut checkbox = yoctui_model::CheckboxState::new("pkg", "busybox");
    checkbox.focused = true;
    checkbox.value = yoctui_model::CheckboxValue::Indeterminate;
    assert_eq!(
        checkbox_text(&checkbox, false),
        "> [-] busybox (indeterminate)"
    );
    checkbox.set_disabled("required by the selected image");
    let disabled = checkbox_text(&checkbox, false);
    assert!(disabled.contains("disabled"), "{disabled}");
    assert!(
        disabled.contains("required by the selected image"),
        "{disabled}"
    );
}

#[test]
fn animation_unknown_progress_never_fabricates_a_percentage() {
    for (theme, color_enabled) in [
        (Theme::DarkPro, true),
        (Theme::WhiteClassic, true),
        (Theme::MatrixGreen, true),
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        let mut app = App::new(10, 1_000);
        app.theme = theme;
        app.color_enabled = color_enabled;
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: None,
                ..yoctui_model::TaskInfo::default()
            },
        );
        let output = rendered_text(&app, 300, 30);
        assert!(output.contains("progress unknown"), "{output}");
        assert!(!output.contains("busybox:do_compile 0%"));
        let _ = rendered_text(&app, 80, 24);
    }
}

#[test]
fn images_workspace_renders_typed_artifacts_inspector_and_responsive_modes() {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.build.target = Some("core-image-minimal".into());
    let request = yoctui_model::ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    let path = std::path::PathBuf::from("/deploy/qemux86-64/core-image-minimal-qemux86-64.wic");
    let artifact = yoctui_model::ImageArtifact {
        identity: yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: path.clone(),
        },
        kind: yoctui_model::ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(8192),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Available(vec![yoctui_model::ImageChecksum {
            algorithm: "sha256".into(),
            digest: "abcdef".into(),
            source: "/deploy/qemux86-64/image.sha256".into(),
        }]),
        manifests: ImageArtifactField::Available(vec!["/deploy/qemux86-64/image.manifest".into()]),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Available(vec!["/deploy/qemux86-64/image.spdx.json".into()]),
        wic_files: ImageArtifactField::Available(vec![path]),
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Partial {
        request,
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
            artifacts: vec![artifact],
        },
        limitations: vec!["one symlink was not followed".into()],
    };

    for (width, theme, color) in [
        (180, Theme::DarkPro, true),
        (120, Theme::WhiteClassic, true),
        (80, Theme::Monochrome, false),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, 32);
        assert!(output.contains("core-image-minimal"), "{output}");
        assert!(output.contains("wic"), "{output}");
        if width >= WIDE_WORKBENCH_MIN_WIDTH {
            assert!(output.contains("F12 Menu"), "{output}");
        } else {
            assert!(output.contains("refresh"), "{output}");
        }
    }
    let wide = rendered_text(&app, 180, 40);
    assert!(wide.contains("Deploy directory"), "{wide}");
    assert!(wide.contains("sha256"), "{wide}");
    assert!(wide.contains("SPDX/SBOM"), "{wide}");
    assert!(wide.contains("one symlink was not followed"), "{wide}");
}

#[test]
fn ux_image_preview_renders_deterministic_metadata_fallback_without_protocol_claims() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Images;
    app.focus = FocusTarget::Inspector;
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/qemux86-64/core-image-minimal.ext4".into(),
    };
    let artifact = yoctui_model::ImageArtifact {
        identity: identity.clone(),
        kind: yoctui_model::ImageArtifactKind::RootFilesystem,
        size_bytes: ImageArtifactField::Available(4096),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Available(Vec::new()),
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Available(Vec::new()),
        spdx: ImageArtifactField::Available(Vec::new()),
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    app.image_artifact_selection = Some(identity);
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

    for (width, height, theme, color) in [
        (180, 50, Theme::DarkPro, true),
        (100, 30, Theme::HighContrast, true),
        (80, 24, Theme::Monochrome, false),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Terminal image preview"), "{output}");
        assert!(output.contains("protocol probe skipped"), "{output}");
        assert!(output.contains("Native graphics: not offered"), "{output}");
        assert!(
            output.contains("Rootfs packages/filesystem composition"),
            "{output}"
        );
        assert!(!output.contains("Kitty graphics"), "{output}");
        assert!(!output.contains("Sixel graphics"), "{output}");
    }
}

#[test]
fn images_workspace_renders_loading_empty_failure_and_search_empty_states() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Images;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let request = yoctui_model::ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    app.image_artifacts = ImageArtifactInventoryState::Loading {
        request: request.clone(),
    };
    assert!(rendered_text(&app, 100, 25).contains("Loading deployed image artifacts"));
    app.image_artifacts = ImageArtifactInventoryState::AvailableEmpty {
        request: request.clone(),
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
            artifacts: Vec::new(),
        },
    };
    assert!(rendered_text(&app, 100, 25).contains("No deployed image artifacts"));
    app.image_artifacts = ImageArtifactInventoryState::Failed {
        request,
        message: "DEPLOY_DIR_IMAGE is unavailable".into(),
    };
    assert!(rendered_text(&app, 100, 25).contains("Artifact scan failed"));
}

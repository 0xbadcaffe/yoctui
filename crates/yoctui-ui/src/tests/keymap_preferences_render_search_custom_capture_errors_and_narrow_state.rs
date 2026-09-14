//! Regression tests grouped around ux_keymap_preferences_render_search_custom_capture_errors_and_narrow_state.
use super::*;

#[test]
fn ux_keymap_preferences_render_search_custom_capture_errors_and_narrow_state() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Settings;
    app.install_keymap(yoctui_model::KeymapPreferences {
        schema_version: yoctui_model::KEYMAP_SCHEMA_VERSION,
        overrides: vec![yoctui_model::KeymapOverride {
            action_id: "navigate.logs".into(),
            scope: yoctui_model::KeymapScope::Global,
            sequences: vec!["z".parse().unwrap()],
        }],
    })
    .unwrap();
    app.keymap_preferences_ui.open = true;
    app.keymap_preferences_ui.query = "navigate.logs".into();
    app.focus = FocusTarget::Dialog;

    let output = rendered_text(&app, 120, 35);
    assert!(output.contains("Keybinding preferences"), "{output}");
    assert!(output.contains("Effective keymap"), "{output}");
    assert!(output.contains("Open Logs"), "{output}");
    assert!(output.contains("Global"), "{output}");
    assert!(output.contains("custom"), "{output}");
    assert!(output.contains("navigate.logs"), "{output}");
    assert!(output.contains("z"), "{output}");
    assert!(output.contains("capture"), "{output}");
    assert!(output.contains("export"), "{output}");

    app.keymap_preferences_ui.capture = Some(yoctui_model::KeymapCaptureState {
        strokes: vec![
            yoctui_model::KeyStroke::Char('g'),
            yoctui_model::KeyStroke::Char('l'),
        ],
    });
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Pending capture: g l"), "{output}");
    assert!(output.contains("Ctrl+S"), "{output}");

    app.keymap_preferences_ui.capture = None;
    app.keymap_preferences_ui.validation_error =
        Some("keymap collision in Global: exact test reason".into());
    app.color_enabled = false;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Conflict/disabled"), "{output}");
    assert!(output.contains("exact test reason"), "{output}");
}

#[test]
fn ux_onboarding_renders_real_typed_workflow_states_responsively_and_accessibly() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::OpenOnboarding);
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Yoctui workflow guide"),
            "{width}x{height}: {output}"
        );
        assert!(
            output.contains("opening/resuming starts no build or process"),
            "{output}"
        );
        assert!(output.contains("Verify environment"), "{output}");
        assert!(output.contains("CURRENT"), "{output}");
        assert!(output.contains("BLOCKED"), "{output}");
        assert!(output.contains("Enter open"), "{output}");
        assert!(output.contains("Esc dismiss"), "{output}");
    }

    app.onboarding
        .progress
        .completed
        .insert(yoctui_model::OnboardingStep::Environment);
    app.color_enabled = false;
    app.reduced_motion = true;
    let stale = rendered_text(&app, 100, 30);
    assert!(stale.contains("[!] Verify environment"), "{stale}");
    assert!(stale.contains("STALE"), "{stale}");

    app.onboarding.progress.completed.clear();
    app.onboarding
        .progress
        .skipped
        .insert(yoctui_model::OnboardingStep::Environment);
    app.onboarding.progress.current = yoctui_model::OnboardingStep::Target;
    app.onboarding.selected = yoctui_model::OnboardingStep::Target;
    let unavailable = rendered_text(&app, 80, 24);
    assert!(unavailable.contains("SKIPPED"), "{unavailable}");
    assert!(unavailable.contains("UNAVAILABLE"), "{unavailable}");
    assert!(unavailable.contains("[?] Select a target"), "{unavailable}");
}

#[test]
fn ux_menu_renders_groups_context_disabled_safety_and_accessible_responsive_states() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenApplicationMenu);
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Application menu"),
            "{width}x{height}: {output}"
        );
        assert!(output.contains("[Workspace]"), "{output}");
        assert!(output.contains("Build"), "{output}");
        assert!(output.contains("Navigate"), "{output}");
        assert!(output.contains("View"), "{output}");
        assert!(output.contains("Tools"), "{output}");
        assert!(output.contains("Help"), "{output}");
        assert!(output.contains("Edit BBMASK"), "{output}");
        assert!(output.contains("Load a Yocto workspace first"), "{output}");
        assert!(output.contains("Esc/F10 close"), "{output}");
    }

    let _ = update(&mut app, Action::CloseMenu);
    app.screen = Screen::Recipes;
    app.workspace.build_dir = Some("/work/build".into());
    app.reduced_motion = true;
    app.color_enabled = false;
    let _ = update(&mut app, Action::OpenContextMenu);
    let _ = update(&mut app, Action::SelectMenuItem { delta: 2 });
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Recipes actions"), "{output}");
    assert!(output.contains("Build selected recipe"), "{output}");
    assert!(output.contains("confirm"), "{output}");
    assert!(output.contains("Select a recipe first."), "{output}");
    assert!(output.contains("> Build selected reci"), "{output}");

    let last_label = app
        .active_menu_items()
        .last()
        .expect("recipe context menu item")
        .label
        .to_owned();
    let _ = update(&mut app, Action::SelectMenuItem { delta: isize::MAX });
    let bottom = rendered_text(&app, 80, 24);
    let selected_prefix = format!("> {}", last_label.chars().take(12).collect::<String>());
    assert!(
        bottom.contains(&selected_prefix),
        "last menu row {last_label:?} lost its highlight: {bottom}"
    );
}

#[test]
fn ux_viewport_chrome_reports_position_and_available_directions() {
    let mut navigator_app = App::new(10, 1_000);
    navigator_app.focus = FocusTarget::Navigator;
    let total_navigation_rows = navigator_app.navigator_visible_row_count();
    let first_navigation_row = navigator_app.navigator_visual_row() + 1;
    let top = rendered_text(&navigator_app, 80, 24);
    assert!(
        top.contains(&format!(
            "Navigator · {first_navigation_row}/{total_navigation_rows} ↓"
        )),
        "{top}"
    );

    navigator_app.navigator_selection = 24;
    let bottom = rendered_text(&navigator_app, 80, 24);
    assert!(
        bottom.contains(&format!(
            "Navigator · {total_navigation_rows}/{total_navigation_rows} ↑"
        )),
        "{bottom}"
    );
    navigator_app.preferences.symbols = SymbolPreference::Ascii;
    let ascii = rendered_text(&navigator_app, 80, 24);
    assert!(
        ascii.contains(&format!(
            "Navigator · {total_navigation_rows}/{total_navigation_rows} ^"
        )),
        "{ascii}"
    );

    let mut palette_app = App::new(10, 1_000);
    palette_app.command_palette_open = true;
    palette_app.focus = FocusTarget::CommandPalette;
    let command_count = palette_app.command_palette_commands().len();
    palette_app.command_palette_selection = command_count.saturating_sub(1);
    let palette = rendered_text(&palette_app, 80, 24);
    assert!(
        palette.contains(&format!(
            "Commands · {command_count}/{command_count} · ↑ · rows"
        )),
        "{palette}"
    );

    palette_app.command_palette_query = "Open Settings".into();
    palette_app.command_palette_selection = 0;
    let fitting = rendered_text(&palette_app, 80, 24);
    assert!(fitting.contains("Commands · 1 match"), "{fitting}");
    assert!(!fitting.contains("Commands · 1/1 · rows"), "{fitting}");

    let mut menu_app = App::new(10, 1_000);
    menu_app.screen = Screen::Recipes;
    menu_app.workspace.build_dir = Some("/work/build".into());
    let _ = update(&mut menu_app, Action::OpenContextMenu);
    let menu_count = menu_app.active_menu_items().len();
    let _ = update(&mut menu_app, Action::SelectMenuItem { delta: isize::MAX });
    let menu = rendered_text(&menu_app, 80, 24);
    assert!(
        menu.contains(&format!(
            "Catalog actions · {menu_count}/{menu_count} · ↑ · rows"
        )),
        "{menu}"
    );
}

#[test]
fn build_environment_workspace_renders_disconnected_state_and_unlock_rule() {
    let app = App::new_unconfigured(10, 1_000);
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Build environment"));
    assert!(output.contains("not configured"));
    assert!(output.contains("available images"));
    assert!(output.contains("verification"));
}

#[test]
fn project_profile_renders_team_intent_and_explicit_resolution_states() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::BuildEnvironment;
    app.project_profile = yoctui_model::ProjectProfileState::Loaded(yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites {
            recipes: vec!["busybox".into(), "removed-recipe".into()],
            images: Vec::new(),
            layers: Vec::new(),
        },
        build_presets: Vec::new(),
        workflows: Vec::new(),
    });
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        ..yoctui_model::Recipe::default()
    });
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Project profile: team intent"), "{output}");
        assert!(output.contains("Recipe favorite: busybox"), "{output}");
        if height >= 30 {
            assert!(
                output.contains("STALE: not reported by BitBake"),
                "{output}"
            );
        }
        assert!(output.contains("p preview/open"), "{output}");
    }

    app.project_profile =
        yoctui_model::ProjectProfileState::Invalid("unsupported schema version 9".into());
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Project profile: invalid"), "{output}");
    assert!(output.contains("unsupported schema version 9"), "{output}");
}

#[test]
fn build_environment_form_renders_selected_typed_fields() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
    let output = rendered_text(&app, 120, 36);
    assert!(output.contains("Edit profile"));
    assert!(output.contains("source:"));
    assert!(output.contains("script:"));
}

#[test]
fn build_environment_editor_renders_large_vi_style_popup() {
    let mut app = App::new_unconfigured(10, 1_000);
    let mut editor =
        yoctui_model::PopupEditor::new("source = \"/home/poky\"\nbuild = \"/home/build\"".into());
    editor.select_toml_value("source").unwrap();
    app.dialogs
        .push_back(Dialog::BuildEnvironmentEditor(editor));
    for (width, height) in [(80, 24), (120, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Build environment.toml"));
        assert!(output.contains("NORMAL"));
        assert!(output.contains("⟦/home/poky⟧▏"));
        assert!(output.contains("Home/End line"));
        assert!(output.contains("Ctrl+V paste"));
    }
}

#[test]
fn build_environment_clone_editor_uses_shared_popup_at_minimum_size() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::OpenBuildEnvironmentCloneEditor);
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Clone Poky.toml"), "{output}");
    assert!(output.contains("repository = \"⟦▏⟧\""), "{output}");
    assert!(output.contains("e change value"), "{output}");
    assert!(output.contains("Ctrl+C copy"), "{output}");
}

#[test]
fn active_task_indicator_uses_braille_motion_and_accessible_fallbacks() {
    let mut app = App::new(10, 1_000);
    app.animation_frame = 0;
    let first = task_activity(&app, None);
    app.animation_frame = 1;
    assert_ne!(task_activity(&app, None), first);
    assert!(
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE
            .symbols
            .contains(&first.as_str())
    );
    assert_eq!(task_activity(&app, Some(0)), "");

    app.reduced_motion = true;
    assert_eq!(task_activity(&app, None), "⣿");
    app.preferences.symbols = SymbolPreference::Ascii;
    assert_eq!(task_activity(&app, None), "#");
}

#[test]
fn ux_throbber_uses_reviewed_symbols_with_ascii_motion_and_terminal_fallbacks() {
    let running = yoctui_model::ActivityProjection::new(
        yoctui_model::ActivityLifecycle::Running,
        5,
        yoctui_model::AnimationSpeed::Fast,
        false,
    );
    assert_eq!(
        activity_symbol(running, true),
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE.symbols
            [running.phase.expect("running activity phase") % 8]
    );
    assert_eq!(
        activity_symbol(running, false),
        throbber_widgets_tui::ASCII.symbols[1]
    );

    let reduced = yoctui_model::ActivityProjection::new(
        yoctui_model::ActivityLifecycle::Waiting,
        u64::MAX,
        yoctui_model::AnimationSpeed::Fast,
        true,
    );
    assert_eq!(activity_symbol(reduced, true), "");
    assert_eq!(reduced.text(), "waiting");

    for (lifecycle, marker) in [
        (yoctui_model::ActivityLifecycle::Succeeded, "+"),
        (yoctui_model::ActivityLifecycle::Failed, "x"),
        (yoctui_model::ActivityLifecycle::Cancelled, "#"),
    ] {
        let terminal = yoctui_model::ActivityProjection::new(
            lifecycle,
            u64::MAX,
            yoctui_model::AnimationSpeed::Fast,
            false,
        );
        assert_eq!(terminal.phase, None);
        assert_eq!(activity_symbol(terminal, false), marker);
    }

    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    app.reduced_motion = true;
    app.build.status = BuildStatus::Running;
    app.build.completed = 3;
    app.screen = Screen::Tasks;
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("progress unknown ⣿  3/—"), "{output}");
    assert!(!output.contains("0%"), "{output}");
}

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
        "modal · Image build options",
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
            "Esc/F10 close",
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
            assert!(output.contains("F10 Menu"), "{output}");
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

#[test]
fn udev_last_rule_and_preview_remain_visible_at_all_sizes() {
    let mut app = ux_rootfs_ui_app();
    let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition else {
        unreachable!()
    };
    composition.system_inventory =
        yoctui_model::RootfsAuthority::Available(yoctui_model::RootfsSystemInventory {
            udev_rules: (0..100)
                .map(|index| yoctui_model::RootfsUdevRule {
                    name: format!("{index:03}-test.rules"),
                    logical_path: yoctui_model::RootfsPathIdentity(
                        format!("/etc/udev/rules.d/{index:03}-test.rules").into(),
                    ),
                    masked: false,
                    overridden_by: None,
                    limitation: None,
                    preview: "# first\nSUBSYSTEM==\"tty\"\n".into(),
                    preview_truncated: false,
                })
                .collect(),
            ..Default::default()
        });
    app.images_view = ImagesView::UdevRules;
    yoctui_model::update(&mut app, Action::SelectRootfsUdevRule { delta: isize::MAX });
    assert_eq!(app.rootfs_udev_selection, 99);
    yoctui_model::update(&mut app, Action::ScrollRootfsUdevPreview { delta: 1 });
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("099-test.rules"), "{text}");
        assert!(text.contains("SUBSYSTEM"), "{text}");
        assert!(text.contains("6 udev"), "{text}");
    }
}

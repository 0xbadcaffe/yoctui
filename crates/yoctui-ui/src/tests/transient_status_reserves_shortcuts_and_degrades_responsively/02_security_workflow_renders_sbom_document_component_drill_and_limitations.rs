#[test]
fn security_workflow_renders_sbom_document_component_drill_and_limitations() {
    let mut app = security_workflow_ui_app();
    let spdx_identity = app
        .security
        .inventory
        .reports()
        .unwrap()
        .iter()
        .find_map(|report| match report {
            SecurityReport::Spdx(document) => Some(document.identity.clone()),
            _ => None,
        })
        .unwrap();
    app.security.view = SecurityView::Sbom;
    app.security.report_selection = Some(spdx_identity);
    for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("SBOM"), "{output}");
        assert!(output.contains("SPDX-2.3"), "{output}");
        assert!(output.contains("core-image-minimal"), "{output}");
    }

    app.security.drilled = true;
    app.security.component_selection = Some("SPDXRef-Package-busybox".into());
    let drilled = rendered_text(&app, 80, 24);
    assert!(drilled.contains("SPDXRef-Package-busybox"), "{drilled}");
    assert!(drilled.contains("GPL-2.0-only"), "{drilled}");

    app.focus = FocusTarget::Inspector;
    let inspector = rendered_text(&app, 160, 40);
    assert!(inspector.contains("https://example.invalid/spdx/image"));
    assert!(inspector.contains("SHA256=abcd1234"));
    assert!(inspector.contains("external references unavailable"));
    assert!(inspector.contains("Organization: Yocto"));
}

#[test]
fn security_workflow_renders_every_inventory_and_capability_state() {
    let mut app = security_workflow_ui_app();
    let request = app.security.inventory.request().unwrap().clone();
    for (state, expected) in [
        (SecurityInventoryState::NotLoaded, "Reports are not loaded"),
        (
            SecurityInventoryState::Loading {
                request: request.clone(),
            },
            "Loading report generation",
        ),
        (
            SecurityInventoryState::AvailableEmpty {
                request: request.clone(),
            },
            "available-empty",
        ),
        (
            SecurityInventoryState::Failed {
                request: request.clone(),
                message: "permission denied".into(),
            },
            "acquisition failed",
        ),
        (
            SecurityInventoryState::Cancelled {
                request: request.clone(),
            },
            "acquisition cancelled",
        ),
        (
            SecurityInventoryState::TimedOut {
                request: request.clone(),
            },
            "acquisition timed out",
        ),
        (
            SecurityInventoryState::Lost {
                request,
                message: "worker channel closed".into(),
            },
            "worker lost",
        ),
    ] {
        app.security.inventory = state;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }

    app.security.inventory = SecurityInventoryState::NotLoaded;
    for (capability, expected) in [
        (SecurityCapability::NotInspected, "not inspected"),
        (SecurityCapability::Inspecting, "inspection in progress"),
        (
            SecurityCapability::Failed("missing class".into()),
            "inspection failed",
        ),
    ] {
        app.security.capability = capability;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn security_workflow_renders_mapper_session_terminal_outcomes_and_bounded_output() {
    let mut app = security_workflow_ui_app();
    let request = app.security.inventory.request().unwrap().clone();
    app.security.inventory = SecurityInventoryState::AvailableEmpty { request };
    for status in [
        SecuritySessionStatus::Starting,
        SecuritySessionStatus::Running,
        SecuritySessionStatus::Cancelling,
        SecuritySessionStatus::Succeeded,
        SecuritySessionStatus::Failed,
        SecuritySessionStatus::Cancelled,
        SecuritySessionStatus::TimedOut,
        SecuritySessionStatus::Lost,
    ] {
        app.security.sessions = vec![security_session(status)];
        let output = rendered_text(&app, 100, 30);
        assert!(
            output.contains(security_session_status_label(status)),
            "{status:?}: {output}"
        );
        assert!(output.contains("mapped busybox"), "{output}");
        assert!(output.contains("[truncated]"), "{output}");
    }
}

#[test]
fn security_workflow_dialogs_render_exact_previews_at_all_breakpoints() {
    let mut app = security_workflow_ui_app();
    app.focus = FocusTarget::Dialog;
    let session = security_session(SecuritySessionStatus::Starting);
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Operation(session.preview)));
    for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Confirm CVE package mapping"), "{output}");
        assert!(output.contains("Exact indexed shell-free"), "{output}");
        assert!(output.contains("1: /build/tmp/log/cve"), "{output}");
    }

    app.dialogs.clear();
    let mut editor = yoctui_model::PopupEditor::new(format!(
        "# exact canonical non-symlink path\nroot = \"/reports/{}\"\n",
        "long-path-".repeat(80)
    ));
    editor.select_toml_value("root").unwrap();
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Import {
            editor,
            validation_error: None,
        }));
    let import = rendered_text(&app, 80, 24);
    assert!(import.contains("Security import.toml"), "{import}");
    assert!(import.contains("Home/End line"), "{import}");
    assert!(import.contains("canonical non-symlink"), "{import}");

    app.dialogs.clear();
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Cancellation(
            yoctui_model::SecuritySessionId(9),
        )));
    let cancellation = rendered_text(&app, 80, 24);
    assert!(
        cancellation.contains("Confirm Security cancellation"),
        "{cancellation}"
    );
    assert!(cancellation.contains("session 9 only"), "{cancellation}");
}

#[test]
fn semantic_theme_exposes_complete_role_catalog_for_every_theme() {
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::VscodeDark,
        Theme::VscodeLight,
        Theme::AccessibleDark,
        Theme::SoftLight,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        let mut app = App::new(10, 1_000);
        app.theme = theme;
        let palette = ThemePalette::for_app(&app);
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.bg == palette.background)
        );
        if theme == Theme::Monochrome {
            assert!(palette.attribute_only);
            assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
            assert!(palette.selected().add_modifier.contains(Modifier::REVERSED));
        } else {
            assert!(!palette.attribute_only);
            assert_ne!(palette.focused_border, palette.inactive_border);
            assert_ne!(palette.selection_background, palette.background);
            assert_ne!(palette.error, palette.success);
            assert_ne!(palette.warning, palette.informational);
            assert_eq!(palette.running, palette.progress);
            assert_eq!(palette.pending, palette.warning);
            assert_eq!(palette.graph_cpu, palette.graph_disk_read);
            assert_eq!(palette.graph_memory, palette.graph_disk_write);
            assert_eq!(palette.graph_network_rx, palette.success);
            assert_eq!(palette.graph_network_tx, palette.warning);
        }
    }
}

#[test]
fn semantic_theme_no_color_resets_colors_and_preserves_attributes() {
    let mut app = App::new(10, 1_000);
    app.theme = Theme::WhiteClassic;
    app.color_enabled = false;
    let palette = ThemePalette::for_app(&app);

    assert!(palette.attribute_only);
    for color in [
        palette.background,
        palette.primary_foreground,
        palette.secondary_foreground,
        palette.focused_border,
        palette.inactive_border,
        palette.selection_foreground,
        palette.selection_background,
        palette.success,
        palette.warning,
        palette.error,
        palette.running,
        palette.pending,
        palette.accent,
        palette.muted,
        palette.progress,
        palette.graph_cpu,
        palette.graph_memory,
        palette.graph_disk_read,
        palette.graph_disk_write,
        palette.graph_network_rx,
        palette.graph_network_tx,
        palette.disabled,
        palette.informational,
        palette.heading,
        palette.table_header,
    ] {
        assert_eq!(color, Color::Reset);
    }
    assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
    assert!(
        selected_style(&app, true)
            .add_modifier
            .contains(Modifier::REVERSED)
    );
    assert!(
        severity_style(&app, Severity::Error)
            .add_modifier
            .contains(Modifier::UNDERLINED)
    );
    assert_ne!(
        severity_style(&app, Severity::Warning).add_modifier,
        severity_style(&app, Severity::Trace).add_modifier
    );

    app.focus = FocusTarget::Navigator;
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(terminal.backend().buffer().content.iter().any(|cell| {
        cell.modifier.contains(Modifier::REVERSED) || cell.modifier.contains(Modifier::BOLD)
    }));
}

#[test]
fn theme_light_shell_and_dialog_apply_the_semantic_background() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.theme = Theme::WhiteClassic;
    app.dialogs.push_back(Dialog::BuildOptions);
    terminal.draw(|frame| render(frame, &app)).unwrap();

    assert_eq!(
        ThemePalette::for_app(&app).background,
        Color::Rgb(248, 248, 248)
    );
    assert_eq!(
        ThemePalette::for_app(&app).informational,
        Color::Rgb(0, 107, 107)
    );
}

#[test]
fn theme_picker_renders_named_choices_and_immediate_apply_hint() {
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::ThemePicker {
        selection: 1,
        original_theme: Theme::DarkPro,
        original_color_enabled: true,
        original_settings_dirty: false,
    });
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Theme — applies immediately"));
    assert!(output.contains("White"));
    assert!(output.contains("Dark gray"));
    assert!(!output.to_ascii_lowercase().contains("vscode"));
    assert!(output.contains("Enter apply"));
}

#[test]
fn theme_progress_and_log_severity_use_semantic_roles() {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.theme = Theme::HighContrast;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(50),
            ..yoctui_model::TaskInfo::default()
        },
    );
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(50, 255, 100))
    );

    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: false,
        diagnostic: None,
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(255, 70, 70))
    );
}

#[test]
fn settings_workspace_renders_typed_rows_and_controls_on_narrow_terminals() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Settings;
    app.settings_selection = 5;
    app.settings_dirty = true;
    app.theme = Theme::MatrixGreen;
    app.animation_speed = yoctui_model::AnimationSpeed::Slow;
    app.reduced_motion = true;
    app.logs.follow = false;

    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Settings (not saved)"));
    assert!(output.contains("Theme"));
    assert!(output.contains("Green"));
    assert!(!output.to_ascii_lowercase().contains("vscode"));
    assert!(output.contains("Animation speed"));
    assert!(output.contains("Reduced motion"));
    assert!(output.contains("Log wrap"));
    assert!(output.contains("Log follow"));
    assert!(output.contains("Keybindings"));
    assert!(output.contains("select"));
    assert!(output.contains("change"));
}

#[test]
fn clone_progress_is_visible_across_terminal_widths() {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::SetBackgroundActivity {
            activity: yoctui_model::BackgroundActivity::Cloning,
            active: true,
        },
    );
    for width in [80, 100, 160] {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| workbench_footer(frame, &app, frame.area(), UNIX_EPOCH))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("Cloning…"), "{width}: {text}");
        assert!(
            text.chars().any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
            "{text}"
        );
    }
}
